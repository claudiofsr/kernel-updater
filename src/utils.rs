use crate::error::KernelUpdaterError;
use std::{
    process::{Command, Stdio},
    thread,
};

// --- Functions ---

/// Executes an external command, waiting for it to finish.
/// STDOUT is piped but not printed on success. STDERR is inherited.
/// Returns Ok(()) if the command succeeds (exit code 0), otherwise returns a KernelUpdaterError::CommandExecutionError.
/// Spawning errors are mapped to KernelUpdaterError::IoError via `#[from]`.
pub fn run_command(command: &str, args: &[&str]) -> Result<(), KernelUpdaterError> {
    let args_string = args.join(" ");
    // Log the command being executed using eprint! (diagnostic output).
    eprintln!("Executing: {} {}", command, args_string);

    // Spawn the command. STDOUT piped, STDERR inherited.
    let child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped()) // Capture stdout (we don't print it on success)
        .stderr(Stdio::inherit()) // Inherit stderr for live output
        .spawn()
        // Map std::io::Error (if spawn fails) to KernelUpdaterError::IoError using #[from]
        .map_err(KernelUpdaterError::IoError)?;

    // Wait for the command to finish. Get captured stdout and status.
    let output = child
        .wait_with_output() // Waits and captures stdout (stderr already handled by inherit)
        // Map std::io::Error (if wait fails) to KernelUpdaterError::IoError using #[from]
        .map_err(KernelUpdaterError::IoError)?;

    // Check the exit status.
    if output.status.success() {
        // Command succeeded. Do NOT print the captured stdout.
        // The stdout might be large binary data (like a downloaded file).
        Ok(())
    } else {
        // Command failed (non-zero exit code). The relevant error was likely shown live on stderr.
        // Return our specific command execution error.
        Err(KernelUpdaterError::CommandExecutionError {
            command: command.into(),
            args: args_string,
            status: output.status,
        })
    }
}

/// Executes an external command and captures its standard output as a String.
/// STDERR is inherited and printed directly to the console.
/// Returns the stdout String on success, or a KernelUpdaterError on failure.
/// Failure includes spawning errors (-> IoError), non-zero exit (-> CommandExecutionError), or non-UTF-8 output (-> Utf8OutputError).
/// Use this specifically for commands whose *output* on stdout you need to parse (like `dkms status`).
pub fn run_command_output(command: &str, args: &[&str]) -> Result<String, KernelUpdaterError> {
    let args_string = args.join(" ");
    eprintln!("Executing (capturing stdout): {} {}", command, args_string); // Log the command

    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped()) // Pipe stdout (to capture and return)
        .stderr(Stdio::inherit()) // Inherit stderr (to print live diagnostics)
        .output() // Execute, wait, and capture stdout/stderr (only stdout is truly captured programmatically here)
        // Map std::io::Error (if run/wait fails) to KernelUpdaterError::IoError using #[from]
        .map_err(KernelUpdaterError::IoError)?;

    if !output.status.success() {
        // Command failed. The relevant error on stderr is already printed due to inherit.
        Err(KernelUpdaterError::CommandExecutionError {
            command: command.into(),
            args: args_string,
            status: output.status,
        })
    } else {
        // Command succeeded, try to convert stdout to String.
        // Use map_err to convert FromUtf8Error to our custom Utf8OutputError.
        String::from_utf8(output.stdout).map_err(|source| {
            KernelUpdaterError::Utf8OutputError {
                command: command.into(),
                source, // The FromUtf8Error
            }
        })
    }
}

/// Calculates the number of CPU cores to use for parallel tasks,
/// leaving a specified number 'free'. Ensures at least 1 core is used.
// This remains in utils as it's a general helper, not specific to file paths or versions.
// Errors from available_parallelism (std::io::Error) map automatically via #[from].
pub fn get_cores(free: usize) -> Result<String, KernelUpdaterError> {
    // Get the total number of available logical cores.
    let num_cpus = thread::available_parallelism()? // Returns NonZeroUsize. Errors (std::io::Error) map to IoError.
        .get(); // Get the usize value

    // Calculate cores to use, ensuring it's at least 1.
    let cores_to_use = if num_cpus > free && num_cpus - free >= 1 {
        num_cpus - free
    } else {
        // If number of CPUs is less than or equal to 'free', or calculation results in 0, use at least 1 core.
        1 // Minimum 1 core for compilation
    };

    // Return the calculated number of cores as a String (as required by commands like 'make -j').
    Ok(cores_to_use.to_string())
}

/// The `update_grub` function is a post-installation step.
/// Errors from `run_command` map automatically via `?`.
pub fn update_grub() -> Result<(), KernelUpdaterError> {
    println!("Updating GRUB boot configuration...");
    // Use the run_command utility from the utils module.
    run_command("update-grub", &[])?; // Propagates KernelUpdaterError from run_command.
    println!("GRUB update completed successfully.");
    Ok(())
}

/// Exits the program with status code 0.
#[allow(dead_code)] // Allow if not used elsewhere
pub fn quit() {
    std::process::exit(0);
}
