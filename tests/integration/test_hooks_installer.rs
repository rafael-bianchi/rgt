#[cfg(test)]
mod tests {
    use rgt::hooks::installer::{
        detect_and_configure_hooks_in_home, detect_and_configure_hooks_with_config, AgentOutcome,
        InstallReport,
    };
    use serde_json::Value;
    use std::path::Path;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    fn read_json(path: &Path) -> Value {
        let content = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    fn configured(report: &InstallReport) -> Vec<String> {
        report
            .outcomes
            .iter()
            .filter_map(|o| match o {
                AgentOutcome::Configured { agent, .. } => Some(agent.clone()),
                _ => None,
            })
            .collect()
    }

    fn skipped(report: &InstallReport) -> Vec<String> {
        report
            .outcomes
            .iter()
            .filter_map(|o| match o {
                AgentOutcome::Skipped { agent } => Some(agent.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_claude_code_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("claude-code"))
                .unwrap();
        assert_eq!(configured(&report), vec!["claude-code".to_string()]);

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

        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("cursor")).unwrap();
        assert_eq!(configured(&report), vec!["cursor".to_string()]);

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

        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("codex")).unwrap();
        assert_eq!(configured(&report), vec!["codex".to_string()]);

        let content = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(content.contains("## RGT Integration"));
        assert!(content.contains("rgt --help"));
        assert!(content.contains("<!-- /RGT Integration -->"));
    }

    #[test]
    fn test_windsurf_hook_config() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("windsurf")).unwrap();
        assert_eq!(configured(&report), vec!["windsurf".to_string()]);

