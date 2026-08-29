use rgt::updater::version::{latest_is_newer, parse_version};

#[test]
fn genuine_newer_version_is_newer() {
    assert!(latest_is_newer("1.10.1", "1.10.0").unwrap());
    assert!(latest_is_newer("v1.2.3", "1.2.2").unwrap());
}

#[test]
fn string_greater_but_semver_smaller_is_not_newer() {
    // The §7.1 downgrade case: "1.9.9" is string-greater than "1.10.0" but
    // semver-smaller — must never be treated as an update.
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

#[test]
fn leading_v_is_tolerated() {
    assert_eq!(
        parse_version("v1.2.3").unwrap(),
        parse_version("1.2.3").unwrap()
    );
}
