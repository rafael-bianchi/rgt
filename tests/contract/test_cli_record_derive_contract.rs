#[cfg(test)]
mod tests {
    use rgt::cli::{execute_init, execute_record};
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    #[test]
    fn test_record_valid_file_returns_ok() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, Some("codex")).is_ok());

        let file_path = dir.path().join("data.csv");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "revenue,120000\ncosts,60000\nprofit,60000\n").unwrap();

        let path_str = file_path.to_string_lossy().to_string();
        assert!(execute_record(&path_str, false).is_ok());
    }

    #[test]
    fn test_record_file_not_found_returns_err() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, Some("codex")).is_ok());

        let result = execute_record("nonexistent.csv", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file not found"));
    }

    #[test]
    fn test_record_empty_file_returns_ok() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, Some("codex")).is_ok());

        let file_path = dir.path().join("empty.txt");
        fs::File::create(&file_path).unwrap();

        let path_str = file_path.to_string_lossy().to_string();
        assert!(execute_record(&path_str, false).is_ok());
    }

    #[test]
    fn test_record_stdin_returns_ok() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, Some("codex")).is_ok());

        let file_path = dir.path().join("stdin_test.csv");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "value,42\n").unwrap();

        // The --stdin flag reads from actual stdin, but we pass it as a flag
        // We still need the file to exist on disk for metadata
        let path_str = file_path.to_string_lossy().to_string();
        assert!(execute_record(&path_str, true).is_ok());
    }

    // -----------------------------------------------------------------------
    // 021 / US2 (T008): an unknown --operation is invalid input (exit 2),
    // never a silent pass-through, and records nothing.
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_unknown_operation_exits_2_and_records_nothing() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([
                "derive",
                "--parents",
                "x",
                "--operation",
                "bogus",
                "--result",
                "1",
            ])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt derive");

        assert_eq!(
            out.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("valid operations: EXPRESSION, DATE_DIFF"),
            "{}",
            stderr
        );
        assert!(!stderr.contains("skipping"), "no silent skip: {}", stderr);
        assert!(
            !dir.path().join(".rgt/store.db").exists(),
            "no derivation may be recorded for an invalid operation"
        );
    }

    #[test]
    fn test_derive_case_mismatched_operation_is_exit_2() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([
                "derive",
                "--parents",
                "x",
                "--operation",
                "expression",
                "--expression",
                "a + b",
                "--result",
                "1",
            ])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt derive");

        assert_eq!(
            out.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // -----------------------------------------------------------------------
    // 021 / convergence T018: missing --expression and >26 parents are exit 2
    // (invalid input) at the CLI layer.
    // -----------------------------------------------------------------------

    fn twenty_seven_parents() -> String {
        (0..27)
            .map(|i| format!("id{}", i))
            .collect::<Vec<_>>()
            .join(",")
    }

    #[test]
    fn test_verify_more_than_26_parents_exits_2() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([
                "verify",
                "--parents",
                &twenty_seven_parents(),
                "--operation",
                "EXPRESSION",
                "--expression",
                "a",
                "--result",
                "1",
            ])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt verify");

        assert_eq!(
            out.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("at most 26 parents"), "{}", stderr);
    }

    #[test]
    fn test_derive_more_than_26_parents_exits_2() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([
                "derive",
                "--parents",
                &twenty_seven_parents(),
                "--operation",
                "EXPRESSION",
                "--expression",
                "a",
                "--result",
                "1",
            ])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt derive");

        assert_eq!(
            out.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn test_verify_missing_expression_exits_2() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let out = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([
                "verify",
                "--parents",
                "x",
                "--operation",
                "EXPRESSION",
                "--result",
                "1",
            ])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt verify");

        assert_eq!(
            out.status.code(),
            Some(2),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("Missing --expression"), "{}", stderr);
    }
}
