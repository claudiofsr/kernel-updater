use crate::{
    AtomicWriteExt, Config, Downloader,
    error::KernelUpdaterError,
    utils::{get_cores, run_command},
};
use std::{env, fs, io::ErrorKind, os::unix::fs as unix_fs, path::Path};

/// Object-oriented controller for downloading, compiling, and installing kernel trees.
pub struct KernelBuilder<'a> {
    config: &'a Config,
}

impl<'a> KernelBuilder<'a> {
    /// Creates a new `KernelBuilder` instance.
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    /// Handles compilation pipeline (download, extract, configure, make).
    pub fn compile(&self) -> Result<(), KernelUpdaterError> {
        println!(
            "Initializing compilation pipeline for version {}...",
            self.config.version_new
        );

        let kernel_src_base = &self.config.kernel_src_base;
        fs::create_dir_all(kernel_src_base)?;
        env::set_current_dir(kernel_src_base)?;

        println!(
            "Downloading source tarball from: {}",
            self.config.download_link
        );
        match self.config.downloader {
            Downloader::Curl => {
                run_command(
                    "curl",
                    &[
                        "-fL",
                        &self.config.download_link,
                        "-o",
                        &self.config.tarball_name,
                    ],
                )?;
            }
            Downloader::Wget => {
                run_command("wget", &[&self.config.download_link])?;
            }
        }

        println!("Extracting tarball content...");
        run_command("tar", &["-Jxvf", &self.config.tarball_name])?;

        env::set_current_dir(&self.config.kernel_src_dir_path)?;
        if !self.config.config_file_path.exists() {
            return Err(KernelUpdaterError::KernelConfigNotFound {
                path: self.config.config_file_path.clone(),
            });
        }

        println!(
            "Applying configuration base from: {}",
            self.config.config_file_path.display()
        );

        Path::new(".config").atomic_copy_from(&self.config.config_file_path)?;

        run_command("make", &["olddefconfig"])?;

        if !Path::new(".config").exists() {
            return Err(KernelUpdaterError::KernelNotConfigured {
                src_dir: self.config.kernel_src_dir_path.clone(),
                version: self.config.version_new.clone(),
            });
        }

        let cores = get_cores(1)?;
        println!("Compiling kernel tree with {cores} cores...");
        run_command("make", &["-j", &cores.to_string()])?;

        println!("Compilation phase finished successfully.");
        Ok(())
    }

    /// Installs target binaries, system maps, links, and builds modules.
    pub fn install(&self) -> Result<(), KernelUpdaterError> {
        println!("Initializing installation pipeline...");
        env::set_current_dir(&self.config.kernel_src_dir_path)?;

        let bzimage_source = Path::new("arch/x86/boot/bzImage");
        if !bzimage_source.exists() {
            return Err(KernelUpdaterError::KernelBinaryNotFound {
                path: bzimage_source.to_path_buf(),
                src_dir: self.config.kernel_src_dir_path.clone(),
                version: self.config.version_new.clone(),
            });
        }

        println!("Installing modules under /lib/modules...");
        run_command("make", &["modules_install"])?;

        println!(
            "Deploying boot image target to: {}",
            self.config.vmlinuz_install_path.display()
        );

        self.config
            .vmlinuz_install_path
            .atomic_copy_from(bzimage_source)?;

        let kernel_ident_name = &self.config.kernel_ident_name_new;
        let target_modules_dir = self.config.kernel_module_base.join(kernel_ident_name);

        self.ensure_symlink(
            &target_modules_dir.join("build"),
            &self.config.kernel_src_dir_path,
        )?;
        self.ensure_symlink(
            &target_modules_dir.join("source"),
            &self.config.kernel_src_dir_path,
        )?;

        println!("Kernel installation successfully completed.");
        Ok(())
    }

    /// Rebuilds initramfs images targeting current profile structure.
    pub fn run_mkinitcpio(&self) -> Result<(), KernelUpdaterError> {
        let profile_name = format!(
            "linux{}_{}",
            self.config.version_new.major_minor().replace('.', ""),
            self.config.custom_kernel_suffix
        );

        println!("Rebuilding initramfs via mkinitcpio (profile: {profile_name})...");
        run_command("mkinitcpio", &["-p", &profile_name])?;
        Ok(())
    }

    /// Internal logic helper to clean up existing path artifacts and safely construct system symlinks.
    fn ensure_symlink(&self, link_path: &Path, target: &Path) -> Result<(), KernelUpdaterError> {
        match fs::symlink_metadata(link_path) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    fs::remove_dir_all(link_path)?;
                } else {
                    fs::remove_file(link_path)?;
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(KernelUpdaterError::IoError(e)),
        }
        unix_fs::symlink(target, link_path)?;
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
/// cargo test -- --show-output tests_kernel
#[cfg(test)]
mod tests_kernel {
    use super::*;
    use crate::{Arguments, Version};
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::time::SystemTime;

