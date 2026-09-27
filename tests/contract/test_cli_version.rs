use std::process::Command;

fn expected_version() -> String {
    format!("rgt {}", env!("CARGO_PKG_VERSION"))
}

#[test]
fn global_version_flag_reports_the_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected_version()
    );
}

#[test]
fn version_subcommand_reports_the_same_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .arg("version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        expected_version()
    );
}
