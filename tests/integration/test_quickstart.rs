#[cfg(test)]
mod tests {
    use rgt::cli::{execute_init, execute_status};
    use tempfile::tempdir;

    #[test]
    fn test_quickstart_e2e_init_and_status() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        assert!(execute_init(false, true).is_ok());
        assert!(dir.path().join(".rgt/store.db").exists());

        assert!(execute_status(false, true).is_ok());
    }
}
