#[path = "update_stub.rs"]
mod update_stub;

#[cfg(test)]
mod tests {
    use super::update_stub;
    use rgt::cli::update::execute_update_with_io;
    use std::process::Command;

    #[test]
    fn installer_dry_run_reports_the_selected_archive() {
        let output = Command::new("sh")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
            .args(["--dry-run"])
            .env("RGT_INSTALL_TEST_OS", "Darwin")
            .env("RGT_INSTALL_TEST_ARCH", "arm64")
            .output()
            .expect("Failed to run install.sh");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("aarch64-apple-darwin"));
    }

    #[test]
    fn update_check_reports_a_newer_injected_release_without_downloading() {
        let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let newer = format!("v{}.{}.0", current.major, current.minor + 1);
        let io = update_stub::CheckOnlyUpdateIo::new(newer);
        execute_update_with_io(true, true, None, false, false, &io).unwrap();
        assert_eq!(io.fetches(), 1);
    }
}
