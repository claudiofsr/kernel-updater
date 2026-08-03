use crate::{
    Config,
    error::KernelUpdaterError,
    utils::{run_command, run_command_output},
};

/// Representation of a parsed DKMS module status entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DkmsEntry {
    pub module_name: String,
    pub module_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub status: String,
}

/// Object-oriented manager for executing actions over multiple DKMS modules.
pub struct DkmsManager<'a> {
    config: &'a Config,
    target_modules: Vec<&'static str>,
}

impl<'a> DkmsManager<'a> {
    /// Instantiates a `DkmsManager` with default target modules.
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            target_modules: vec!["nvidia", "v4l2loopback"],
        }
    }

    /// Queries the operational system via `dkms status` and parses the response.
    pub fn get_installed_modules(&self) -> Result<Vec<DkmsEntry>, KernelUpdaterError> {
        println!("Querying current DKMS module statuses...");
        let dkms_output = run_command_output("dkms", &["status"])?;
        Ok(Self::parse_status_output(&dkms_output))
    }

    /// Parses the raw string output of the `dkms status` command.
    fn parse_status_output(output: &str) -> Vec<DkmsEntry> {
        output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                // Expected format e.g.:
                // nvidia/610.43.03, 7.1.4-1-MANJARO, x86_64: installed
                let (left_side, status) = line.split_once(':')?;
                let status = status.trim().to_string();

                let mut comma_parts = left_side.split(',');
                let mod_ver_part = comma_parts.next()?.trim();

                let (module_name, module_version) = mod_ver_part.split_once('/')?;
                let module_name = module_name.trim().to_string();
                let module_version = module_version.trim().to_string();

                // Safety check: a valid module version should not contain further directories or slashes
                if module_version.contains('/') {
                    return None;
                }

                let kernel_version = comma_parts
                    .next()
                    .map_or(String::new(), |s| s.trim().to_string());
                let architecture = comma_parts
                    .next()
                    .map_or(String::new(), |s| s.trim().to_string());

                Some(DkmsEntry {
                    module_name,
                    module_version,
                    kernel_version,
                    architecture,
                    status,
                })
            })
            .collect()
    }

    /// Checks if a module with the specified name is currently registered/installed in the system.
    pub fn is_module_installed(&self, name: &str) -> Result<bool, KernelUpdaterError> {
        let registered = self.get_installed_modules()?;
        Ok(registered.iter().any(|entry| entry.module_name == name))
    }

    /// Searches active version for a target module in the parsed registry.
    fn find_installed_version(&self, name: &str, registered: &[DkmsEntry]) -> Option<String> {
        registered
            .iter()
            .find(|entry| entry.module_name == name)
            .map(|entry| entry.module_version.clone())
    }

    /// Builds and installs target modules for the newly compiled kernel.
    pub fn install_modules(&self) -> Result<(), KernelUpdaterError> {
        let registered = self.get_installed_modules()?;
        let kernel_name_new = &self.config.kernel_ident_name_new;

        for target in &self.target_modules {
            if let Some(version) = self.find_installed_version(target, &registered) {
                println!(
                    "Installing DKMS module '{target}' version '{version}' for kernel {kernel_name_new}..."
                );

                let module_spec = format!("{target}/{version}");
                let install_args = ["install", "--force", &module_spec, "-k", kernel_name_new];

                run_command("dkms", &install_args)?;
                println!("DKMS module '{target}' installed successfully for {kernel_name_new}.\n");
            } else {
                println!("Warning: Module '{target}' is not registered on system. Skipping build.");
            }
        }
        Ok(())
    }

    /// Safely uninstalls target modules from the older kernel version.
    pub fn remove_modules(&self) -> Result<(), KernelUpdaterError> {
        let kernel_name_old = match &self.config.kernel_ident_name_old {
            Some(name) => name,
            None => return Ok(()),
        };

        let registered = self.get_installed_modules()?;

        for target in &self.target_modules {
            if let Some(version) = self.find_installed_version(target, &registered) {
                println!(
                    "Uninstalling DKMS module '{target}' version '{version}' from old kernel {kernel_name_old}..."
                );

                let module_spec = format!("{target}/{version}");
                let remove_args = ["remove", &module_spec, "-k", kernel_name_old];

                if let Err(e) = run_command("dkms", &remove_args) {
                    eprintln!(
                        "Warning: Failed to clean up '{target}' for old kernel {kernel_name_old}: {e}"
                    );
                } else {
                    println!("Successfully removed '{target}' from old kernel registry.");
                }

                let leftover_var_dir =
                    format!("/var/lib/dkms/{target}/{version}/{kernel_name_old}");
                let _ = std::fs::remove_dir_all(leftover_var_dir);
            }
        }
        Ok(())
    }
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

