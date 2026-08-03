use crate::{KernelUpdaterError, KernelUpdaterResult};
use std::{fs, io, path::Path};

/// An extension trait to perform filesystem-safe, atomic file operations.
///
/// This trait provides helper functions to write or copy files by first executing
/// operations on a temporary file within the target parent directory, then performing
/// an atomic POSIX-compliant rename swap. This guarantees that targets are never
/// left in a partially written or corrupted state during a crash or interruption.
pub trait AtomicWriteExt {
    /// Writes content to a temporary file via a closure, then atomically renames
    /// it to the destination path.
    ///
    /// The temporary file is guaranteed to be created in the same directory as
    /// the target path, satisfying POSIX requirements for atomic renames.
    ///
    /// # Errors
    ///
    /// Returns a [`KernelUpdaterError::IOError`] if directory creation, the write closure,
    /// or the final rename swap fails.
    fn atomic_write<F>(&self, write_fn: F) -> KernelUpdaterResult<()>
    where
        F: FnOnce(&Path) -> KernelUpdaterResult<()>;

    /// Safely copies a file from a source location to this path atomically.
    ///
    /// This method copies files to a temporary location adjacent to the target destination path
    /// before performing an atomic rename. This is suitable for replacing external commands
    /// like `cp` with a safer, native Rust alternative.
    ///
    /// # Errors
    ///
    /// Returns a [`KernelUpdaterError::IOError`] if parent directory creation, the underlying copy,
    /// or the final rename swap fails.
    fn atomic_copy_from(&self, source: &Path) -> KernelUpdaterResult<()>;
}

impl AtomicWriteExt for Path {
    fn atomic_write<F>(&self, write_fn: F) -> KernelUpdaterResult<()>
    where
        F: FnOnce(&Path) -> KernelUpdaterResult<()>,
    {
        // 1. Generate a temp file name in the same parent directory
        let mut temp_name = self
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file path"))?
            .to_os_string();

        temp_name.push(format!(".tmp-{}", std::process::id()));

        let temp_path = self.with_file_name(temp_name);

        // 2. Execute the user-provided write closure on the temporary path
        let write_res = write_fn(&temp_path);

        if let Err(err) = write_res {
            // Attempt clean up of the temporary file if writing failed
            let _ = fs::remove_file(&temp_path);
            return Err(err);
        }

        // 3. Perform the atomic rename swap
        fs::rename(&temp_path, self).map_err(|io_error| {
            // Clean up the temporary file if the atomic swap fails
            let _ = fs::remove_file(&temp_path);
            KernelUpdaterError::IOError {
                path: self.to_path_buf(),
                io_error,
            }
        })?;

        Ok(())
    }

    fn atomic_copy_from(&self, source: &Path) -> KernelUpdaterResult<()> {
        self.atomic_write(|temp_path| {
            fs::copy(source, temp_path).map_err(|io_error| KernelUpdaterError::IOError {
                path: temp_path.to_path_buf(),
                io_error,
            })?;
            Ok(())
        })
    }
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

// cargo test -- --help
// cargo test -- --nocapture
// cargo test -- --show-output

/// Run tests with:
/// cargo test -- --show-output tests_traits
#[cfg(test)]
mod tests_traits {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    // Creates a unique file path within the system temp directory
    fn create_temp_path(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        path.push(format!(
            "kernel_updater_test_{}_{}_{}",
            prefix,
            std::process::id(),
            count
        ));
        path
    }

    // RAII helper to clean up test files after a test completes
    struct TempFileCleaner {
        path: PathBuf,
    }

    impl Drop for TempFileCleaner {
        fn drop(&mut self) {
            if self.path.exists() {
                let _ = fs::remove_file(&self.path);
            }
        }
    }

    #[test]
    fn test_atomic_write_success() {
        let target_path = create_temp_path("write_success");
        let _cleaner = TempFileCleaner {
            path: target_path.clone(),
        };

        let write_res = target_path.atomic_write(|temp_path| {
            fs::write(temp_path, b"atomic write content").map_err(|e| {
                KernelUpdaterError::IOError {
                    path: temp_path.to_path_buf(),
                    io_error: e,
                }
            })?;
            Ok(())
        });

        assert!(write_res.is_ok());
        assert!(target_path.exists());

        let content = fs::read_to_string(&target_path).expect("Failed to read file");
        assert_eq!(content, "atomic write content");
    }

    #[test]
    fn test_atomic_write_failure_cleanup() {
        let target_path = create_temp_path("write_failure");
        let _cleaner = TempFileCleaner {
            path: target_path.clone(),
        };

        let write_res = target_path.atomic_write(|_temp_path| {
            Err(KernelUpdaterError::IoError(io::Error::other(
                "Simulated write failure",
            )))
        });

        assert!(write_res.is_err());
        assert!(!target_path.exists());

        // Check that no leftover temporary files remain in the temp directory
        if let Some(parent) = target_path.parent()
            && let Ok(entries) = fs::read_dir(parent)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("kernel_updater_test_write_failure")
                    && name.contains(".tmp-")
                {
                    panic!("Leftover temporary file found: {:?}", path);
                }
            }
        }
    }

    #[test]
    fn test_atomic_write_no_overwrite_on_failure() {
        let target_path = create_temp_path("write_no_overwrite");
        let _cleaner = TempFileCleaner {
            path: target_path.clone(),
        };

        // Write initial content
        fs::write(&target_path, b"initial content").expect("Failed to write initial content");

        let write_res = target_path.atomic_write(|temp_path| {
            fs::write(temp_path, b"new content that should fail").map_err(|e| {
                KernelUpdaterError::IOError {
                    path: temp_path.to_path_buf(),
                    io_error: e,
                }
            })?;

            // Return failure after writing to the temp path
            Err(KernelUpdaterError::IoError(io::Error::other(
                "Simulated failure after write",
            )))
        });

        assert!(write_res.is_err());
        assert!(target_path.exists());

        // Verify original content was untouched
        let content = fs::read_to_string(&target_path).expect("Failed to read file");
        assert_eq!(content, "initial content");
    }

    #[test]
    fn test_atomic_copy_from_success() {
        let source_path = create_temp_path("copy_source");
        let dest_path = create_temp_path("copy_dest");
        let _cleaner_source = TempFileCleaner {
            path: source_path.clone(),
        };
        let _cleaner_dest = TempFileCleaner {
            path: dest_path.clone(),
        };

        fs::write(&source_path, b"copy content").expect("Failed to write source content");

        let copy_res = dest_path.atomic_copy_from(&source_path);

        assert!(copy_res.is_ok());
        assert!(dest_path.exists());
        assert!(source_path.exists());

        let content = fs::read_to_string(&dest_path).expect("Failed to read destination file");
        assert_eq!(content, "copy content");
    }

    #[test]
    fn test_atomic_copy_from_source_not_found() {
        let source_path = create_temp_path("non_existent_source");
        let dest_path = create_temp_path("copy_dest_fail");
        let _cleaner_dest = TempFileCleaner {
            path: dest_path.clone(),
        };

        let copy_res = dest_path.atomic_copy_from(&source_path);

        assert!(copy_res.is_err());
        assert!(!dest_path.exists());
    }
}
