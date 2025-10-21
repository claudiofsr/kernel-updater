use crate::{
    Config, Downloader,
    error::KernelUpdaterError,
    utils::{get_cores, run_command},
};
use std::{env, fs, io::ErrorKind, os::unix::fs as unix_fs, path::PathBuf};

/// Downloads, extracts, configures, and compiles the new kernel source code.
/// The compiled source tree is left in config.kernel_src_dir_path.
pub fn kernel_compile(config: &Config) -> Result<(), KernelUpdaterError> {
    println!(
        "Starting kernel compilation for version {}...",
        config.version_new
    );

    // Get paths and names from Config
    let config_file = &config.config_file_path;
    let kernel_src_dir_path = &config.kernel_src_dir_path;
    let kernel_src_base = &config.kernel_src_base;
    let tarball_name = &config.tarball_name;
    let download_link = &config.download_link;

    // Ensure the kernel source base directory exists.
    // fs::create_dir_all returns Result<(), std::io::Error>. `?` propagates it,
    // mapping automatically to KernelUpdaterError::IoError thanks to #[from].
    println!(
        "Ensuring kernel source base directory exists: {}",
        kernel_src_base.display()
    );
    fs::create_dir_all(kernel_src_base)?; // Handles io::Error

    println!(
        "Change directory to the kernel source base: '{}'",
        kernel_src_base.display()
    );
    // Change directory. env::set_current_dir returns Result<(), std::io::Error>.
    env::set_current_dir(kernel_src_base)?; // Handles io::Error

    // Download the kernel source tarball.
    println!("Downloading kernel source from {}", download_link);
    // run_command returns Result<(), KernelUpdaterError>. `?` propagates it.
    match config.downloader {
        Downloader::Curl => {
            let curl_args = &["-fL", download_link, "-o", tarball_name];
            run_command("curl", curl_args) // Error propagated by `?`
        }
        Downloader::Wget => run_command("wget", &[download_link]), // Error propagated by `?`
    }?;

    println!("\nDownload complete.");
    println!("Extracting {}...", tarball_name);
    let tar_args = &["-Jxvf", tarball_name];
    run_command("tar", tar_args)?; // Error propagated by `?`

    // Change directory to the extracted kernel source directory for configuring and building.
    println!(
        "Changing directory to extracted source: {}",
        kernel_src_dir_path.display()
    );
    env::set_current_dir(kernel_src_dir_path)?; // Handles io::Error

    // Copy the existing kernel configuration file to the source directory.
    // Check if the config file exists before copying. Handle specific NotFound error.
    match fs::metadata(config_file) {
        // Returns Result<Metadata, std::io::Error>
        Ok(_) => {
            println!(
                "Copying config from {} to .config...",
                config_file.display()
            );
            // run_command returns KernelUpdaterError. Propagated by `?`
            run_command("/usr/bin/cp", &[&config_file.to_string_lossy(), ".config"])?;
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // Explicitly return our custom NotFound error for the config file.
            // No `.into()` needed now that function returns KernelUpdaterError directly.
            return Err(KernelUpdaterError::KernelConfigNotFound {
                path: config_file.clone(),
            });
        }
        Err(e) => {
            // Other std::io::Error during metadata lookup (e.g., permissions).
            // Convert to KernelUpdaterError::IoError implicitly by ?.
            Err(e)?;
        }
    }

    // Optional but recommended: Update config based on new source
    println!("Running 'make olddefconfig' to update kernel configuration...");
    // run_command returns KernelUpdaterError. Propagated by `?`
    run_command("make", &["olddefconfig"])?;

    // Check if the .config file exists after olddefconfig. It should.
    let dot_config_path = PathBuf::from(".config");
    match fs::metadata(&dot_config_path) {
        Ok(_) => {
            println!(".config file confirmed to exist after olddefconfig.");
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // This indicates a failure in make olddefconfig or a bad source tree.
            // Return a specific error indicating the source tree wasn't configured.
            return Err(KernelUpdaterError::KernelNotConfigured {
                src_dir: kernel_src_dir_path.clone(),
                version: config.version_new.clone(),
            });
        }
        Err(e) => {
            // Other std::io::Error during metadata lookup. Let `?` handle it.
            Err(e)?;
        }
    }

    // Determine the number of cores to use for compilation.
    // get_cores returns Result<String, KernelUpdaterError>. `?` propagates it.
    let cores = get_cores(1)?;

    println!("Using {} cores for compilation.", cores);

    // Run the main kernel build.
    println!("Running 'make' with -j {}...", cores);
    // run_command returns KernelUpdaterError. Propagated by `?`
    run_command("make", &["-j", &cores])?;

    println!(
        "Kernel compilation completed successfully in {}.\n",
        kernel_src_dir_path.display()
    );
    Ok(())
}

