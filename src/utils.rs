use crate::error::KernelUpdaterError;
use std::{
    process::{Command, Stdio},
    thread,
};

/// Runs a command, showing stderr on real-time, capturing stdout on success.
pub fn run_command(command: &str, args: &[&str]) -> Result<(), KernelUpdaterError> {
    let args_joined = args.join(" ");
    println!("Executing: {command} {args_joined}");

    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(KernelUpdaterError::CommandExecutionError {
            command: command.to_string(),
            args: args_joined,
            status,
        })
    }
}

/// Executes a command in a specific directory (optional).
#[allow(dead_code)]
pub fn run_command_in_dir(
    command: &str,
    args: &[&str],
    dir: Option<&std::path::Path>,
) -> Result<(), KernelUpdaterError> {
    let args_joined = args.join(" ");
    println!("Executing: {command} {args_joined}");

    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(path) = dir {
        cmd.current_dir(path);
    }

    let mut child = cmd.spawn()?;
    let status = child.wait()?;

    if status.success() {
        Ok(())
    } else {
        Err(KernelUpdaterError::CommandExecutionError {
            command: command.to_string(),
            args: args_joined,
            status,
        })
    }
}

/// Executes system utilities demanding stdout capture and parsing.
pub fn run_command_output(command: &str, args: &[&str]) -> Result<String, KernelUpdaterError> {
    let args_joined = args.join(" ");

    let output = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|source| KernelUpdaterError::Utf8OutputError {
            command: command.to_string(),
            source,
        })
    } else {
        Err(KernelUpdaterError::CommandExecutionError {
            command: command.to_string(),
            args: args_joined,
            status: output.status,
        })
    }
}

/// Detects available processing units safely.
pub fn get_cores(spare: usize) -> Result<usize, KernelUpdaterError> {
    let raw_cores = thread::available_parallelism()?.get();
    let computed_cores = if raw_cores > spare {
        raw_cores - spare
    } else {
        1
    };
    Ok(computed_cores)
}

/// Re-generates system boot menus targeting GRUB bootloader instances.
pub fn update_grub() -> Result<(), KernelUpdaterError> {
    println!("Updating GRUB entries...");
    run_command("update-grub", &[])
}
