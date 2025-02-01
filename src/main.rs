// main.rs
// kernel_updater -o 6.12.3 -n 6.12.4 dkms-install
use anyhow::Result;
use clap::Parser;
use kernel_updater::*;

fn main() -> Result<()> {
    let args = Arguments::parse();

    println!("kernel_old: {}", args.old);
    println!("kernel_new: {}\n", args.new);
    println!("Update kernel version: {} -> {}\n", args.old, args.new);

    match &args.command {
        Some(Commands::DkmsInstall) => dkms_install(&args.old, &args.new)?,
        Some(Commands::KernelCompile) => kernel_compile(&args.new)?,
        None => {
            kernel_compile(&args.new)?;
            dkms_install(&args.old, &args.new)?;
        }
    }

    mkinitcpio()?;
    update_grub()?;

    println!("All operations completed successfully.");
    Ok(())
}
