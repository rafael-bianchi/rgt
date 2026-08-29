//! Semantic-version ordering for the update path.
//!
//! The self-update decision must use SemVer 2.0 precedence, not string
//! comparison: `/releases/latest` is time-sorted (GitHub's own docs), so a
//! republished older tag is string-"greater" but semver-smaller. Comparing via
//! [`semver::Version`] `Ord` prevents silent downgrades (findings §7.1).

use semver::Version;

/// Parses a version string (leading `v` tolerated) into a [`Version`].
///
/// # Errors
/// Returns a user-recoverable message when the input is not valid SemVer
/// (e.g. `v1.x`).
pub fn parse_version(raw: &str) -> Result<Version, String> {
    let trimmed = raw.trim().trim_start_matches('v');
    Version::parse(trimmed).map_err(|e| format!("invalid semantic version `{}`: {}", raw.trim(), e))
}

/// Whether `latest` is strictly newer than `current`, by SemVer precedence.
///
/// This is the single gate for auto-update: an equal or older tag is "up to
/// date" and must never be installed by default (FR-004/FR-006).
///
/// # Errors
/// Returns a user-recoverable message when either version fails to parse.
pub fn latest_is_newer(latest: &str, current: &str) -> Result<bool, String> {
    let latest = parse_version(latest)?;
    let current = parse_version(current)?;
    Ok(latest > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genuine_newer_version_is_newer() {
        assert!(latest_is_newer("1.10.1", "1.10.0").unwrap());
        assert!(latest_is_newer("v1.2.3", "1.2.2").unwrap());
    }

    #[test]
    fn string_greater_but_semver_smaller_is_not_newer() {
        // The §7.1 downgrade case: "1.9.9" is string-greater than "1.10.0"
        // but semver-smaller.
        assert!(!latest_is_newer("1.9.9", "1.10.0").unwrap());
    }

    #[test]
    fn equal_versions_are_not_newer() {
        assert!(!latest_is_newer("1.10.0", "1.10.0").unwrap());
    }

    #[test]
    fn pre_release_orders_below_release() {
        assert!(!latest_is_newer("1.0.0-alpha.1", "1.0.0").unwrap());
        assert!(latest_is_newer("1.0.0", "1.0.0-alpha.1").unwrap());
    }

    #[test]
    fn invalid_versions_error() {
        assert!(latest_is_newer("v1.x", "1.0.0").is_err());
        assert!(parse_version("not-a-version").is_err());
    }
}
