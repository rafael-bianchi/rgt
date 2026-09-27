#[cfg(test)]
mod tests {
    use rgt::hooks::installer::{
        detect_and_configure_hooks_in_home, detect_and_configure_hooks_with_config,
        detect_and_configure_hooks_with_config_and_exe, AgentOutcome, InstallReport,
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
                AgentOutcome::Configured { agent, .. } | AgentOutcome::Migrated { agent, .. } => {
                    Some(agent.clone())
                }
                _ => None,
            })
            .collect()
    }

    fn skipped(report: &InstallReport) -> Vec<String> {
        report
            .outcomes
            .iter()
            .filter_map(|o| match o {
                AgentOutcome::AlreadyCurrent { agent, .. } => Some(agent.clone()),
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
        assert_eq!(post["matcher"].as_str().unwrap(), "Read|Edit|Write|Bash");
        let post_hooks = post["hooks"].as_array().unwrap();
        assert_eq!(post_hooks[0]["type"].as_str().unwrap(), "command");
        let post_cmd = post_hooks[0]["command"].as_str().unwrap();
        assert!(post_cmd.contains("hook post --agent claude-code --rgt-managed"));
        assert!(post_cmd.contains("sleep 0.8"));
        assert!(!post_cmd.starts_with("rgt "), "must not use bare rgt");

        let pre = &hooks["PreToolUse"][0];
        assert_eq!(pre["matcher"].as_str().unwrap(), "Read|Edit|Write|Bash");
        let pre_hooks = pre["hooks"].as_array().unwrap();
        let pre_cmd = pre_hooks[0]["command"].as_str().unwrap();
        assert!(pre_cmd.contains("hook pre --agent claude-code --rgt-managed"));
        assert!(pre_cmd.contains("sleep 0.8"));

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
        let pre_cmd = pre["command"].as_str().unwrap();
        assert!(pre_cmd.contains("hook pre --agent cursor --rgt-managed"));
        assert!(pre_cmd.contains("sleep 0.8"));
        assert_eq!(pre["matcher"].as_str().unwrap(), "Shell");

        let post = &hooks["hooks"]["postToolUse"][0];
        let post_cmd = post["command"].as_str().unwrap();
        assert!(post_cmd.contains("hook post --agent cursor --rgt-managed"));
        assert!(post_cmd.contains("sleep 0.8"));
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
        let hooks: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(".codex").join("hooks.json")).unwrap(),
        )
        .unwrap();
        let command = hooks["hooks"]["PostToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(command.contains("hook post --agent codex"));
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
        let hooks: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(
                dir.path()
                    .join(".codeium")
                    .join("windsurf")
                    .join("hooks.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(hooks["hooks"]["post_read_code"][0]["show_output"], false);
        assert!(hooks["hooks"]["post_read_code"][0]["command"]
            .as_str()
            .unwrap()
            .contains("hook post --agent windsurf"));
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

    #[test]
    fn hermes_inline_enablement_reinitializes_safely_in_both_scopes() {
        use rgt::hooks::registration::{inspect_local_registrations, RegistrationHealth};

        let _guard = CWD_MUTEX.lock().unwrap_or_else(|error| error.into_inner());
        for global in [false, true] {
            let dir = tempdir().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            let home = dir.path().join("home");
            let config_dir = dir.path().join("config");
            let executable = std::env::current_exe().unwrap();
            rgt::store::DbStore::open_in_project(dir.path()).unwrap();
            let config = if global {
                home.join(".hermes/config.toml")
            } else {
                dir.path().join(".hermes/config.toml")
            };
            let install = || {
                detect_and_configure_hooks_with_config_and_exe(
                    &home,
                    &config_dir,
                    global,
                    false,
                    Some("hermes"),
                    &executable,
                )
                .unwrap()
            };
            let health = || {
                inspect_local_registrations(&home, &config_dir)
                    .into_iter()
                    .find(|item| item.surface_id == "hermes")
                    .unwrap()
                    .health
            };
            let doctor = || {
                let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
                    .arg("doctor")
                    .current_dir(dir.path())
                    .env("HOME", &home)
                    .env("XDG_CONFIG_HOME", &config_dir)
                    .output()
                    .unwrap();
                (
                    output.status.code(),
                    String::from_utf8(output.stdout).unwrap(),
                )
            };

            assert_eq!(configured(&install()), vec!["hermes".to_string()]);
            let current =
                "# user comment\nplugins = { enabled = [\"other\", \"rgt\"], theme = \"dark\" }\n";
            std::fs::write(&config, current).unwrap();
            assert_eq!(health(), RegistrationHealth::Active);
            let (code, output) = doctor();
            assert_eq!(code, Some(0), "{output}");
            assert!(output.contains("hermes: Active registration"), "{output}");
            assert_eq!(skipped(&install()), vec!["hermes".to_string()]);
            assert_eq!(std::fs::read_to_string(&config).unwrap(), current);

            let missing = "# user comment\nplugins = { enabled = [\"other\"], theme = \"dark\" }\n";
            std::fs::write(&config, missing).unwrap();
            assert_eq!(health(), RegistrationHealth::Obsolete);
            assert_eq!(configured(&install()), vec!["hermes".to_string()]);
            assert_eq!(
                std::fs::read_to_string(format!("{}.rgt.bak", config.display())).unwrap(),
                missing
            );
            let updated = std::fs::read_to_string(&config).unwrap();
            assert!(updated.contains("# user comment"));
            let doc: toml_edit::DocumentMut = updated.parse().unwrap();
            let inline = doc["plugins"]
                .as_value()
                .unwrap()
                .as_inline_table()
                .unwrap();
            assert_eq!(
                inline.get("theme").and_then(toml_edit::Value::as_str),
                Some("dark")
            );
            assert_eq!(health(), RegistrationHealth::Active);
            assert_eq!(doctor().0, Some(0));
            assert_eq!(skipped(&install()), vec!["hermes".to_string()]);
            assert_eq!(std::fs::read_to_string(&config).unwrap(), updated);

            let malformed = "# user comment\nplugins = { enabled = [\"rgt\", 42] }\n";
            std::fs::write(&config, malformed).unwrap();
            assert_eq!(health(), RegistrationHealth::Malformed);
            let (code, output) = doctor();
            assert_eq!(code, Some(1), "{output}");
            assert!(
                output.contains("hermes: Malformed registration"),
                "{output}"
            );
            let report = install();
            let expected_artifact = if global {
                config.as_path()
            } else {
                Path::new(".hermes/config.toml")
            };
            assert!(
                report.outcomes.iter().any(|outcome| matches!(
                    outcome,
                    AgentOutcome::Conflict { agent, artifact, reason, .. }
                        if agent == "hermes"
                            && artifact.as_deref() == Some(expected_artifact)
                            && reason.contains("enabled")
                )),
                "{report:?}"
            );
            assert_eq!(std::fs::read_to_string(&config).unwrap(), malformed);

            let wrong_shape = "# user comment\nplugins = { enabled = \"rgt\" }\n";
            std::fs::write(&config, wrong_shape).unwrap();
            assert_eq!(health(), RegistrationHealth::Malformed);
            let report = install();
            assert!(
                report.outcomes.iter().any(|outcome| matches!(
                    outcome,
                    AgentOutcome::Conflict { agent, reason, .. }
                        if agent == "hermes" && reason.contains("enabled")
                )),
                "{report:?}"
            );
            assert_eq!(std::fs::read_to_string(&config).unwrap(), wrong_shape);
        }
    }

    #[test]
    fn pi_hermes_and_vibe_install_loadable_recording_guidance() {
        use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();

        let pi_path = Path::new(".pi/APPEND_SYSTEM.md");
        std::fs::create_dir_all(pi_path.parent().unwrap()).unwrap();
        std::fs::write(pi_path, "# Team system instructions\nKeep this.\n").unwrap();
        for agent in ["pi", "hermes", "vibe"] {
            detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                false,
                false,
                Some(agent),
                &executable,
            )
            .unwrap();
        }
        for (surface, path) in [
            ("pi", pi_path),
            ("hermes", Path::new("AGENTS.md")),
            ("vibe", Path::new("AGENTS.md")),
        ] {
            let inspection = inspect_local_guidance(&home, &config)
                .into_iter()
                .find(|item| item.surface_id == surface)
                .unwrap();
            assert_eq!(inspection.artifact_path, path, "{surface}");
            assert_eq!(inspection.health, GuidanceHealth::Usable, "{surface}");
        }
        assert!(std::fs::read_to_string(pi_path)
            .unwrap()
            .contains("Keep this."));
        assert!(!home.join(".vibe/prompts/rgt.md").exists());

        for agent in ["pi", "hermes", "vibe"] {
            let report = detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                false,
                false,
                Some(agent),
                &executable,
            )
            .unwrap();
            assert_eq!(skipped(&report), vec![agent.to_string()]);
        }
    }

    #[test]
    fn global_pi_and_vibe_guidance_uses_user_instruction_paths() {
        use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        let pi_override = home.join(".pi/agent/AGENTS.override.md");
        std::fs::create_dir_all(pi_override.parent().unwrap()).unwrap();
        std::fs::write(&pi_override, "# Personal instructions\nKeep this.\n").unwrap();
        for agent in ["pi", "vibe"] {
            detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                true,
                false,
                Some(agent),
                &executable,
            )
            .unwrap();
        }
        for (surface, path) in [("pi", pi_override), ("vibe", home.join(".vibe/AGENTS.md"))] {
            let inspection = inspect_local_guidance(&home, &config)
                .into_iter()
                .find(|item| item.surface_id == surface)
                .unwrap();
            assert_eq!(inspection.artifact_path, path, "{surface}");
            assert_eq!(inspection.health, GuidanceHealth::Usable, "{surface}");
        }
    }

    #[test]
    fn cursor_repairs_owned_rule_frontmatter_without_losing_user_content() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        let rule = Path::new(".cursor/rules/rgt.mdc");
        std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
        std::fs::write(
            rule,
            "---\ndescription: Team rule\nalwaysApply: false\n---\n# Keep this\n## RGT Integration\nOld `rgt record <file>` instructions.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();

        let report = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config,
            false,
            false,
            Some("cursor"),
            &executable,
        )
        .unwrap();
        assert_eq!(configured(&report), vec!["cursor".to_string()]);
        let text = std::fs::read_to_string(rule).unwrap();
        assert!(text.contains("alwaysApply: true"));
        assert!(text.contains("# Keep this"));
        assert!(text.contains("description: Team rule"));
        assert!(Path::new(".cursor/rules/rgt.mdc.rgt.bak").exists());

        let rerun = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config,
            false,
            false,
            Some("cursor"),
            &executable,
        )
        .unwrap();
        assert_eq!(skipped(&rerun), vec!["cursor".to_string()]);
        let guidance = rgt::hooks::registration::inspect_local_guidance(&home, &config);
        assert_eq!(
            guidance
                .iter()
                .find(|item| item.surface_id == "cursor")
                .unwrap()
                .health,
            rgt::hooks::registration::GuidanceHealth::Usable
        );
    }

    #[test]
    fn cursor_repairs_crlf_frontmatter_in_both_scopes_and_doctor_sees_guidance() {
        use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

        let _guard = CWD_MUTEX.lock().unwrap_or_else(|error| error.into_inner());
        for global in [false, true] {
            let dir = tempdir().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            let home = dir.path().join("home");
            let config = dir.path().join("config");
            let executable = std::env::current_exe().unwrap();
            let rule = Path::new(".cursor/rules/rgt.mdc");
            std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
            let original = "---\r\ndescription: Team rule\r\nalwaysApply: false\r\n---\r\n# Keep this\r\n## RGT Integration\r\nOld `rgt record <file>` instructions.\r\n<!-- /RGT Integration -->\r\n# Keep below\r\n";
            std::fs::write(rule, original).unwrap();

            let report = detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                global,
                false,
                Some("cursor"),
                &executable,
            )
            .unwrap();
            assert_eq!(configured(&report), vec!["cursor".to_string()]);
            let updated = std::fs::read_to_string(rule).unwrap();
            assert!(updated.contains("alwaysApply: true\r\n---\r\n"));
            assert!(updated.contains("# Keep this\r\n"));
            assert!(updated.ends_with("# Keep below\r\n"));
            assert_eq!(
                std::fs::read_to_string(".cursor/rules/rgt.mdc.rgt.bak").unwrap(),
                original
            );
            let guidance = inspect_local_guidance(&home, &config)
                .into_iter()
                .find(|item| item.surface_id == "cursor")
                .unwrap();
            assert_eq!(guidance.health, GuidanceHealth::Usable);
            assert_eq!(
                rgt::cli::guidance_diagnostic(&guidance).status,
                rgt::cli::DiagnosticStatus::Healthy
            );
            let rerun = detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                global,
                false,
                Some("cursor"),
                &executable,
            )
            .unwrap();
            assert_eq!(skipped(&rerun), vec!["cursor".to_string()]);
            assert_eq!(std::fs::read_to_string(rule).unwrap(), updated);
        }
    }

    #[test]
    fn cursor_repairs_only_the_always_apply_setting_in_lf_and_crlf_rules() {
        use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

        let _guard = CWD_MUTEX.lock().unwrap_or_else(|error| error.into_inner());
        for global in [false, true] {
            for newline in ["\n", "\r\n"] {
                let dir = tempdir().unwrap();
                std::env::set_current_dir(dir.path()).unwrap();
                let home = dir.path().join("home");
                let config = dir.path().join("config");
                let executable = std::env::current_exe().unwrap();
                let rule = Path::new(".cursor/rules/rgt.mdc");
                std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
                let original = format!(
                    "---{newline}description: Team uses alwaysApply: false in examples{newline}alwaysApply: false{newline}---{newline}# Keep this{newline}## RGT Integration{newline}Old `rgt record <file>` instructions.{newline}<!-- /RGT Integration -->{newline}"
                );
                std::fs::write(rule, &original).unwrap();

                let report = detect_and_configure_hooks_with_config_and_exe(
                    &home,
                    &config,
                    global,
                    false,
                    Some("cursor"),
                    &executable,
                )
                .unwrap();
                assert_eq!(configured(&report), vec!["cursor".to_string()]);
                let updated = std::fs::read_to_string(rule).unwrap();
                assert!(
                    updated.contains(&format!(
                        "description: Team uses alwaysApply: false in examples{newline}alwaysApply: true{newline}"
                    )),
                    "{updated}"
                );
                assert!(updated.contains(&format!("# Keep this{newline}")));
                assert_eq!(
                    std::fs::read_to_string(".cursor/rules/rgt.mdc.rgt.bak").unwrap(),
                    original
                );
                let guidance = inspect_local_guidance(&home, &config)
                    .into_iter()
                    .find(|item| item.surface_id == "cursor")
                    .unwrap();
                assert_eq!(guidance.health, GuidanceHealth::Usable);
                let rerun = detect_and_configure_hooks_with_config_and_exe(
                    &home,
                    &config,
                    global,
                    false,
                    Some("cursor"),
                    &executable,
                )
                .unwrap();
                assert_eq!(skipped(&rerun), vec!["cursor".to_string()]);
                assert_eq!(std::fs::read_to_string(rule).unwrap(), updated);
            }
        }
    }

    #[test]
    fn cursor_conflicting_always_apply_keys_are_not_reported_as_usable() {
        use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

        let _guard = CWD_MUTEX.lock().unwrap_or_else(|error| error.into_inner());
        for global in [false, true] {
            let dir = tempdir().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            let home = dir.path().join("home");
            let config = dir.path().join("config");
            let executable = std::env::current_exe().unwrap();
            let rule = Path::new(".cursor/rules/rgt.mdc");
            std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
            let original = "---\nalwaysApply: true\nalwaysApply : false\n---\n## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n";
            std::fs::write(rule, original).unwrap();
            rgt::store::DbStore::open_in_project(dir.path()).unwrap();
            let guidance = inspect_local_guidance(&home, &config)
                .into_iter()
                .find(|item| item.surface_id == "cursor")
                .unwrap();
            assert_eq!(guidance.health, GuidanceHealth::Incomplete);
            assert_eq!(
                rgt::cli::guidance_diagnostic(&guidance).status,
                rgt::cli::DiagnosticStatus::Error
            );
            let doctor = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
                .arg("doctor")
                .current_dir(dir.path())
                .env("HOME", &home)
                .env("XDG_CONFIG_HOME", &config)
                .output()
                .unwrap();
            assert_eq!(doctor.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&doctor.stdout).contains("cursor: Incomplete guidance"));

            let report = detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                global,
                false,
                Some("cursor"),
                &executable,
            )
            .unwrap();
            assert!(report.outcomes.iter().any(|outcome| matches!(
                outcome,
                AgentOutcome::Conflict { agent, .. } if agent == "cursor"
            )));
            assert_eq!(std::fs::read_to_string(rule).unwrap(), original);
            let hooks = if global {
                home.join(".cursor/hooks.json")
            } else {
                dir.path().join(".cursor/hooks.json")
            };
            assert!(!hooks.exists());
        }
    }

    #[test]
    fn cursor_global_init_reports_malformed_owned_rule_before_writing_hooks() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        let rule = Path::new(".cursor/rules/rgt.mdc");
        std::fs::create_dir_all(rule.parent().unwrap()).unwrap();
        let original = "## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n";
        std::fs::write(rule, original).unwrap();

        let report = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config,
            true,
            false,
            Some("cursor"),
            &executable,
        )
        .unwrap();
        assert!(report.outcomes.iter().any(|outcome| matches!(
            outcome,
            AgentOutcome::Conflict { agent, .. } if agent == "cursor"
        )));
        assert_eq!(std::fs::read_to_string(rule).unwrap(), original);
        assert!(!home.join(".cursor/hooks.json").exists());
    }

    #[test]
    fn hermes_guidance_uses_active_context_file_and_preserves_other_rules() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        std::fs::write(".hermes.md", "# Hermes team instructions\nKeep this.\n").unwrap();
        std::fs::write("AGENTS.md", "# Shared instructions\nKeep shared.\n").unwrap();
        detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config,
            true,
            false,
            Some("hermes"),
            &executable,
        )
        .unwrap();
        let inspection = rgt::hooks::registration::inspect_local_guidance(&home, &config)
            .into_iter()
            .find(|item| item.surface_id == "hermes")
            .unwrap();
        assert_eq!(inspection.artifact_path, Path::new(".hermes.md"));
        assert_eq!(
            inspection.health,
            rgt::hooks::registration::GuidanceHealth::Usable
        );
        assert!(std::fs::read_to_string(".hermes.md")
            .unwrap()
            .contains("Keep this."));
        assert_eq!(
            std::fs::read_to_string("AGENTS.md").unwrap(),
            "# Shared instructions\nKeep shared.\n"
        );
    }

    #[test]
    fn test_codex_normal_init_migrates_legacy_hook_with_backup_and_no_duplicates() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config_dir = dir.path().join("config");
        let hooks = home.join(".codex/hooks.json");
        std::fs::create_dir_all(hooks.parent().unwrap()).unwrap();
        let legacy = r#"{
  "hooks": {"PostToolUse": [{"matcher":".*","hooks":[{"type":"command","command":"/old/rgt hook post --agent codex"}]}]},
  "mcp_servers": {"keep": true}
}"#;
        std::fs::write(&hooks, legacy).unwrap();

        let first = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            true,
            false,
            Some("codex"),
            &std::env::current_exe().unwrap(),
        )
        .unwrap();
        assert_eq!(configured(&first), vec!["codex".to_string()]);
        assert!(
            matches!(first.outcomes.as_slice(), [AgentOutcome::Migrated { backups, .. }] if backups == &vec![hooks.with_extension("json.rgt.bak")])
        );
        assert_eq!(
            std::fs::read_to_string(hooks.with_extension("json.rgt.bak")).unwrap(),
            legacy
        );
        let migrated = std::fs::read_to_string(&hooks).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&migrated).unwrap();
        assert_eq!(parsed["mcp_servers"]["keep"], true);
        assert_eq!(migrated.matches("--rgt-managed").count(), 1);

        let second = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            true,
            false,
            Some("codex"),
            &std::env::current_exe().unwrap(),
        )
        .unwrap();
        assert_eq!(skipped(&second), vec!["codex".to_string()]);
        assert_eq!(std::fs::read_to_string(&hooks).unwrap(), migrated);
    }

    #[test]
    fn test_global_init_detects_codex_claude_cursor_and_windsurf_together() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config_dir = dir.path().join("config");
        for directory in [".codex", ".claude", ".cursor", ".windsurf"] {
            std::fs::create_dir_all(home.join(directory)).unwrap();
        }

        let first = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            true,
            false,
            None,
            &std::env::current_exe().unwrap(),
        )
        .unwrap();
        for expected in ["claude-code", "cursor", "codex", "windsurf"] {
            assert!(
                configured(&first).contains(&expected.to_string()),
                "{expected}"
            );
        }
        assert!(home.join(".codex/hooks.json").exists());
        assert!(home.join(".claude/settings.json").exists());
        assert!(home.join(".cursor/hooks.json").exists());
        assert!(home.join(".codeium/windsurf/hooks.json").exists());

        let second = detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            true,
            false,
            None,
            &std::env::current_exe().unwrap(),
        )
        .unwrap();
        for expected in ["claude-code", "cursor", "codex", "windsurf"] {
            assert!(
                skipped(&second).contains(&expected.to_string()),
                "{expected}"
            );
        }
    }
}
