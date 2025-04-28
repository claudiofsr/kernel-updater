use clap::Parser;
use kernel_updater::{
    Arguments, Commands, Config, KernelUpdaterResult, dkms_install, dkms_remove, kernel_compile,
    kernel_install, mkinitcpio, update_grub,
};
use std::process;

fn main() {
    // Call the separate function that contains the main logic and can return Result
    let run_result = run();

    // Now handle the result returned by the 'run' function
    match run_result {
        Ok(_) => {
            println!("All requested operations finished.");
            process::exit(0); // Explicitly exit with success code
        }
        Err(error) => {
            eprintln!("Operation failed:");
            eprintln!("Error: {}", error); // Using Display prints the #[error] message
            process::exit(1); // Explicitly exit with failure code
        }
    }
}

fn run() -> KernelUpdaterResult<()> {
    let args = Arguments::parse();

    let config = Config::new(args)?;
    config.show_summary();

    match &config.command {
        Some(Commands::KernelCompile) => {
            println!("Executing 'kernel-compile' subcommand...");
            kernel_compile(&config)?;
            println!(
                "Kernel compilation complete.\n\
                Binary and modules not installed.\n\
                Run 'kernel-install' or default command to install.\n"
            );
            Ok(())
        }

        Some(Commands::KernelInstall) => {
            println!("Executing 'kernel-install' subcommand...");
            kernel_install(&config)?;
            mkinitcpio(&config)?;
            update_grub()?;
            println!(
                "Kernel installation complete. Kernel {} is installed.\n",
                config.version_new
            );
            Ok(())
        }

        Some(Commands::DkmsInstall) => {
            println!("Executing 'dkms-install' subcommand...");
            dkms_remove(&config)?;
            dkms_install(&config)?;
            mkinitcpio(&config)?;
            update_grub()?;
            println!(
                "DKMS installation steps complete for kernel {}.\n",
                config.version_new
            );
            Ok(())
        }

        None => {
            // Default operation: compile + install + dkms
            println!("Executing default operation (kernel compile, install, DKMS install)...");

            println!("--- Step 1: Kernel Compilation ---");
            kernel_compile(&config)?;

            println!("--- Step 2: Kernel Installation ---");
            kernel_install(&config)?;

            println!("--- Step 3: DKMS Installation ---");
            dkms_remove(&config)?;
            dkms_install(&config)?;

            println!("--- Step 4: Update Boot ---");
            mkinitcpio(&config)?;
            update_grub()?;

            if let Some(version_old) = &config.version_old {
                println!(
                    "\nKernel updated successfully: {} -> {}\n",
                    version_old, config.version_new,
                );
            }
            Ok(())
        }
    }
}
