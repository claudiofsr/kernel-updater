use crate::{
    Version,
    args::{Arguments, Commands, Downloader},
    error::KernelUpdaterError,
};
use std::path::PathBuf;

/// Represents the final, validated configuration derived from command-line arguments and constants.
/// Contains all paths, versions, and settings needed to perform an operation.
/// This struct performs necessary semantic validation based on the chosen command.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub version_old: Option<Version>,
    pub version_new: Version,
    pub command: Option<Commands>,
    pub kernel_url_base: String,
    pub kernel_src_base: PathBuf,
    pub kernel_module_base: PathBuf,
    pub kernel_config_base: PathBuf,
    pub custom_kernel_suffix: String,
    pub config_file_path: PathBuf,
    pub kernel_src_dir_name: String,
    pub kernel_src_dir_path: PathBuf,
    pub tarball_name: String,
    pub download_link: String,
    pub kernel_ident_name_new: String,
    pub kernel_ident_name_old: Option<String>,
    pub vmlinuz_install_path: PathBuf,
    pub downloader: Downloader,
}

impl Config {
    /// Creates a new `Config` instance from the parsed `Arguments`.
    ///
    /// Performs validation:
    /// 1. If `--old` is provided, validates that `--new > --old`.
    /// 2. If the command requires `--old` (dkms-install or default), validates that `--old` is provided.
    ///
    /// Returns `KernelUpdaterError` on failure.
    pub fn new(args: Arguments) -> Result<Self, KernelUpdaterError> {
        // Return our specific error
        // Standard, often distribution-dependent constants
        let kernel_url_base = "https://cdn.kernel.org/pub/linux/kernel/v6.x".to_string();
        let kernel_src_base = PathBuf::from("/lib/modules");
        let kernel_module_base = PathBuf::from("/lib/modules");
        let kernel_config_base = PathBuf::from("/lib/modules");
        let custom_kernel_suffix = args.suffix;

        // --- Validation 1: If old version is provided, new MUST be strictly greater ---
        if let Some(old_version) = &args.old
            && &args.new <= old_version
        {
            return Err(KernelUpdaterError::VersionComparisonError {
                new: args.new.clone(),
                old: old_version.clone(),
            });
        }

        // --- Validation 2: Check if --old is required by the command ---
        match &args.command {
            Some(Commands::DkmsInstall) | None => {
                // Commands that require --old
                if args.old.is_none() {
                    return Err(KernelUpdaterError::MissingRequiredArgument {
                        argument_name: "--old".to_string(),
                        command: args.command.clone(),
                    });
                }
            }
            Some(Commands::KernelCompile) | Some(Commands::KernelInstall) => {
                // These commands do NOT require --old.
            }
        }

        // --- Calculate Derived Paths and Names (Only reached if all validation passes) ---
        let config_file_path = kernel_config_base.join(format!("config-{}", &custom_kernel_suffix));

        let kernel_src_dir_name = if args.new.patch == 0 {
            format!("linux-{}.{}", &args.new.major, &args.new.minor)
        } else {
            format!("linux-{}", &args.new)
        };

        let kernel_src_dir_path = kernel_src_base.join(&kernel_src_dir_name);

        let tarball_name = if args.new.patch == 0 {
            format!("linux-{}.{}.tar.xz", &args.new.major, &args.new.minor)
        } else {
            format!("linux-{}.tar.xz", &args.new)
        };

        let download_link = format!("{}/{}", &kernel_url_base, &tarball_name);

        let kernel_ident_name_new = if args.new.patch == 0 {
            format!(
                "{}.{}-{}",
                &args.new.major, &args.new.minor, &custom_kernel_suffix
            )
        } else {
            format!("{}-{}", &args.new, &custom_kernel_suffix)
        };

        let kernel_ident_name_old = args
            .old
            .as_ref()
            .map(|v| format!("{}-{}", v, &custom_kernel_suffix));
        let vmlinuz_install_path =
            PathBuf::from("/boot").join(format!("vmlinuz-{}.{}", args.new.major, args.new.minor));

        // --- Return the populated Config struct ---
        Ok(Self {
            version_old: args.old,
            version_new: args.new,
            command: args.command,
            kernel_url_base,
            kernel_src_base,
            kernel_module_base,
            kernel_config_base,
            custom_kernel_suffix,
            config_file_path,
            kernel_src_dir_name,
            kernel_src_dir_path,
            tarball_name,
            download_link,
            kernel_ident_name_new,
            kernel_ident_name_old,
            vmlinuz_install_path,
            downloader: args.downloader,
        })
    }

