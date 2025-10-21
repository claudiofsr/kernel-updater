use crate::{Version, args::Commands};
use std::{num::ParseIntError, path::PathBuf, process::ExitStatus, string::FromUtf8Error};
use thiserror::Error;

/**
Result type to simplify function signatures.

This is a custom result type that uses our custom `KernelUpdaterError` for the error type.

Functions can return `KernelUpdaterResult<T>` and then use `?` to automatically propagate errors.
*/
pub type KernelUpdaterResult<T> = Result<T, KernelUpdaterError>;

#[derive(Error, Debug)]
pub enum KernelUpdaterError {
    // --- Basic OS/Process Errors ---
    // Use #[from] to allow automatic conversion from std::io::Error using the `?` operator
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    // Error for external command execution failure (non-zero exit code)
    // Does NOT include spawning errors (those are handled by IoError via `#[from]`).
    #[error("Command '{command} {args}' failed with status: {status}")]
    CommandExecutionError {
        command: String,
        args: String, // Join args for cleaner display
        status: ExitStatus,
    },

    // Error for external command returning invalid UTF-8 output
    #[error("Command '{command}' succeeded but output is not valid UTF-8: {source}")]
    Utf8OutputError {
        command: String,
        #[source]
        source: FromUtf8Error,
    },

    // --- Configuration/Validation Errors ---
    #[error(
        "Configuration validation failed: --new version ({new}) must be strictly greater than --old version ({old})"
    )]
    VersionComparisonError { new: Version, old: Version },

    #[error(
        "Configuration validation failed: {argument_name} argument is required for command {:?}",
        command
    )]
    MissingRequiredArgument {
        argument_name: String,     // e.g., "--old"
        command: Option<Commands>, // e.g., Some(Commands::DkmsInstall) or None (for default)
    },

    // --- DKMS-specific Errors ---
    #[error(
        "NVIDIA DKMS module entry not found in `dkms status`. Is the NVIDIA driver installed via DKMS?"
    )]
    DkmsModuleNotFound,

    // Specific error for parsing the output of `dkms status`
    #[error(
        "Failed to parse NVIDIA module version from `dkms status` output: {reason}. Output was:\n{output}"
    )]
    DkmsStatusParseError {
        output: String, // Include the actual output that failed parsing
        reason: String,
    },

    // --- Kernel File/Path/Build Errors ---
    #[error("Kernel config file not found at {}", path.display())]
    KernelConfigNotFound { path: PathBuf },

    #[error(
        "Kernel source tree ({}) for version {} is not configured.\n\
        Required '.config' file is missing.\n\
        Did `make olddefconfig` or equivalent fail or not run?",
        src_dir.display(),
        version
    )]
    KernelNotConfigured { src_dir: PathBuf, version: Version },

    #[error("Compiled kernel binary not found at {}.\n\
    Kernel source tree ({}) for version {} does not appear to be compiled.", path.display(), src_dir.display(), version)]
    KernelBinaryNotFound {
        path: PathBuf,
        src_dir: PathBuf,
        version: Version,
    },

    // --- Version Parsing Errors ---
    #[error("Invalid version component: failed to parse as integer ({source})")]
    VersionParseIntError {
        #[from]
        source: ParseIntError,
    }, // Automatic conversion from ParseIntError

    #[error(
        "Invalid version format '{input}': expected exactly three dot-separated numbers (e.g., X.Y.Z as 6.15.3)"
    )]
    VersionParseFormatError { input: String },
}
