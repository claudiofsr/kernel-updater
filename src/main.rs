use clap::Parser;
use kernel_updater::{
    Arguments, Commands, Config, DkmsManager, KernelBuilder, KernelUpdaterResult, update_grub,
};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("\nExecution stopped due to a fatal error:");
        eprintln!("Error: {e}");
        process::exit(1);
    }
    println!("Execution completed with status: Success");
}

fn run() -> KernelUpdaterResult<()> {
    let args = Arguments::parse();
    let config = Config::new(args)?;

    config.show_summary();

    let builder = KernelBuilder::new(&config);
    let dkms = DkmsManager::new(&config);

    match &config.command {
        Some(Commands::KernelCompile) => {
            println!("Executing: Kernel Compilation...");
            builder.compile()?;
        }
        Some(Commands::KernelInstall) => {
            println!("Executing: Kernel Installation...");
            builder.install()?;
            builder.run_mkinitcpio()?;
            update_grub()?;
        }
        Some(Commands::DkmsInstall) => {
            println!("Executing: DKMS Configuration...");
            dkms.remove_modules()?;
            dkms.install_modules()?;
            builder.run_mkinitcpio()?;
            update_grub()?;
        }
        None => {
            println!("Executing sequence: Complete Upgrade Pipeline...");

            println!("\n--- Phase 1 of 4: Compiling Source Tree ---");
            builder.compile()?;

            println!("\n--- Phase 2 of 4: Installing Target Kernel Tree ---");
            builder.install()?;

            println!("\n--- Phase 3 of 4: Updating DKMS Registries ---");
            dkms.remove_modules()?;
            dkms.install_modules()?;

            println!("\n--- Phase 4 of 4: Rebuilding Boot Configurations ---");
            builder.run_mkinitcpio()?;
            update_grub()?;

            if let Some(ref old) = config.version_old {
                println!(
                    "\nKernel updated successfully: {old} -> {}",
                    config.version_new
                );
            }
        }
    }

    Ok(())
}