    /// Show summary information
    pub fn show_summary(&self) {
        println!("Running with configuration:");
        if let Some(old) = &self.version_old {
            println!("  Old version: {:?}", old);
        }
        println!("  New version: {:?}", self.version_new);
        println!("  Command: {:?}\n", self.command);

        println!("  Downloader: {:?}", self.downloader);
        println!("  Kernel Source Base: {}", self.kernel_src_base.display());
        println!("  Custom Suffix: {}", self.custom_kernel_suffix);
        println!("  New Kernel Ident: {}", self.kernel_ident_name_new);
        if let Some(old_ident) = &self.kernel_ident_name_old {
            println!("  Old Kernel Ident: {}", old_ident);
        }
        println!();
    }
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

/// Run tests with:
/// cargo test -- --show-output tests_config
#[cfg(test)]
mod tests_config {
    use super::*;
    use crate::Version;
    use crate::args::{Arguments, Commands, Downloader};
    use std::str::FromStr;

    // Helper to create Version, includes panic on parse error for simplicity in test setup
    fn v(s: &str) -> Version {
        Version::from_str(s).expect("Failed to parse test version string")
    }

    // Helper function to create Arguments struct for testing
    fn create_test_args(old: Option<&str>, new: &str, command: Option<Commands>) -> Arguments {
        let old_version = old.map(v); // Use v() helper
        let new_version = v(new); // Use v() helper
        Arguments {
            downloader: Downloader::Curl, // Use a default value
            suffix: "ClaudioFSR".to_string(),
            old: old_version,
            new: new_version,
            command,
        }
    }

    // Expected Config for a typical valid case (default command, new > old)
    // Note: We use v() helper here too
    fn expected_config_valid(old: Option<&str>, new: &str, command: Option<Commands>) -> Config {
        let args = create_test_args(old, new, command); // Create corresponding args
        let version_old_val = args.old;
        let version_new_val = args.new.clone();

        let custom_kernel_suffix = "ClaudioFSR".to_string();
        let kernel_url_base = "https://cdn.kernel.org/pub/linux/kernel/v6.x".to_string();
        let kernel_src_base = PathBuf::from("/lib/modules");
        let kernel_module_base = PathBuf::from("/lib/modules");
        let kernel_config_base = PathBuf::from("/lib/modules");

        let config_file_path = kernel_config_base.join(format!("config-{}", &custom_kernel_suffix));

        let kernel_src_dir_name = if args.new.patch == 0 {
            format!("linux-{}.{}", &args.new.major, &args.new.minor)
        } else {
            format!("linux-{}", &args.new)
        };

        let kernel_src_dir_path = kernel_src_base.join(&kernel_src_dir_name);

        let tarball_name = if args.new.patch == 0 {
            format!("linux-{}.{}.tar.xz", &args.new.major, &args.new.minor)
        } else {
            format!("linux-{}.tar.xz", &args.new)
        };

        let download_link = format!("{}/{}", &kernel_url_base, &tarball_name);

        let kernel_ident_name_new = if args.new.patch == 0 {
            format!(
                "{}.{}-{}",
                &args.new.major, &args.new.minor, &custom_kernel_suffix
            )
        } else {
            format!("{}-{}", &args.new, &custom_kernel_suffix)
        };

        let kernel_ident_name_old = version_old_val
            .as_ref()
            .map(|ver| format!("{}-{}", ver, &custom_kernel_suffix));
        let vmlinuz_install_path = PathBuf::from("/boot").join(format!(
            "vmlinuz-{}.{}",
            version_new_val.major, version_new_val.minor
        ));

        Config {
            version_old: version_old_val,
            version_new: version_new_val,
            command: args.command.clone(),
            kernel_url_base,
            kernel_src_base,
            kernel_module_base,
            kernel_config_base,
            custom_kernel_suffix,
            config_file_path,
            kernel_src_dir_name,
            kernel_src_dir_path,
            tarball_name,
            download_link,
            kernel_ident_name_new,
            kernel_ident_name_old,
            vmlinuz_install_path,
            downloader: args.downloader,
        }
    }

