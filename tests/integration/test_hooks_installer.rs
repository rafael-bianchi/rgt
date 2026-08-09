#[cfg(test)]
mod tests {
    use rgt::hooks::installer::detect_and_configure_hooks_in_home;
    use serde_json::Value;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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

        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("claude-code"))
                .unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Claude Code"));
        assert!(result[0].contains("settings.json"));

        let settings = read_json(&dir.path().join(".claude").join("settings.json"));
        let hooks = &settings["hooks"];
        assert!(hooks.get("PostToolUse").is_some());
        assert!(hooks.get("PreToolUse").is_some());

        let post = &hooks["PostToolUse"][0];
        assert_eq!(post["matcher"].as_str().unwrap(), "");
        let post_hooks = post["hooks"].as_array().unwrap();
        assert_eq!(post_hooks[0]["type"].as_str().unwrap(), "command");
        assert_eq!(post_hooks[0]["command"].as_str().unwrap(), "rgt hook post");

        let pre = &hooks["PreToolUse"][0];
        assert_eq!(pre["matcher"].as_str().unwrap(), "");
        let pre_hooks = pre["hooks"].as_array().unwrap();
        assert_eq!(pre_hooks[0]["command"].as_str().unwrap(), "rgt hook pre");

        // Old hooks.json must NOT exist
        assert!(!dir.path().join(".claude").join("hooks.json").exists());
    }

    #[test]
    fn test_cursor_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("cursor")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Cursor"));

        let hooks = read_json(&dir.path().join(".cursor").join("hooks.json"));
        assert_eq!(hooks["version"].as_i64().unwrap(), 1);

        let pre = &hooks["hooks"]["preToolUse"][0];
        assert_eq!(pre["command"].as_str().unwrap(), "rgt hook pre");
        assert_eq!(pre["matcher"].as_str().unwrap(), "Shell");

        let post = &hooks["hooks"]["postToolUse"][0];
        assert_eq!(post["command"].as_str().unwrap(), "rgt hook post");
        assert_eq!(post["matcher"].as_str().unwrap(), "Shell");
    }

    #[test]
    fn test_codex_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("codex")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Codex CLI"));
        assert!(result[0].contains("rules-file"));

        let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(content.contains("## RGT Integration"));
        assert!(content.contains("rgt --help"));

        // No hooks.json with rgt_hook key
        let hooks_path = dir.path().join(".codex").join("hooks.json");
        assert!(
            !hooks_path.exists() || {
                let hooks = read_json(&hooks_path);
                hooks.get("rgt_hook").is_none()
            }
        );
    }

    #[test]
    fn test_windsurf_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("windsurf")).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Windsurf"));
        assert!(result[0].contains("rules-file"));

        let content = std::fs::read_to_string(dir.path().join(".windsurfrules")).unwrap();
        assert!(content.contains("RGT Integration"));
        assert!(content.contains("rgt --help"));
    }

    #[test]
    fn test_all_agents_without_flag() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let result = detect_and_configure_hooks_in_home(dir.path(), true, true, None).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_no_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        // First run: creates hook config
        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("claude-code"))
                .unwrap();
        assert_eq!(result.len(), 1);

        // Second run without force: should not overwrite
        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, false, Some("claude-code"))
                .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_unknown_agent_returns_empty() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let result =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("nonexistent"))
                .unwrap();
        assert!(result.is_empty());
    }
}
