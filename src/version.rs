use crate::error::KernelUpdaterError;
use std::{fmt, num::ParseIntError, str::FromStr};

/// Represents a kernel version (Major.Minor.Patch).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)] // Added PartialOrd, Ord for comparison
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

// Implement FromStr trait to allow parsing a string into a Version struct.
// This is used by clap for command-line argument parsing.
impl FromStr for Version {
    type Err = KernelUpdaterError; // Use our specific error type

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components: Vec<u32> = s
            .split('.')
            .map(|part| part.trim().parse::<u32>())
            .collect::<Result<Vec<u32>, ParseIntError>>()?;

        // Check if exactly three components were parsed successfully.
        if components.len() == 3 {
            Ok(Version {
                major: components[0],
                minor: components[1],
                patch: components[2],
            })
        } else {
            // Return our specific error if the format is incorrect (wrong number of components)
            Err(KernelUpdaterError::VersionParseFormatError {
                input: s.to_string(),
            })
        }
    }
}

// Implement Display trait to allow formatting a Version struct as a string.
// This is used by macros like println! and format!.
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the major, minor, and patch components separated by dots.
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// Optional helper function to get a Version from a string.
// It's a simple wrapper around FromStr::from_str.
// #[allow(dead_code)] allows this function to exist even if not called directly.
#[allow(dead_code)]
pub fn get_version(version: &str) -> Result<Version, KernelUpdaterError> {
    Version::from_str(version)
}

//----------------------------------------------------------------------------//
//                                   Tests                                    //
//----------------------------------------------------------------------------//

/// Run tests with:
/// cargo test -- --show-output tests_version`
#[cfg(test)]
mod tests_version {
    use super::*;

    #[test]
    fn test_version_from_str_valid() {
        let version = "6.15.3";
        let parsed_version = Version::from_str(version).expect("Failed to parse valid version");
        assert_eq!(parsed_version.major, 6);
        assert_eq!(parsed_version.minor, 15);
        assert_eq!(parsed_version.patch, 3);

        let version_leading_trailing_spaces = "  6.15.3  ";
        let parsed_version_spaces = Version::from_str(version_leading_trailing_spaces)
            .expect("Failed to parse valid version with spaces");
        assert_eq!(parsed_version_spaces.major, 6);
        assert_eq!(parsed_version_spaces.minor, 15);
        assert_eq!(parsed_version_spaces.patch, 3);
    }

    #[test]
    fn test_version_display() {
        let version = Version {
            major: 6,
            minor: 15,
            patch: 3,
        };
        assert_eq!(format!("{}", version), "6.15.3");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::from_str("6.14.3").unwrap();
        let v2 = Version::from_str("6.14.4").unwrap();
        let v3 = Version::from_str("6.15.0").unwrap();
        let v4 = Version::from_str("7.0.0").unwrap();
        let v_same = Version::from_str("6.14.3").unwrap();

        assert!(v2 > v1);
        assert!(v3 > v2);
        assert!(v4 > v3);
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert_eq!(v1, v_same);
        assert!((v1 <= v_same));
        assert!((v1 >= v_same));
        assert!(v1 >= v_same);
        assert!(v1 <= v_same);
    }

    #[test]
    fn test_version_from_str_invalid() {
        // Test string with too few components (e.g., 6.15 splits to 2 components)
        let result_too_few = Version::from_str("6.15");
        assert!(result_too_few.is_err());
        let err_too_few = result_too_few.unwrap_err();
        // Check it's the correct error variant for wrong number of components
        if let KernelUpdaterError::VersionParseFormatError { .. } = err_too_few {
            // And check the display message content for this variant
            assert!(
                err_too_few
                    .to_string()
                    .contains("expected exactly three dot-separated numbers")
            );
        } else {
            panic!(
                "Wrong error type returned for 'too few' components: {:?}",
                err_too_few
            );
        }

        // Test string with too many components (e.g., 6.15.3.1 splits to 4 components)
        let result_too_many = Version::from_str("6.15.3.1");
        assert!(result_too_many.is_err());
        let err_too_many = result_too_many.unwrap_err();
        // Check it's the correct error variant for wrong number of components
        if let KernelUpdaterError::VersionParseFormatError { .. } = err_too_many {
            // And check the display message content for this variant
            assert!(
                err_too_many
                    .to_string()
                    .contains("expected exactly three dot-separated numbers")
            );
        } else {
            panic!(
                "Wrong error type returned for 'too many' components: {:?}",
                err_too_many
            );
        }

        // Test string with non-numeric component (e.g., "x" cannot parse to u32)
        let result_non_numeric = Version::from_str("6.x.3");
        assert!(result_non_numeric.is_err());
        let err_non_numeric = result_non_numeric.unwrap_err();
        // Check it's the correct error variant for failed component parsing (via ParseIntError)
        if let KernelUpdaterError::VersionParseIntError { source: _ } = err_non_numeric {
            // And check the display message content for this variant
            assert!(
                err_non_numeric
                    .to_string()
                    .contains("failed to parse as integer")
            );
        } else {
            panic!(
                "Wrong error type returned for 'non-numeric' component: {:?}",
                err_non_numeric
            );
        }

        // Test empty string (e.g., "" splits to [""] which cannot parse)
        let result_empty = Version::from_str("");
        assert!(result_empty.is_err());
        let err_empty = result_empty.unwrap_err();
        // "" splits to [""] which causes a ParseIntError -> VersionParseIntError
        if let KernelUpdaterError::VersionParseIntError { source: _ } = err_empty {
            assert!(err_empty.to_string().contains("failed to parse as integer"));
        } else {
            panic!(
                "Wrong error type returned for 'empty string': {:?}",
                err_empty
            );
        }

        // Test string with empty components (e.g., 6..3 splits to ["6", "", "3"])
        let result_empty_components_1 = Version::from_str("6..3");
        assert!(result_empty_components_1.is_err());
        let err_empty_components_1 = result_empty_components_1.unwrap_err();
        // "" splits to ["6", "", "3"] which causes a ParseIntError on "" -> VersionParseIntError
        if let KernelUpdaterError::VersionParseIntError { source: _ } = err_empty_components_1 {
            assert!(
                err_empty_components_1
                    .to_string()
                    .contains("failed to parse as integer")
            );
        } else {
            panic!(
                "Wrong error type returned for 'empty component (middle)': {:?}",
                err_empty_components_1
            );
        }

        // Test string with empty components (e.g., 6.15. splits to ["6", "15", ""])
        let result_empty_components_2 = Version::from_str("6.15.");
        assert!(result_empty_components_2.is_err());
        let err_empty_components_2 = result_empty_components_2.unwrap_err();
        // "6.15." splits to ["6", "15", ""] which causes a ParseIntError on "" -> VersionParseIntError
        if let KernelUpdaterError::VersionParseIntError { source: _ } = err_empty_components_2 {
            assert!(
                err_empty_components_2
                    .to_string()
                    .contains("failed to parse as integer")
            );
        } else {
            panic!(
                "Wrong error type returned for 'empty component (trailing)': {:?}",
                err_empty_components_2
            );
        }
    }
}