    #[test]
    fn test_config_new_default_valid() {
        let args = create_test_args(Some("6.15.3"), "6.15.4", None);
        let config =
            Config::new(args.clone()).expect("Config::new should succeed for valid default args");
        let expected = expected_config_valid(Some("6.15.3"), "6.15.4", None);
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_new_dkms_install_valid() {
        let args = create_test_args(Some("6.14.3"), "6.14.4", Some(Commands::DkmsInstall));
        let config = Config::new(args.clone())
            .expect("Config::new should succeed for valid dkms-install args");
        let expected = expected_config_valid(Some("6.14.3"), "6.14.4", Some(Commands::DkmsInstall));
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_new_kernel_compile_valid_no_old() {
        let args = create_test_args(None, "6.15.0", Some(Commands::KernelCompile));
        let config = Config::new(args.clone())
            .expect("Config::new should succeed for kernel-compile args without old");
        let expected = expected_config_valid(None, "6.15.0", Some(Commands::KernelCompile));
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_new_kernel_compile_valid_with_old() {
        // Providing a valid old version should not fail config creation for this command
        let args = create_test_args(Some("6.14.3"), "6.15.0", Some(Commands::KernelCompile));
        let config = Config::new(args.clone())
            .expect("Config::new should succeed for kernel-compile args with valid old");
        let expected =
            expected_config_valid(Some("6.14.3"), "6.15.0", Some(Commands::KernelCompile));
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_new_kernel_install_valid_no_old() {
        let args = create_test_args(None, "6.14.4", Some(Commands::KernelInstall));
        let config = Config::new(args.clone())
            .expect("Config::new should succeed for kernel-install args without old");
        let expected = expected_config_valid(None, "6.14.4", Some(Commands::KernelInstall));
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_new_kernel_install_valid_with_old() {
        // Providing a valid old version should not fail config creation for this command
        let args = create_test_args(Some("6.14.3"), "6.14.4", Some(Commands::KernelInstall));
        let config = Config::new(args.clone())
            .expect("Config::new should succeed for kernel-install args with valid old");
        let expected =
            expected_config_valid(Some("6.14.3"), "6.14.4", Some(Commands::KernelInstall));
        assert_eq!(config, expected);
    }

    #[test]
    fn test_config_downloader_set() {
        let mut args = create_test_args(None, "6.15.0", Some(Commands::KernelCompile));
        args.downloader = Downloader::Wget;
        let config = Config::new(args.clone()).expect("Config::new should handle downloader arg");
        assert_eq!(config.downloader, Downloader::Wget);

        let args_default = create_test_args(None, "6.15.0", Some(Commands::KernelCompile));
        let config_default = Config::new(args_default.clone())
            .expect("Config::new should handle default downloader");
        assert_eq!(config_default.downloader, Downloader::Curl); // Assuming Curl is Default in Args struct
    }

    // --- Validation Failure Tests (checking for specific KernelUpdaterError variants) ---

    #[test]
    fn test_config_new_default_missing_old_invalid() {
        let args = create_test_args(None, "6.14.4", None); // Missing --old, Default command
        let result = Config::new(args);
        assert!(result.is_err());
        // Check the *type* of the error variant and its specific fields
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::MissingRequiredArgument { argument_name, command } if argument_name == "--old" && command.is_none())
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_dkms_install_missing_old_invalid() {
        let args = create_test_args(None, "6.14.4", Some(Commands::DkmsInstall)); // Missing --old, DKMS command
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::MissingRequiredArgument { argument_name, command } if argument_name == "--old" && *command == Some(Commands::DkmsInstall))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_default_new_eq_old_invalid() {
        let old = "6.14.4";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, None); // new == old, Default command
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Check the *type* of the error variant and its specific fields
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_default_new_lt_old_invalid() {
        let old = "6.15.0";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, None); // new < old, Default command
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_dkms_install_new_eq_old_invalid() {
        let old = "6.14.4";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::DkmsInstall)); // new == old, DKMS command
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_dkms_install_new_lt_old_invalid() {
        let old = "6.15.0";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::DkmsInstall)); // new < old, DKMS command
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    // --- Tests for new <= old provided with commands that don't require --old (Should still fail VersionComparisonError) ---

    #[test]
    fn test_config_new_kernel_compile_new_eq_old_invalid() {
        let old = "6.14.4";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::KernelCompile)); // new == old, Kernel Compile
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_kernel_compile_new_lt_old_invalid() {
        let old = "6.15.0";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::KernelCompile)); // new < old, Kernel Compile
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_kernel_install_new_eq_old_invalid() {
        let old = "6.14.4";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::KernelInstall)); // new == old, Kernel Install
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }

    #[test]
    fn test_config_new_kernel_install_new_lt_old_invalid() {
        let old = "6.15.0";
        let new = "6.14.4";
        let args = create_test_args(Some(old), new, Some(Commands::KernelInstall)); // new < old, Kernel Install
        let result = Config::new(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, KernelUpdaterError::VersionComparisonError { old: err_old, new: err_new } if err_old == &v(old) && err_new == &v(new))
        );
        println!("Received expected error: {:?}", err);
    }
}
