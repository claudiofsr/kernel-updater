// lib.rs
use anyhow::{anyhow, Context, Result};
use std::{env, fs, process::Command};

mod args;
pub use args::{Arguments, Commands};

const KERNEL_URL_BASE: &str = "https://cdn.kernel.org/pub/linux/kernel/v6.x";
const KERNEL_SRC_BASE: &str = "/lib/modules";

pub fn kernel_compile(kernel_new: &str) -> Result<()> {
    let cores = get_cores(1)?;
    let config_file = format!("{KERNEL_SRC_BASE}/config-ClaudioFSR");
    let kernel_src_dir = format!("{KERNEL_SRC_BASE}/linux-{kernel_new}");
    let link = format!("{KERNEL_URL_BASE}/linux-{kernel_new}.tar.xz");

    println!("Compiling kernel {kernel_new} with {cores} cores...");

    env::set_current_dir(KERNEL_SRC_BASE)?;

    run_command("wget", &[&link])?;
    run_command("tar", &["-Jxvf", &format!("linux-{kernel_new}.tar.xz")])?;

    env::set_current_dir(&kernel_src_dir)?;

    // Build the kernel
    run_command("cp", &["-f", &config_file, ".config"])?;
    run_command("make", &["-j", &cores])?;
    run_command("make", &["modules_install"])?;
    run_command(
        "/usr/bin/cp",
        &["-fv", "arch/x86/boot/bzImage", "/boot/vmlinuz-6.12"],
    )?;

    println!("Kernel compilation completed successfully.\n");
    Ok(())
}

pub fn dkms_install(kernel_old: &str, kernel_new: &str) -> Result<()> {
    println!("Running DKMS installation...");

    let kernel_src_dir = format!("{KERNEL_SRC_BASE}/linux-{kernel_new}");
    env::set_current_dir(&kernel_src_dir)?;
    run_command("make", &["modules_install"])?;

    let dkms_versao = get_nvidia_version()?;

    dkms_build(kernel_new, &dkms_versao)?;

    dkms_remove(kernel_old, &dkms_versao)?;

    println!("DKMS installation completed successfully.\n");
    Ok(())
}

fn get_nvidia_version() -> Result<String> {
    let dkms_output = run_command_output("dkms", &["status"])?;

    if !dkms_output.contains("nvidia") {
        return Err(anyhow!("NVIDIA DKMS module not found"));
    }

    println!("DKMS status:\n{}", dkms_output);

    // DKMS status:
    // nvidia/550.135, 6.11.10-2-MANJARO, x86_64: installed
    // nvidia/550.135, 6.12.4-ClaudioFSR, x86_64: installed

    let nvidia_version = dkms_output
        .lines()
        .find(|&line| line.starts_with("nvidia"))
        .and_then(|line| line.split(['/', ',']).nth(1))
        .ok_or(anyhow!("Failed to extract nvidia version."))?;

    println!("DKMS nvidia version: {nvidia_version}");

    Ok(nvidia_version.to_string())
}

fn dkms_build(kernel_new: &str, dkms_versao: &str) -> Result<()> {
    let nvidia = format!("nvidia/{}", dkms_versao);
    let kernel = format!("{}-ClaudioFSR", kernel_new);
    let args = ["install", "--force", "--no-depmod", &nvidia, "-k", &kernel];

    run_command("dkms", &args)?;

    println!("DKMS build for {} completed successfully.", kernel_new);
    Ok(())
}

fn dkms_remove(kernel_old: &str, dkms_versao: &str) -> Result<()> {
    println!("Removing old DKMS files...");
    println!("kernel_old: {}", kernel_old);

    let dkms_file = format!(
        "/var/lib/dkms/nvidia/kernel-{}-ClaudioFSR-x86_64",
        kernel_old
    );

    if fs::remove_file(&dkms_file).is_ok() {
        println!("Removed dkms file: {dkms_file:?}");
    } else {
        println!("Not found dkms file: {dkms_file:?}");
    }

    let dkms_subdir = format!(
        "/var/lib/dkms/nvidia/{}/{}-ClaudioFSR",
        dkms_versao, kernel_old
    );

    if fs::remove_dir_all(&dkms_subdir).is_ok() {
        println!("Removed dkms dir: {dkms_subdir:?}");
    } else {
        println!("Not found dkms dir: {dkms_subdir:?}");
    }

    Ok(())
}

pub fn mkinitcpio() -> Result<()> {
    println!("Running mkinitcpio...");
    run_command("mkinitcpio", &["-p", "linux612_ClaudioFSR"])?;
    println!("mkinitcpio completed successfully.");
    Ok(())
}

pub fn update_grub() -> Result<()> {
    println!("Updating GRUB...");
    run_command("update-grub", &[])?;
    println!("GRUB update completed successfully.");
    Ok(())
}

pub fn get_cores(free: usize) -> Result<String> {
    let num_cpus = std::thread::available_parallelism()?.get();

    if num_cpus > free {
        Ok(format!("{}", num_cpus - free))
    } else {
        Ok(format!("{}", num_cpus))
    }
}

fn run_command(command: &str, args: &[&str]) -> Result<()> {
    Command::new(command)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run {} {:?}", command, args))?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("Command failed"))
}

fn run_command_output(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute command `{}`", command))?;

    if !output.status.success() {
        return Err(anyhow!(
            "
            Command failed with status: {status}\
            Error: {error}
            ",
            status = output.status,
            error = String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8(output.stdout)?)
}

pub fn quit() {
    std::process::exit(0);
}

/*
/// Executa comandos com argumentos.
///
/// * `command`: O comando a ser executado.
/// * `args`: Os argumentos do comando.
/// * `flush`: Uma flag que indica se o output deve ser impresso após a execução do comando.
///
fn run_command(command: &str, args: &[&str], flush: bool) -> Result<Output> {
    let child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    match child.wait_with_output() {
        Ok(output) => {
            // Se flush, imprir output imediatamente após a execução do comando.
            if flush {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                println!("{}", stdout);
                println!("{}", stderr);
            }

            Ok(output)
        }
        Err(err) => Err(anyhow!(
            "
            Comando: {command}\n\
            Argumentos: {args:?}\n\
            Erro: {err}\n
            "
        )),
    }
}
*/