// cargo test -- --help
// cargo test -- --nocapture
// cargo test -- --show-output

/// Run tests with:
/// cargo test -- --show-output tests_dkms
#[cfg(test)]
mod tests_dkms {
    use super::*;
    use crate::Arguments;

    // Helper to generate a basic config for the manager
    fn get_stub_config() -> Config {
        let args = Arguments {
            downloader: crate::args::Downloader::Curl,
            suffix: "TestSuffix".to_string(),
            new: crate::Version::new(6, 15, 4),
            old: Some(crate::Version::new(6, 15, 3)),
            command: None,
        };
        Config::new(args).expect("Failed to initialize test config")
    }

    #[test]
    fn test_parse_status_output_valid() {
        let raw_output = r#"
            nvidia/610.43.03, 7.1.4-1-MANJARO, x86_64: installed
            v4l2loopback/0.15.4, 7.1.4-1-MANJARO, x86_64: installed
        "#;

        let parsed = DkmsManager::parse_status_output(raw_output);

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].module_name, "nvidia");
        assert_eq!(parsed[0].module_version, "610.43.03");
        assert_eq!(parsed[0].kernel_version, "7.1.4-1-MANJARO");
        assert_eq!(parsed[0].architecture, "x86_64");
        assert_eq!(parsed[0].status, "installed");

        assert_eq!(parsed[1].module_name, "v4l2loopback");
        assert_eq!(parsed[1].module_version, "0.15.4");
        assert_eq!(parsed[1].kernel_version, "7.1.4-1-MANJARO");
        assert_eq!(parsed[1].architecture, "x86_64");
        assert_eq!(parsed[1].status, "installed");
    }

    #[test]
    fn test_parse_status_output_malformed_lines() {
        let raw_output = r#"
            some-random-broken-line-without-colon
            nvidia/610.43.03, 7.1.4-1-MANJARO, x86_64: installed
            v4l2loopback: installed
            another/broken/split/output: status
        "#;

        let parsed = DkmsManager::parse_status_output(raw_output);

        // Only the valid line should be extracted
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].module_name, "nvidia");
        assert_eq!(parsed[0].module_version, "610.43.03");
    }

    #[test]
    fn test_find_installed_version_found() {
        let config = get_stub_config();
        let manager = DkmsManager::new(&config);

        let registered = vec![
            DkmsEntry {
                module_name: "nvidia".to_string(),
                module_version: "550.40.01".to_string(),
                kernel_version: "6.12.0".to_string(),
                architecture: "x86_64".to_string(),
                status: "installed".to_string(),
            },
            DkmsEntry {
                module_name: "v4l2loopback".to_string(),
                module_version: "0.12.7".to_string(),
                kernel_version: "6.12.0".to_string(),
                architecture: "x86_64".to_string(),
                status: "installed".to_string(),
            },
        ];

        let version = manager.find_installed_version("nvidia", &registered);
        assert_eq!(version, Some("550.40.01".to_string()));
    }

    #[test]
    fn test_find_installed_version_not_found() {
        let config = get_stub_config();
        let manager = DkmsManager::new(&config);

        let registered = vec![DkmsEntry {
            module_name: "v4l2loopback".to_string(),
            module_version: "0.12.7".to_string(),
            kernel_version: "6.12.0".to_string(),
            architecture: "x86_64".to_string(),
            status: "installed".to_string(),
        }];

        let version = manager.find_installed_version("nvidia", &registered);
        assert_eq!(version, None);
    }
}
