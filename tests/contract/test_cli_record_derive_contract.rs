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

        assert!(execute_init(false, true, None).is_ok());

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

        assert!(execute_init(false, true, None).is_ok());

        let result = execute_record("nonexistent.csv", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file not found"));
    }

    #[test]
    fn test_record_empty_file_returns_ok() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, None).is_ok());

        let file_path = dir.path().join("empty.txt");
        fs::File::create(&file_path).unwrap();

        let path_str = file_path.to_string_lossy().to_string();
        assert!(execute_record(&path_str, false).is_ok());
    }

    #[test]
    fn test_record_stdin_returns_ok() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, None).is_ok());

        let file_path = dir.path().join("stdin_test.csv");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "value,42\n").unwrap();

        // The --stdin flag reads from actual stdin, but we pass it as a flag
        // We still need the file to exist on disk for metadata
        let path_str = file_path.to_string_lossy().to_string();
        assert!(execute_record(&path_str, true).is_ok());
    }
}
