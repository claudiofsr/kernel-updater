mod args;
mod config;
mod dkms;
mod error;
mod kernel;
mod utils;
mod version;

pub use args::{Arguments, Commands, Downloader};
pub use config::Config;
pub use dkms::{dkms_install, dkms_remove, get_nvidia_version};
pub use error::{KernelUpdaterError, KernelUpdaterResult};
pub use kernel::{kernel_compile, kernel_install, mkinitcpio};
pub use utils::{get_cores, run_command, run_command_output, update_grub};
pub use version::{Version, get_version};
