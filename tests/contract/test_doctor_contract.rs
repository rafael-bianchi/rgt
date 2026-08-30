#[cfg(test)]
mod tests {
    use rgt::cli::run_doctor;
    use rgt::cli::DiagnosticStatus;
    use rgt::store::DbStore;
    use std::sync::Mutex;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    fn store_exists() -> bool {
        std::path::Path::new(".rgt").join("store.db").exists()
    }

    fn open_store() -> Result<DbStore, String> {
        DbStore::open_in_project(".").map_err(|e| e.to_string())
    }

    /// A fresh temp dir with an initialized `.rgt/store.db` and one recorded node.
    fn populated_store_dir() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
        let dir = tempfile::tempdir().unwrap();
        let guard = set_cwd(dir.path());
        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let p = dir.path().join("d.csv");
        std::fs::write(&p, "amount,42\n").unwrap();
        rgt::cli::execute_record(&p.to_string_lossy(), false, None).unwrap();
        (dir, guard)
    }

    fn store_dir() -> (tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
        let dir = tempfile::tempdir().unwrap();
        let guard = set_cwd(dir.path());
        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        (dir, guard)
    }

    #[test]
    fn not_initialized_is_error_exit_1() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let (diags, code) = run_doctor(|| false, open_store, || false);
        assert_eq!(code, 1);
        assert!(diags.iter().any(|d| d.status == DiagnosticStatus::Error));
    }

    #[test]
    fn initialized_with_values_is_healthy_exit_0() {
        let (_dir, _guard) = populated_store_dir();
        let (diags, code) = run_doctor(store_exists, open_store, || true);
        assert_eq!(code, 0);
        assert!(
            !diags.iter().any(|d| d.status == DiagnosticStatus::Error),
            "no errors: {:?}",
            diags
        );
    }

    #[test]
    fn initialized_empty_graph_with_hooks_is_warning_exit_0() {
        let (_dir, _guard) = store_dir();
        let (diags, code) = run_doctor(store_exists, open_store, || true);
        assert_eq!(code, 0);
        let captures = diags
            .iter()
            .find(|d| d.name == "captures")
            .expect("captures diagnostic present");
        assert_eq!(captures.status, DiagnosticStatus::Warning);
        assert!(captures.message.contains("PATH"));
    }

    #[test]
    fn initialized_without_hooks_is_warning_exit_0() {
        let (_dir, _guard) = store_dir();
        let (diags, code) = run_doctor(store_exists, open_store, || false);
        assert_eq!(code, 0);
        assert!(
            diags
                .iter()
                .any(|d| d.name == "hooks" && d.status == DiagnosticStatus::Warning),
            "no hooks warning: {:?}",
            diags
        );
    }
}
