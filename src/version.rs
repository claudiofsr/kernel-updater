use crate::error::KernelUpdaterError;
use std::{fmt, str::FromStr};

/// Represents a parsed semantic kernel version (Major.Minor.Patch).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Creates a new `Version` instance.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Checks if this is a major release or has a patch level of zero (e.g., 6.15.0).
    pub const fn is_major_point_release(&self) -> bool {
        self.patch == 0
    }

    /// Returns the major and minor versions as a string (e.g., "6.15").
    pub fn major_minor(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl FromStr for Version {
    type Err = KernelUpdaterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(4, '.');

        let major_str =
            parts
                .next()
                .ok_or_else(|| KernelUpdaterError::VersionParseFormatError {
                    input: s.to_string(),
                })?;
        let minor_str =
            parts
                .next()
                .ok_or_else(|| KernelUpdaterError::VersionParseFormatError {
                    input: s.to_string(),
                })?;
        let patch_str =
            parts
                .next()
                .ok_or_else(|| KernelUpdaterError::VersionParseFormatError {
                    input: s.to_string(),
                })?;

        if parts.next().is_some() {
            return Err(KernelUpdaterError::VersionParseFormatError {
                input: s.to_string(),
            });
        }

        let major = major_str.trim().parse::<u32>()?;
        let minor = minor_str.trim().parse::<u32>()?;
        let patch = patch_str.trim().parse::<u32>()?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

// cargo test -- --help
// cargo test -- --nocapture
// cargo test -- --show-output

/// Run tests with:
/// cargo test -- --show-output tests_version
#[cfg(test)]
mod tests_version {
    use super::*;

    #[test]
    fn test_version_new() {
        let v = Version::new(6, 15, 4);
        assert_eq!(v.major, 6);
        assert_eq!(v.minor, 15);
        assert_eq!(v.patch, 4);
    }

    #[test]
    fn test_is_major_point_release() {
        assert!(Version::new(6, 15, 0).is_major_point_release());
        assert!(!Version::new(6, 15, 4).is_major_point_release());
    }

    #[test]
    fn test_major_minor() {
        let v = Version::new(6, 15, 4);
        assert_eq!(v.major_minor(), "6.15");
    }

    #[test]
    fn test_from_str_valid() {
        let parsed = Version::from_str("6.15.4").unwrap();
        assert_eq!(parsed, Version::new(6, 15, 4));

        let parsed_with_spaces = Version::from_str("  10 . 20 . 30  ").unwrap();
        assert_eq!(parsed_with_spaces, Version::new(10, 20, 30));
    }

    #[test]
    fn test_from_str_invalid_format() {
        assert!(matches!(
            Version::from_str("6.15"),
            Err(KernelUpdaterError::VersionParseFormatError { .. })
        ));

        assert!(matches!(
            Version::from_str("6.15.4.1"),
            Err(KernelUpdaterError::VersionParseFormatError { .. })
        ));
    }

    #[test]
    fn test_from_str_invalid_int() {
        assert!(matches!(
            Version::from_str("6.15.a"),
            Err(KernelUpdaterError::VersionParseIntError { .. })
        ));
    }

    #[test]
    fn test_display() {
        let v = Version::new(6, 15, 4);
        assert_eq!(v.to_string(), "6.15.4");
    }

    #[test]
    fn test_ordering() {
        let v1 = Version::new(6, 14, 4);
        let v2 = Version::new(6, 15, 3);
        let v3 = Version::new(6, 15, 4);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 > v1);
        assert_eq!(v3, Version::new(6, 15, 4));
    }
}
