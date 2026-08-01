#[cfg(test)]
mod tests {
    use rgt::hooks::installer::detect_and_configure_hooks;
    use serde_json::Value;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    fn read_json(path: &std::path::Path) -> Value {
        let content = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    #[test]
    fn test_claude_code_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, Some("claude-code")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Claude Code"));

        let hooks = read_json(&dir.path().join(".claude").join("hooks.json"));
        assert!(hooks.get("PostToolUse").is_some());
        assert!(hooks.get("PreToolUse").is_some());
        assert_eq!(
            hooks["PostToolUse"]["command"].as_str().unwrap(),
            "rgt hook post"
        );
    }

    #[test]
    fn test_cursor_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, Some("cursor")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Cursor"));

        let hooks = read_json(&dir.path().join(".cursor").join("hooks.json"));
        let arr = hooks["hooks"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["event"].as_str().unwrap(), "PostToolUse");
        assert_eq!(arr[1]["event"].as_str().unwrap(), "PreToolUse");
    }

    #[test]
    fn test_codex_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, Some("codex")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Codex CLI"));

        let hooks = read_json(&dir.path().join(".codex").join("hooks.json"));
        assert_eq!(hooks["rgt_hook"].as_str().unwrap(), "rgt hook post");
    }

    #[test]
    fn test_windsurf_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, Some("windsurf")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Windsurf"));

        let hooks = read_json(
            &dir.path()
                .join(".codeium")
                .join("windsurf")
                .join("hooks.json"),
        );
        assert_eq!(
            hooks["hooks"]["post_execution"].as_str().unwrap(),
            "rgt hook post"
        );
    }

    #[test]
    fn test_all_agents_without_flag() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, None).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_no_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());

        // First run: creates hook config
        let result = detect_and_configure_hooks(true, true, Some("claude-code")).unwrap();
        assert_eq!(result.len(), 1);

        // Second run without force: should not overwrite
        let result = detect_and_configure_hooks(true, false, Some("claude-code")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_unknown_agent_returns_empty() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        std::env::set_var("HOME", dir.path().to_str().unwrap());
        let result = detect_and_configure_hooks(true, true, Some("nonexistent")).unwrap();
        assert!(result.is_empty());
    }
}