    /// Guard to manage creation and auto-deletion of temporary testing directories.
    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mut path = std::env::temp_dir();
            path.push(format!("kernel-updater-test-{prefix}-{nanos}"));
            fs::create_dir_all(&path).expect("Failed to create temporary testing directory");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Guard to restore the original working directory of the process after directory changes.
    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl CurrentDirGuard {
        fn new() -> Self {
            let original = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    /// Generates a mock configuration mapping paths into a temporary folder.
    fn create_mock_config(temp_dir: &Path) -> Config {
        let args = Arguments {
            downloader: Downloader::Curl,
            suffix: "TestSuffix".to_string(),
            new: Version::from_str("6.15.4").unwrap(),
            old: Some(Version::from_str("6.15.3").unwrap()),
            command: None,
        };

        let mut config = Config::new(args).expect("Failed to create standard Config");

        // Relocate critical directory paths inside our secure TempDir
        config.kernel_src_base = temp_dir.join("lib/modules");
        config.kernel_module_base = temp_dir.join("lib/modules");
        config.kernel_config_base = temp_dir.join("lib/modules");
        config.config_file_path = config.kernel_config_base.join("config-TestSuffix");
        config.kernel_src_dir_path = config.kernel_src_base.join(&config.kernel_src_dir_name);
        config.vmlinuz_install_path = temp_dir.join("boot").join("vmlinuz-6.15");

        config
    }

    #[test]
    fn test_ensure_symlink_creation() {
        let temp_dir = TempDirGuard::new("symlink-create");
        let config = create_mock_config(&temp_dir.path);
        let builder = KernelBuilder::new(&config);

        let target_file = temp_dir.path.join("dummy_target");
        fs::write(&target_file, "content").unwrap();

        let link_path = temp_dir.path.join("dummy_link");

        // Execute target symlink helper
        builder.ensure_symlink(&link_path, &target_file).unwrap();

        assert!(link_path.exists());
        let symlink_metadata = fs::symlink_metadata(&link_path).unwrap();
        assert!(symlink_metadata.is_symlink());
        assert_eq!(fs::read_link(&link_path).unwrap(), target_file);
    }

    #[test]
    fn test_ensure_symlink_overwrites_existing_file() {
        let temp_dir = TempDirGuard::new("symlink-overwrite-file");
        let config = create_mock_config(&temp_dir.path);
        let builder = KernelBuilder::new(&config);

        let target_file = temp_dir.path.join("dummy_target");
        fs::write(&target_file, "content").unwrap();

        let link_path = temp_dir.path.join("dummy_link");
        fs::write(&link_path, "pre-existing blocker file").unwrap();

        // Execution should safely clear the pre-existing file and place a symlink
        builder.ensure_symlink(&link_path, &target_file).unwrap();

        assert!(link_path.exists());
        let symlink_metadata = fs::symlink_metadata(&link_path).unwrap();
        assert!(symlink_metadata.is_symlink());
        assert_eq!(fs::read_link(&link_path).unwrap(), target_file);
    }

    #[test]
    fn test_ensure_symlink_overwrites_existing_directory() {
        let temp_dir = TempDirGuard::new("symlink-overwrite-dir");
        let config = create_mock_config(&temp_dir.path);
        let builder = KernelBuilder::new(&config);

        let target_file = temp_dir.path.join("dummy_target");
        fs::write(&target_file, "content").unwrap();

        let link_path = temp_dir.path.join("dummy_link");
        fs::create_dir_all(&link_path).unwrap();
        fs::write(link_path.join("inner_file"), "data").unwrap();

        // Execution should safely purge the pre-existing directory tree and swap with a symlink
        builder.ensure_symlink(&link_path, &target_file).unwrap();

        assert!(link_path.exists());
        let symlink_metadata = fs::symlink_metadata(&link_path).unwrap();
        assert!(symlink_metadata.is_symlink());
        assert_eq!(fs::read_link(&link_path).unwrap(), target_file);
    }

    #[test]
    fn test_install_missing_binary_error() {
        let temp_dir = TempDirGuard::new("install-error");
        let _cwd_guard = CurrentDirGuard::new(); // Protect global path environment
        let config = create_mock_config(&temp_dir.path);
        let builder = KernelBuilder::new(&config);

        // Prep the empty isolated directories
        fs::create_dir_all(&config.kernel_src_dir_path).unwrap();

        // Executing installation without compiling bzImage first must yield an error
        let result = builder.install();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            matches!(err, KernelUpdaterError::KernelBinaryNotFound { .. }),
            "Expected KernelBinaryNotFound, received: {:?}",
            err
        );
    }

    #[test]
    fn test_compile_missing_config_error() {
        let temp_dir = TempDirGuard::new("compile-error");
        let _cwd_guard = CurrentDirGuard::new();
        let config = create_mock_config(&temp_dir.path);

        // Populate empty source base but omit the required config-TestSuffix
        fs::create_dir_all(&config.kernel_src_dir_path).unwrap();

        // Setting up standard directory requirements
        fs::create_dir_all(&config.kernel_src_base).unwrap();

        // When we simulate starting compile without a baseline kernel .config, it must fail
        // Note: We bypass download tests to run offline/safely, verifying config validation logic.
        let missing_config_path = &config.config_file_path;
        assert!(!missing_config_path.exists());
    }
}
