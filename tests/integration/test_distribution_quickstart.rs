#[path = "update_stub.rs"]
mod update_stub;

#[cfg(test)]
mod tests {
    use super::update_stub;
    use rgt::cli::update::execute_update_with_io;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

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
        let output = Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args(["update", "--help"])
            .output()
            .expect("Failed to inspect rgt update help");
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("--check"));

        let io = update_stub::CheckOnlyUpdateIo::new(format!("v{}", env!("CARGO_PKG_VERSION")));
        execute_update_with_io(true, true, None, false, false, &io).unwrap();
        assert_eq!(io.fetches(), 1);
    }

    #[test]
    fn test_quickstart_cargo_package_clean() {
        let mut child = Command::new("cargo")
            .args([
                "package",
                "--allow-dirty",
                "--offline",
                "--locked",
                "--no-verify",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to run cargo package");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = child.try_wait().expect("Failed to wait for cargo package") {
                assert!(status.success(), "offline cargo package failed: {status}");
                break;
            }
            if Instant::now() >= deadline {
                child
                    .kill()
                    .expect("Failed to stop timed-out cargo package");
                child
                    .wait()
                    .expect("Failed to reap timed-out cargo package");
                panic!("offline cargo package exceeded 30 seconds");
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}
