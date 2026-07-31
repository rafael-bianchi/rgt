use std::process::Command;
use std::time::Instant;

const INSTALL_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/install.sh");

fn run_installer(args: &[&str], env_vars: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new("sh");
    cmd.arg(INSTALL_SCRIPT);
    for arg in args {
        cmd.arg(arg);
    }
    for (k, v) in env_vars {
        cmd.env(k, v);
    }
    cmd.output().expect("Failed to execute install.sh")
}

#[test]
fn test_help_flag_prints_usage() {
    let output = run_installer(&["-h"], &[]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RGT"));
    assert!(stdout.contains("--bin-dir"));
}

#[test]
fn test_dry_run_detects_os_and_arch() {
    let output = run_installer(
        &["--dry-run"],
        &[
            ("RGT_INSTALL_TEST_OS", "Darwin"),
            ("RGT_INSTALL_TEST_ARCH", "arm64"),
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("darwin"));
    assert!(stdout.contains("aarch64"));
    assert!(stdout.contains("aarch64-apple-darwin"));
}

#[test]
fn test_dry_run_linux_x86_64() {
    let output = run_installer(
        &["--dry-run"],
        &[
            ("RGT_INSTALL_TEST_OS", "Linux"),
            ("RGT_INSTALL_TEST_ARCH", "x86_64"),
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("linux"));
    assert!(stdout.contains("x86_64"));
    assert!(stdout.contains("x86_64-unknown-linux-gnu"));
}

#[test]
fn test_unsupported_os_exits_1() {
    let output = run_installer(
        &["--dry-run"],
        &[
            ("RGT_INSTALL_TEST_OS", "FreeBSD"),
            ("RGT_INSTALL_TEST_ARCH", "x86_64"),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unsupported OS") || stderr.contains("cargo install"));
}

#[test]
fn test_unsupported_arch_exits_1() {
    let output = run_installer(
        &["--dry-run"],
        &[
            ("RGT_INSTALL_TEST_OS", "Darwin"),
            ("RGT_INSTALL_TEST_ARCH", "sparc64"),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_custom_bin_dir_flag_accepted() {
    let output = run_installer(
        &["--dry-run", "--bin-dir", "/custom/bin"],
        &[
            ("RGT_INSTALL_TEST_OS", "Darwin"),
            ("RGT_INSTALL_TEST_ARCH", "x86_64"),
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/custom/bin"));
}

#[test]
fn test_system_flag_accepted() {
    let output = run_installer(
        &["--dry-run", "--system"],
        &[
            ("RGT_INSTALL_TEST_OS", "Linux"),
            ("RGT_INSTALL_TEST_ARCH", "x86_64"),
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/usr/local/bin"));
}

#[test]
fn test_dry_run_completes_quickly() {
    let start = Instant::now();
    let output = run_installer(
        &["--dry-run"],
        &[
            ("RGT_INSTALL_TEST_OS", "Darwin"),
            ("RGT_INSTALL_TEST_ARCH", "x86_64"),
        ],
    );
    let elapsed = start.elapsed();
    assert!(output.status.success());
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "Dry-run took {:.1}s, expected <5s for SC-001 baseline",
        elapsed.as_secs_f64()
    );
}
