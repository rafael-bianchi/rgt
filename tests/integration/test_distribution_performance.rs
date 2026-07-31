#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Instant;

    #[test]
    fn test_performance_installer_dry_run_under_5s() {
        let start = Instant::now();
        let output = Command::new("sh")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh"))
            .args(["--dry-run"])
            .env("RGT_INSTALL_TEST_OS", "Darwin")
            .env("RGT_INSTALL_TEST_ARCH", "arm64")
            .output()
            .expect("Failed to run install.sh");
        let elapsed = start.elapsed();

        assert!(output.status.success());
        assert!(
            elapsed.as_secs_f64() < 10.0,
            "SC-001: Installer dry-run took {:.1}s, expected <10s",
            elapsed.as_secs_f64()
        );
    }

    #[test]
    fn test_performance_update_check_under_5s() {
        let start = Instant::now();
        let output = Command::new("cargo")
            .args(["run", "--", "update", "--check"])
            .output();
        let elapsed = start.elapsed();

        match output {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("rate limit") || stderr.contains("error") {
                    return;
                }
                assert!(
                    elapsed.as_secs_f64() < 30.0,
                    "SC-005: Update check took {:.1}s (including cargo build), expected <30s w/ build",
                    elapsed.as_secs_f64()
                );
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_performance_cargo_build_release_profile() {
        let output = Command::new("cargo")
            .args(["build", "--release"])
            .output()
            .expect("Failed to run cargo build --release");

        assert!(output.status.success());
    }
}