/// Installs the compiled kernel modules and binary.
/// This step requires the kernel source tree for config.version_new to be already compiled
/// in the expected location (config.kernel_src_dir_path).
/// Uses the provided `Config` for paths and version names.
/// Returns `Result<(), KernelUpdaterError>`.
pub fn kernel_install(config: &Config) -> Result<(), KernelUpdaterError> {
    println!(
        "Starting kernel installation for version {}...",
        config.version_new
    );

    // Get paths from Config
    let kernel_src_dir_path = &config.kernel_src_dir_path;
    let kernel_ident_name = format!("{}-{}", &config.version_new, &config.custom_kernel_suffix);
    let vmlinuz_path = &config.vmlinuz_install_path;
    let modules_install_base = &config.kernel_module_base;
    let modules_install_path = modules_install_base.join(kernel_ident_name);

    // Ensure we are in the compiled kernel source directory for installation commands.
    println!(
        "Changing directory to compiled source: {}",
        kernel_src_dir_path.display()
    );
    env::set_current_dir(kernel_src_dir_path)?; // Handles io::Error

    // Check if the kernel binary exists. Handle specific NotFound error.
    let bzimage_path_in_source = PathBuf::from("arch/x86/boot/bzImage");
    match fs::metadata(&bzimage_path_in_source) {
        // Returns Result<Metadata, std::io::Error>
        Ok(_) => {
            println!(
                "Verified compiled kernel binary exists at {}.",
                bzimage_path_in_source.display()
            );
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // Explicitly return our custom BinaryNotFound error.
            // No `.into()` needed.
            return Err(KernelUpdaterError::KernelBinaryNotFound {
                path: bzimage_path_in_source.clone(),
                src_dir: kernel_src_dir_path.clone(),
                version: config.version_new.clone(),
            });
        }
        Err(e) => {
            // Other std::io::Error during metadata lookup. Let `?` convert it.
            Err(e)?;
        }
    }

    // Install kernel modules to /lib/modules/<version>-<suffix>.
    println!("Running 'make modules_install'...");
    // run_command returns KernelUpdaterError. Propagated by `?`. Requires root.
    run_command("make", &["modules_install"])?;

    println!(
        "Kernel modules installed to {}",
        modules_install_path.display()
    );

    // Copy the compiled kernel image (bzImage) to the boot directory.
    println!("Copying bzImage to {}...", vmlinuz_path.display());
    // run_command returns KernelUpdaterError. Propagated by `?`. Requires root.
    run_command(
        "/usr/bin/cp",
        &[
            &bzimage_path_in_source.to_string_lossy(),
            &vmlinuz_path.to_string_lossy(),
        ],
    )?;

    println!("Kernel binary copied.");

    // Handle symlinks (build and source) using helper. Helper returns Result<(), KernelUpdaterError>.
    let modules_build_link_target = kernel_src_dir_path;
    let modules_build_link_path = modules_install_path.join("build");
    let modules_source_link_path = modules_install_path.join("source");

    println!(
        "Ensuring symlink from {} points to the source directory {}...",
        modules_build_link_path.display(),
        modules_build_link_target.display()
    );
    // ensure_symlink returns KernelUpdaterError. Propagated by `?`.
    ensure_symlink(&modules_build_link_path, modules_build_link_target)?;

    println!(
        "Ensuring symlink from {} points to the source directory {}...",
        modules_source_link_path.display(),
        modules_build_link_target.display()
    );
    // ensure_symlink returns KernelUpdaterError. Propagated by `?`.
    ensure_symlink(&modules_source_link_path, modules_build_link_target)?;

    println!("Kernel installation completed.\n");
    Ok(())
}

/// Helper function to remove an existing file/symlink/dir and create a new symlink.
/// Handles `std::io::Error` which gets implicitly converted to `KernelUpdaterError::IoError`
/// via the `#[from]` attribute and the `?` operator.
/// Returns Result<(), KernelUpdaterError>.
fn ensure_symlink(link_path: &PathBuf, link_target: &PathBuf) -> Result<(), KernelUpdaterError> {
    // fs::symlink_metadata returns Result<Metadata, std::io::Error>. Using `?` converts it to IoError.
    // Handle NotFound specifically, otherwise allow ? to convert other errors.
    match fs::symlink_metadata(link_path) {
        Ok(metadata) => {
            println!(
                "Removing existing link/file/dir at {}...",
                link_path.display()
            );
            if metadata.is_dir() {
                // fs::remove_dir returns Result<(), std::io::Error>. `?` handles it (-> IoError).
                fs::remove_dir(link_path)?;
            } else {
                // fs::remove_file returns Result<(), std::io::Error>. `?` handles it (-> IoError).
                fs::remove_file(link_path)?;
            }
        }
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            // This is fine, the link just doesn't exist. Do nothing and proceed to create it.
        }
        Err(e) => {
            // Other std::io::Error during metadata lookup. Let `?` convert it.
            Err(e)?; // Propagates as IoError
        }
    }

    // unix_fs::symlink returns Result<(), std::io::Error`. `?` handles it (-> IoError). Requires root.
    unix_fs::symlink(link_target, link_path)?;

    println!(
        "Symlink created/updated successfully for {}.",
        link_path.display()
    );
    Ok(())
}

/// Runs mkinitcpio to generate the initial ramdisk for the specified kernel version.
/// Uses the provided `Config`.
/// Returns `Result<(), KernelUpdaterError>`. Errors from `run_command` will be the source.
pub fn mkinitcpio(config: &Config) -> Result<(), KernelUpdaterError> {
    // Generate the profile name using Config.
    let mkinitcpio_profile_name = format!(
        "linux{}{}_{}",
        config.version_new.major, config.version_new.minor, config.custom_kernel_suffix
    );

    println!(
        "Running mkinitcpio for kernel version {} with profile {}...",
        config.version_new, mkinitcpio_profile_name
    );
    // run_command returns KernelUpdaterError. Propagated by `?`. Requires root.
    run_command("mkinitcpio", &["-p", &mkinitcpio_profile_name])?;

    println!("mkinitcpio completed successfully.");
    Ok(())
}
