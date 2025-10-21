# kernel-updater

Automates common steps for building/installing custom Linux kernels and managing NVIDIA DKMS modules.

**Note:** Tailored for Arch/Manjaro systems (GRUB, NVIDIA DKMS, specific paths/suffixes). Modifications needed for other distributions.

## Prerequisites

*   Rust and Cargo
*   Standard kernel build tools (gcc, make, flex, bison, openssl, etc.)
*   External commands: `curl` (or `wget`), `tar`, `dkms`, `mkinitcpio`, `update-grub`
*   Root privileges (`sudo`)
*   A compatible base kernel `.config` file at `/lib/modules/config-<your_suffix>`. Ensure DKMS and required options are enabled. (Suffix defined in `src/config.rs`).

## Building and Installation

1.  Clone the repo: `git clone https://github.com/claudiofsr/kernel-updater.git`
2.  Change the working directory: `cd kernel-updater`
3.  Build and install: `cargo b -r && cargo install --path=.`

## Usage

Run with `sudo`: `sudo kernel-updater [OPTIONS] [COMMAND]`

**OPTIONS:**
*   `-n`, `--new <VER>` (Required): New kernel version (X.Y.Z).
*   `-o`, `--old <VER>` (Optional, req for some cmds): Old kernel version (X.Y.Z). Must be `< --new` for default/`dkms-install`.

**COMMANDS:**
*   *(Default)*: Full update: Compile, Install kernel & DKMS, Update boot. Requires `-n > -o`.
*   `kernel-compile`: Download and compile new kernel source. Requires `-n`.
*   `kernel-install`: Install *compiled* new kernel (modules, binary, symlinks). Requires `-n`. Assumes source is compiled. Runs `mkinitcpio`/`update-grub`.
*   `dkms-install`: Update NVIDIA DKMS (remove old, build/install new). Requires `-n > -o`. Requires `--new` kernel is already installed. Runs `mkinitcpio`/`update-grub`.

## Examples

Assuming update from 6.15.3 to 6.15.4:

*   Full update: `sudo kernel-updater -o 6.15.3 -n 6.15.4`
*   Compile 6.15.4 only: `sudo kernel-updater -n 6.15.4 kernel-compile`
*   Install 6.15.4 (after compile): `sudo kernel-updater -n 6.15.4 kernel-install`
*   Update DKMS for 6.15.4/6.15.3 (after 6.15.4 installed): `sudo kernel-updater -o 6.15.3 -n 6.15.4 dkms-install`

## Important Validation

For the default command and `dkms-install`, the NEW version (`-n`) must be strictly greater than the OLD version (`-o`).
*   Valid: `kernel-updater -o 6.15.3 -n 6.15.4`
*   Invalid: `kernel-updater -o 6.15.4 -n 6.15.3` (NEW <= OLD fails validation)

## Notes & Warnings

*   Requires `sudo`.
*   **System Specific:** Highly tailored for Arch/Manjaro (paths, tools, suffix, GRUB). Requires source modification for other distributions.
*   **DKMS Specific:** Only manages the 'nvidia' DKMS module currently.
*   **Kernel Config:** A correct base `.config` is essential for a successful build.
*   **Risky:** Kernel building/installing is risky. Ensure backups and know recovery procedures (e.g., booting a working kernel via GRUB).
