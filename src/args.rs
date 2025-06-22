use crate::Version;
use clap::{Parser, Subcommand, ValueEnum};

// --- Structs ---

/// A utility to automate steps for compiling, installing custom Linux kernels and managing NVIDIA DKMS modules.
///
/// This tool assists in upgrading your kernel by handling tasks such as:
/// - Downloading and compiling new kernel source code.
/// - Installing kernel modules and binaries.
/// - Managing NVIDIA DKMS modules for the new and old kernel versions.
///
/// Note: This utility requires root privileges (`sudo`) for most operations involving writing to system directories like /lib/modules, /boot.
/// It is currently tailored for a specific setup (Arch/Manjaro-like paths, GRUB, NVIDIA DKMS, custom suffix).
///
#[derive(Parser, Debug, Clone)]
#[command(author, version, about)] // Let the struct doc comment be long_about
#[command(after_help = "
Examples:

  Compile source tree for 6.15.4:
  sudo kernel-updater -n 6.15.4 kernel-compile

  Install compiled 6.15.4 kernel:
  sudo kernel-updater -n 6.15.4 kernel-install # Assumes 6.15.4 compiled in /lib/modules/linux-6.15.4

  Build/install DKMS for 6.15.4, remove for 6.15.3:
  sudo kernel-updater -o 6.15.3 -n 6.15.4 dkms-install # Assumes 6.15.4 already installed

  Full update (compile, install, dkms update):
  sudo kernel-updater -o 6.15.3 -n 6.15.4

WARNING: For the default operation (no command) and 'dkms-install' command, the NEW version (-n) must be strictly greater than the OLD version (-o). 
This validation is performed after parsing.
E.g., kernel-updater -o 6.15.3 -n 6.15.4 is valid, but -o 6.15.4 -n 6.15.3 or -o 6.15.4 -n 6.15.4 will fail validation.
")]
pub struct Arguments {
    /// Optional subcommand to execute. If none is specified, a full update (compile, install, dkms) is performed.
    #[command(subcommand)]
    pub command: Option<Commands>, // Optional subcommand

    /// Download Linux kernel with curl or wget
    #[arg(short,
        long,
        value_enum,
        default_value_t = Downloader::default(),
        help = "Downloader program to use (curl or wget)"
    )]
    pub downloader: Downloader,

    /// The Kernel suffix
    #[arg(short, long, default_value = "ClaudioFSR", help = "The Kernel suffix")]
    pub suffix: String,

    /// The new kernel version (Major.Minor.Patch, e.g., "6.15.4").
    #[arg(
        short,
        long,
        required = true, // Always required
        help = "The new kernel version (e.g., \"6.15.4\")"
    )] // Added help
    pub new: Version, // Parsed directly into a Version

    /// The old kernel version ( Major.Minor.Patch, e.g., "6.15.3").
    #[arg(
        short,
        long,
        required = false, // Only conditionally required based on command - validated in Config::new
        help = "The old kernel version (e.g., \"6.15.3\")",
        long_help = "The old kernel version (Major.Minor.Patch, e.g., \"6.15.3\").\n\
        Required for DKMS operations ('dkms-install') or the default command.\n\
        If provided with these commands, it must be strictly less than the --new version (validated later)."
    )] // Updated long_help to indicate where validation occurs
    pub old: Option<Version>, // Parsed into an Option<Version>
}

// --- Enums ---

/// Available subcommands for the kernel updater utility.
/// See --help for detailed command descriptions and examples.
#[derive(Subcommand, Debug, Clone, PartialEq)] // Added PartialEq for testing
pub enum Commands {
    /// Download, configure, and compile the new kernel source code.
    #[command(name = "kernel-compile", about = "Compile the new kernel source")] // Added about
    KernelCompile,

    /// Install the compiled kernel modules and binary to system directories (/lib/modules, /boot).
    /// Requires a compiled source tree for --new to exist. Runs mkinitcpio and update-grub.
    #[command(name = "kernel-install", about = "Install the compiled kernel")] // Added about
    KernelInstall,

    /// Build and install DKMS modules for the new kernel, and remove old modules.
    /// Requires --new AND --old, and NEW > OLD. Runs mkinitcpio and update-grub.
    #[command(name = "dkms-install", about = "Build/install DKMS modules")] // Added about
    DkmsInstall,
}

#[derive(Debug, Default, Clone, ValueEnum, PartialEq)]
pub enum Downloader {
    #[default]
    Curl,
    Wget,
}