        let content = std::fs::read_to_string(dir.path().join(".windsurfrules")).unwrap();
        assert!(content.contains("RGT Integration"));
        assert!(content.contains("rgt --help"));
        assert!(content.contains("<!-- /RGT Integration -->"));
    }

    #[test]
    fn test_all_agents_without_flag_detects_installed() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        // Emulate the 4 existing agents as installed via their detection triggers.
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::create_dir_all(dir.path().join(".cursor")).unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "# Test\n").unwrap();
        std::fs::write(dir.path().join(".windsurfrules"), "# Test\n").unwrap();
        let config_dir = dir.path().join("config");

        let report =
            detect_and_configure_hooks_with_config(dir.path(), &config_dir, true, true, None)
                .unwrap();
        let c = configured(&report);
        assert_eq!(c.len(), 4);
        for agent in ["claude-code", "cursor", "codex", "windsurf"] {
            assert!(c.contains(&agent.to_string()), "missing {}", agent);
        }
    }

    #[test]
    fn test_no_agents_installed_configures_nothing() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let config_dir = dir.path().join("config");

        let report =
            detect_and_configure_hooks_with_config(dir.path(), &config_dir, true, true, None)
                .unwrap();
        assert!(report.is_empty());
    }

    #[test]
    fn test_no_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        // First run: creates hook config
        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("claude-code"))
                .unwrap();
        assert_eq!(configured(&report), vec!["claude-code".to_string()]);

        // Second run without force: skipped, nothing written
        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, false, Some("claude-code"))
                .unwrap();
        assert_eq!(skipped(&report), vec!["claude-code".to_string()]);
    }

    #[test]
    fn test_unknown_agent_returns_empty() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let report =
            detect_and_configure_hooks_in_home(dir.path(), true, true, Some("nonexistent"))
                .unwrap();
        assert!(report.is_empty());
    }

    // T021(b): --force on a rules file preserves content below the RGT block.
    #[test]
    fn test_force_preserves_content_below_rgt_block() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let config_dir = dir.path().join("config");

        let rules = dir.path().join(".clinerules");
        std::fs::write(
            &rules,
            "# My project rules\n## RGT Integration\nOLD\n<!-- /RGT Integration -->\n# My personal notes\nkeep me\n",
        )
        .unwrap();

        let report = detect_and_configure_hooks_with_config(
            dir.path(),
            &config_dir,
            false,
            true,
            Some("cline"),
        )
        .unwrap();
        assert_eq!(configured(&report), vec!["cline".to_string()]);

        let content = std::fs::read_to_string(&rules).unwrap();
        assert!(content.contains("# My project rules\n"));
        assert!(!content.contains("OLD"));
        assert!(
            content.contains("# My personal notes\nkeep me\n"),
            "notes below block preserved"
        );
        assert_eq!(content.matches("## RGT Integration").count(), 1);
    }

    // T021(c): Codex AGENTS.md user content preserved on --force.
    #[test]
    fn test_codex_force_preserves_agenda_md_user_content() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let config_dir = dir.path().join("config");

        let agents_md = dir.path().join("AGENTS.md");
        std::fs::write(
            &agents_md,
            "# Project instructions\nUse strict types.\n## RGT Integration\nOLD RGT BLOCK\n<!-- /RGT Integration -->\n# Team notes\nkeep\n",
        )
        .unwrap();

        let report = detect_and_configure_hooks_with_config(
            dir.path(),
            &config_dir,
            true,
            true,
            Some("codex"),
        )
        .unwrap();
        assert_eq!(configured(&report), vec!["codex".to_string()]);

        let content = std::fs::read_to_string(&agents_md).unwrap();
        assert!(content.contains("# Project instructions\nUse strict types.\n"));
        assert!(!content.contains("OLD RGT BLOCK"));
        assert!(
            content.contains("# Team notes\nkeep\n"),
            "user content below RGT block preserved"
        );
        assert!(content.contains("rgt record <file>"));
    }

    // T021(d): Windsurf .windsurfrules user content preserved on --force.
    #[test]
    fn test_windsurf_force_preserves_user_rules() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let config_dir = dir.path().join("config");

        let rules = dir.path().join(".windsurfrules");
        std::fs::write(
            &rules,
            "# RGT Integration\nOLD\n<!-- /RGT Integration -->\n# Personal windsurf rules\nkeep\n",
        )
        .unwrap();

        let report = detect_and_configure_hooks_with_config(
            dir.path(),
            &config_dir,
            true,
            true,
            Some("windsurf"),
        )
        .unwrap();
        assert_eq!(configured(&report), vec!["windsurf".to_string()]);

        let content = std::fs::read_to_string(&rules).unwrap();
        assert!(!content.contains("OLD"));
        assert!(
            content.contains("# Personal windsurf rules\nkeep\n"),
            "user rules preserved"
        );
        assert!(content.contains("rgt record <file>"));
    }

    // T021(e): Hermes config.toml stays valid TOML after re-init.
    #[test]
    fn test_hermes_toml_stays_valid_after_reinit() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let config_dir = dir.path().join("config");

        let config = dir.path().join(".hermes").join("config.toml");
        std::fs::create_dir_all(config.parent().unwrap()).unwrap();
        std::fs::write(&config, "# keep\n[plugins]\nenabled = [ \"git\", ]\n").unwrap();

        let report = detect_and_configure_hooks_with_config(
            dir.path(),
            &config_dir,
            true,
            false,
            Some("hermes"),
        )
        .unwrap();
        assert_eq!(configured(&report), vec!["hermes".to_string()]);

        let content = std::fs::read_to_string(&config).unwrap();
        assert!(content.contains("# keep"));
        let doc: toml_edit::DocumentMut = content.parse().expect("must remain valid TOML");
        let enabled = doc["plugins"]["enabled"].as_array().unwrap();
        let vals: Vec<&str> = enabled.iter().filter_map(|v| v.as_str()).collect();
        assert!(vals.contains(&"rgt"), "rgt enabled: {:?}", vals);
        assert!(vals.contains(&"git"));

        // Re-run: idempotent.
        let report2 = detect_and_configure_hooks_with_config(
            dir.path(),
            &config_dir,
            true,
            false,
            Some("hermes"),
        )
        .unwrap();
        assert_eq!(skipped(&report2), vec!["hermes".to_string()]);
    }
}
