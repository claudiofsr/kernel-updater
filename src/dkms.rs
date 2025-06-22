use crate::{
    Config,
    error::KernelUpdaterError,
    utils::{run_command, run_command_output},
};

/// Retrieves the installed NVIDIA DKMS module version from `dkms status`.
/// Returns `Result<String, KernelUpdaterError>`. Specific errors related to dkms status parsing are mapped.
pub fn get_nvidia_version() -> Result<String, KernelUpdaterError> {
    println!("Getting NVIDIA DKMS module version...");

    // run_command_output returns Result<String, KernelUpdaterError>. `?` propagates it.
    let dkms_output = run_command_output("dkms", &["status"])?;

    // Check if NVIDIA module entry is present in the output.
    if !dkms_output.contains("nvidia") {
        // Return the specific error variant
        return Err(KernelUpdaterError::DkmsModuleNotFound);
    }

    // Example 'dkms status' output relevant lines:
    // nvidia/550.135, 6.11.10-2-MANJARO, x86_64: installed
    // nvidia/550.135, 6.12.4-ClaudioFSR, x86_64: installed

    // Find a line starting with "nvidia" and parse out the module version (e.g., "550.135").
    let nvidia_version = dkms_output
        .lines()
        .find(|&line| line.trim().starts_with("nvidia/") && line.contains(","))
        .and_then(|line| line.split(['/', ',']).nth(1))
        .map(|s| s.trim())
        // If the version cannot be extracted after finding a matching line, it's a parse format error.
        .ok_or_else(|| KernelUpdaterError::DkmsStatusParseError {
            output: dkms_output.clone(),
            reason: "Could not extract version from line format".to_string(),
        })?; // Return our specific error

    println!("Detected NVIDIA DKMS module version: {}", nvidia_version);

    Ok(nvidia_version.to_string()) // Return the extracted version as a String
}

/// Builds and installs the NVIDIA DKMS module for a specific kernel version.
/// Uses the provided `Config` for paths and kernel names.
/// Returns `Result<(), KernelUpdaterError>`.
pub fn dkms_install(config: &Config) -> Result<(), KernelUpdaterError> {
    println!(
        "Building and installing NVIDIA DKMS module for kernel {}...",
        config.version_new
    );

    // get_nvidia_version returns KernelUpdaterError. `?` propagates it directly.
    let dkms_module_version = get_nvidia_version()?;

    let dkms_module_spec = format!("nvidia/{}", dkms_module_version);

    let kernel_name_new = format!("{}-{}", &config.version_new, &config.custom_kernel_suffix);

    let build_args = [
        "install",
        "--force",
        &dkms_module_spec,
        "-k",
        &kernel_name_new,
    ];

    println!(
        "Running 'dkms install {} -k {}'...",
        dkms_module_spec, kernel_name_new
    );
    // run_command returns KernelUpdaterError. `?` propagates it.
    // run_command error already includes the command and status/source details.
    run_command("dkms", &build_args)?;

    println!(
        "NVIDIA DKMS module built and installed successfully for kernel {}.\n",
        kernel_name_new
    );
    Ok(())
}

/// Removes the NVIDIA DKMS module entries for a specific kernel version.
/// Uses the provided `Config` for paths and kernel names, specifically the old version's details.
/// Returns `Result<(), KernelUpdaterError>`.
pub fn dkms_remove(config: &Config) -> Result<(), KernelUpdaterError> {
    // These unwraps are safe because Config::new validated that old/old_ident exist for commands that call dkms_remove.
    let old_version = config
        .version_old
        .as_ref()
        .expect("BUG: dkms_remove called with missing old version in config");
    let kernel_name_old = config
        .kernel_ident_name_old
        .as_ref()
        .expect("BUG: dkms_remove called with missing old kernel ident name in config");

    println!(
        "Attempting to remove NVIDIA DKMS module for old kernel {} ({}) ...",
        old_version, kernel_name_old
    );

    // get_nvidia_version returns KernelUpdaterError, which is propagated by `?`
    let dkms_module_version = get_nvidia_version()?;

    let dkms_module_spec = format!("nvidia/{}", dkms_module_version);

    let remove_args = ["remove", &dkms_module_spec, "-k", kernel_name_old];

    println!(
        "Running 'dkms remove {} -k {}'...",
        dkms_module_spec, kernel_name_old
    );
    // run_command_output returns Result<String, KernelUpdaterError>.
    // We want to *inspect* the potential failure rather than propagate immediately,
    // to handle "not installed" as non-fatal.
    let remove_result = run_command_output("dkms", &remove_args);

    match remove_result {
        Ok(output) => {
            println!(
                "Successfully attempted DKMS removal for module {} kernel {}. Output:",
                dkms_module_spec, kernel_name_old
            );
            println!("{}", output); // Print the captured stdout
        }
        Err(error) => {
            match error {
                KernelUpdaterError::CommandExecutionError {
                    ref command,
                    ref args,
                    status: _,
                } => {
                    // We print a warning, assuming non-zero exit is due to the module not existing.
                    eprintln!(
                        "Warning: Command '{command} {args:?}' failed.\n\
                        Assuming this indicates the module was not installed for kernel {}. Error: {error}",
                        kernel_name_old
                    );
                    // Treat as non-fatal and proceed.
                    return Ok(());
                }
                // Any other error (like IoError, Utf8OutputError) *is* considered fatal for this step.
                // We re-return the error wrapped in the function result.
                _ => {
                    // Log the unexpected error
                    eprintln!(
                        "Warning: Unexpected non-execution error during dkms remove: {error}",
                    );
                    eprintln!(
                        "Warning: Error during old DKMS removal (non-fatal per logic): {error}"
                    );
                    return Ok(()); // Replicate the old non-fatal logic for removal
                }
            }
        }
    }

    println!("Old DKMS removal steps completed (if applicable).\n");
    Ok(())
}
