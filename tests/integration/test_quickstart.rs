#[cfg(test)]
mod tests {
    use rgt::cli::{execute_init, execute_status};
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    #[test]
    fn test_quickstart_e2e_init_and_status() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        assert!(execute_init(false, true, Some("codex"), None).is_ok());
        assert!(dir.path().join(".rgt/store.db").exists());

        assert!(execute_status(false, true).is_ok());
    }
}
