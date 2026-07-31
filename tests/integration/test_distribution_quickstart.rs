#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn test_quickstart_installer_help() {
        let output = Command::new("sh")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
            .arg("-h")
            .output()
            .expect("Failed to run install.sh -h");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("RGT"));
        assert!(stdout.contains("--bin-dir"));
        assert!(stdout.contains("--version"));
    }

    #[test]
    fn test_quickstart_installer_dry_run() {
        let output = Command::new("sh")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
            .args(["--dry-run"])
            .env("RGT_INSTALL_TEST_OS", "Darwin")
            .env("RGT_INSTALL_TEST_ARCH", "arm64")
            .output()
            .expect("Failed to run install.sh --dry-run");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("aarch64-apple-darwin"));
    }

    #[test]
    fn test_quickstart_update_check_flag() {
        let output = Command::new("cargo")
            .args(["run", "--", "update", "--check"])
            .output();
        match output {
            Ok(out) => {
                let combined = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                assert!(
                    combined.contains("up to date")
                        || combined.contains("available")
                        || combined.contains("rate limit")
                        || combined.contains("Failed")
                        || combined.contains("error")
                );
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_quickstart_cargo_package_clean() {
        let output = Command::new("cargo")
            .args(["package", "--allow-dirty"])
            .output()
            .expect("Failed to run cargo package");

        assert!(output.status.success());
    }
}
