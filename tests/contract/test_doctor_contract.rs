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
    fn initialized_empty_graph_with_hooks_reports_only_local_health() {
        let (_dir, _guard) = store_dir();
        let (diags, code) = run_doctor(store_exists, open_store, || true);
        assert_eq!(code, 0);
        let hooks = diags
            .iter()
            .find(|d| d.name == "hooks")
            .expect("hooks diagnostic present");
        assert_eq!(hooks.status, DiagnosticStatus::Healthy);
        assert!(hooks.message.contains("does not verify"));
        assert!(!diags.iter().any(|d| d.name == "captures"));
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

    #[test]
    fn local_surface_inspection_distinguishes_registration_from_graph_values() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        let executable = std::env::current_exe().unwrap();
        let command = format!(
            "\"{}\" hook post --agent codex --rgt-managed",
            executable.display()
        );
        let codex_hooks = serde_json::json!({
            "hooks": {"PostToolUse": [{"hooks": [{"type": "command", "command": command}]}]}
        });
        std::fs::write(
            home.join(".codex/hooks.json"),
            serde_json::to_vec(&codex_hooks).unwrap(),
        )
        .unwrap();
        std::fs::write(
            ".clinerules",
            rgt::hooks::glue::with_instruction("## RGT Integration\nUse `rgt record <file>`.\n"),
        )
        .unwrap();
        let registrations = rgt::hooks::registration::inspect_local_registrations(&home, &config);
        assert_eq!(registrations.len(), 15);
        let codex = registrations
            .iter()
            .find(|r| r.surface_id == "codex")
            .unwrap();
        assert_eq!(
            codex.health,
            rgt::hooks::registration::RegistrationHealth::Active
        );
        assert_eq!(
            codex.capture_tier,
            rgt::hooks::registration::CaptureTier::ConfiguredUnverified
        );
        let cline = registrations
            .iter()
            .find(|r| r.surface_id == "cline")
            .unwrap();
        assert_eq!(
            cline.health,
            rgt::hooks::registration::RegistrationHealth::GuidanceOnly
        );
        let roo = registrations
            .iter()
            .find(|r| r.surface_id == "roo-code")
            .unwrap();
        assert_eq!(
            roo.health,
            rgt::hooks::registration::RegistrationHealth::GuidanceOnly
        );
        assert!(!registrations.iter().any(|r| {
            r.capture_tier == rgt::hooks::registration::CaptureTier::VerifiedAutomatic
        }));
    }

    #[test]
    fn malformed_registration_is_reported_without_claiming_capture() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        std::fs::create_dir_all(".codex").unwrap();
        std::fs::write(".codex/hooks.json", "{broken").unwrap();
        let registrations = rgt::hooks::registration::inspect_local_registrations(
            &dir.path().join("home"),
            &dir.path().join("config"),
        );
        let codex = registrations
            .iter()
            .find(|r| r.surface_id == "codex")
            .unwrap();
        assert_eq!(
            codex.health,
            rgt::hooks::registration::RegistrationHealth::Malformed
        );
        assert_ne!(
            codex.capture_tier,
            rgt::hooks::registration::CaptureTier::VerifiedAutomatic
        );
    }

    #[test]
    fn codex_health_distinguishes_bare_malformed_obsolete_duplicate_and_unresolvable_entries() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let hooks = dir.path().join(".codex/hooks.json");
        std::fs::create_dir_all(hooks.parent().unwrap()).unwrap();

        let status = || {
            rgt::hooks::registration::inspect_local_registrations(&home, &config)
                .into_iter()
                .find(|registration| registration.surface_id == "codex")
                .unwrap()
                .health
        };

        std::fs::write(&hooks, r#"{"settings":{}}"#).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Missing
        );

        std::fs::write(&hooks, "{malformed").unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Malformed
        );

        let executable = std::env::current_exe().unwrap();
        let pre = format!(
            "\"{}\" hook pre --agent codex --rgt-managed",
            executable.display()
        );
        let post = format!(
            "\"{}\" hook post --agent codex --rgt-managed",
            executable.display()
        );
        let obsolete = serde_json::json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":pre}]}]}});
        std::fs::write(&hooks, serde_json::to_vec(&obsolete).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Obsolete
        );

        let post_under_pre = serde_json::json!({"hooks":{"PreToolUse":[{"hooks":[{
            "type":"command","command":post
        }]}]}});
        std::fs::write(&hooks, serde_json::to_vec(&post_under_pre).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Obsolete,
            "a post command under a pre event is not an active registration"
        );

        let wrong_callback_group = serde_json::json!({"hooks":{"PostToolUse":[{"hooks":[{
            "type":"shell","command":post
        }]}]}});
        std::fs::write(&hooks, serde_json::to_vec(&wrong_callback_group).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Obsolete,
            "the host callback type must be a command callback"
        );

        let duplicate = serde_json::json!({"hooks":{"PostToolUse":[
            {"hooks":[{"type":"command","command":post}]},
            {"hooks":[{"type":"command","command":post}]}
        ]}});
        std::fs::write(&hooks, serde_json::to_vec(&duplicate).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Ambiguous
        );

        let unavailable = serde_json::json!({"hooks":{"PostToolUse":[{"hooks":[{
            "type":"command","command":"/no-such-rgt-directory/rgt hook post --agent codex --rgt-managed"
        }]}]}});
        std::fs::write(&hooks, serde_json::to_vec(&unavailable).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Obsolete
        );

        let windows_wrapper = rgt::hooks::installer::direct_hook_command_for_platform(
            std::path::Path::new("/no-such-rgt-directory/rgt's $HOME"),
            "post",
            "codex",
            true,
        );
        let windows = serde_json::json!({"hooks":{"PostToolUse":[{"hooks":[{
            "type":"command","command":windows_wrapper
        }]}]}});
        std::fs::write(&hooks, serde_json::to_vec(&windows).unwrap()).unwrap();
        assert_eq!(
            status(),
            rgt::hooks::registration::RegistrationHealth::Obsolete,
            "doctor must inspect the nested RGT path in the PowerShell wrapper"
        );
    }

    #[test]
    fn marker_only_plugin_is_not_reported_as_active() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let plugin = dir.path().join(".pi/extensions/rgt.ts");
        std::fs::create_dir_all(plugin.parent().unwrap()).unwrap();
        std::fs::write(&plugin, "// RGT-managed integration.\n// marker only\n").unwrap();

        let registrations = rgt::hooks::registration::inspect_local_registrations(
            &dir.path().join("home"),
            &dir.path().join("config"),
        );
        let pi = registrations
            .iter()
            .find(|registration| registration.surface_id == "pi")
            .unwrap();
        assert_eq!(
            pi.health,
            rgt::hooks::registration::RegistrationHealth::Obsolete
        );
        assert_eq!(
            pi.ownership,
            rgt::hooks::registration::Ownership::CurrentRgt
        );

        let executable = std::env::current_exe().unwrap();
        std::fs::write(
            &plugin,
            rgt::hooks::glue::pi_extension(&executable.to_string_lossy()),
        )
        .unwrap();
        let registrations = rgt::hooks::registration::inspect_local_registrations(
            &dir.path().join("home"),
            &dir.path().join("config"),
        );
        let pi = registrations
            .iter()
            .find(|registration| registration.surface_id == "pi")
            .unwrap();
        assert_eq!(
            pi.health,
            rgt::hooks::registration::RegistrationHealth::Active,
            "a matching callback and resolvable nested executable are active"
        );

        std::fs::write(&plugin, rgt::hooks::glue::pi_extension("/missing/rgt")).unwrap();
        let registrations = rgt::hooks::registration::inspect_local_registrations(
            &dir.path().join("home"),
            &dir.path().join("config"),
        );
        let pi = registrations
            .iter()
            .find(|registration| registration.surface_id == "pi")
            .unwrap();
        assert_eq!(
            pi.health,
            rgt::hooks::registration::RegistrationHealth::Obsolete,
            "the plugin must not be active when its RGT executable is missing"
        );
    }

    #[test]
    fn copilot_chat_cli_and_windsurf_path_health_remain_separate_and_local() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        let command = format!(
            "\"{}\" hook post --agent copilot --rgt-managed",
            executable.display()
        );
        let chat = rgt::hooks::paths::copilot_user_settings(&config);
        std::fs::create_dir_all(chat.parent().unwrap()).unwrap();
        std::fs::write(&chat, serde_json::to_vec(&serde_json::json!({
            "github.copilot.chat.hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":command}]}]}
        })).unwrap()).unwrap();
        let cli = rgt::hooks::paths::copilot_cli_config_dir(&home, &config).join("AGENTS.md");
        std::fs::create_dir_all(cli.parent().unwrap()).unwrap();
        std::fs::write(
            &cli,
            rgt::hooks::glue::with_instruction("## RGT Integration\nUse `rgt record <file>`.\n"),
        )
        .unwrap();

        let mut registrations =
            rgt::hooks::registration::inspect_local_registrations(&home, &config);
        let chat = registrations
            .iter()
            .find(|registration| registration.surface_id == "copilot-chat")
            .unwrap();
        let cli = registrations
            .iter()
            .find(|registration| registration.surface_id == "copilot-cli")
            .unwrap();
        assert_eq!(
            chat.health,
            rgt::hooks::registration::RegistrationHealth::Active
        );
        assert_eq!(
            cli.health,
            rgt::hooks::registration::RegistrationHealth::GuidanceOnly
        );
        assert_eq!(
            chat.capture_tier,
            rgt::hooks::registration::CaptureTier::ConfiguredUnverified
        );
        assert_eq!(
            cli.capture_tier,
            rgt::hooks::registration::CaptureTier::InstructionOnly
        );

        let preferred = rgt::hooks::paths::windsurf_workspace_hooks_json();
        let legacy = rgt::hooks::paths::windsurf_legacy_workspace_hooks_json();
        std::fs::create_dir_all(preferred.parent().unwrap()).unwrap();
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&preferred, "{broken").unwrap();
        std::fs::write(&legacy, r#"{"hooks":{"post_read_code":[]}}"#).unwrap();
        registrations = rgt::hooks::registration::inspect_local_registrations(&home, &config);
        let windsurf = registrations
            .iter()
            .find(|registration| registration.surface_id == "windsurf")
            .unwrap();
        assert_eq!(
            windsurf.health,
            rgt::hooks::registration::RegistrationHealth::Malformed
        );
        assert_eq!(windsurf.artifact_path, preferred);
        assert_eq!(
            windsurf.capture_tier,
            rgt::hooks::registration::CaptureTier::InstructionOnly
        );
    }

    #[test]
    fn guidance_for_unverified_surfaces_does_not_promote_doctor_capture_tier() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        for agent in ["cursor", "gemini", "copilot"] {
            rgt::hooks::installer::detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                false,
                true,
                Some(agent),
                &executable,
            )
            .unwrap();
        }
        let registrations = rgt::hooks::registration::inspect_local_registrations(&home, &config);
        for (surface_id, tier) in [
            (
                "cursor",
                rgt::hooks::registration::CaptureTier::ConfiguredUnverified,
            ),
            (
                "copilot-chat",
                rgt::hooks::registration::CaptureTier::ConfiguredUnverified,
            ),
            (
                "gemini",
                rgt::hooks::registration::CaptureTier::InstructionOnly,
            ),
        ] {
            let registration = registrations
                .iter()
                .find(|r| r.surface_id == surface_id)
                .unwrap();
            assert_eq!(
                registration.health,
                rgt::hooks::registration::RegistrationHealth::Active
            );
            assert_eq!(registration.capture_tier, tier);
        }
        assert!(std::path::Path::new(".cursor/rules/rgt.mdc").exists());
        assert!(std::path::Path::new("GEMINI.md").exists());
        assert!(std::path::Path::new(".github/copilot-instructions.md").exists());
    }

    #[test]
    fn instruction_registration_requires_complete_owned_guidance() {
        use rgt::hooks::registration::{Ownership, RegistrationHealth};

        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let health = || {
            rgt::hooks::registration::inspect_local_registrations(&home, &config)
                .into_iter()
                .find(|registration| registration.surface_id == "cline")
                .unwrap()
        };

        std::fs::write(".clinerules", "# Team notes\n").unwrap();
        let unowned = health();
        assert_eq!(unowned.health, RegistrationHealth::Missing);
        assert_eq!(unowned.ownership, Ownership::UserOwned);

        std::fs::write(".clinerules", "## RGT Integration\n").unwrap();
        assert_eq!(health().health, RegistrationHealth::Malformed);

        std::fs::write(
            ".clinerules",
            "## RGT Integration\nNo recording command.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        assert_eq!(health().health, RegistrationHealth::Malformed);

        std::fs::write(
            ".clinerules",
            "## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        assert_eq!(health().health, RegistrationHealth::GuidanceOnly);

        std::fs::write(
            ".clinerules",
            "## RGT Integration\nUse `rgt record <file>`.\n",
        )
        .unwrap();
        let hand_maintained = health();
        assert_eq!(hand_maintained.health, RegistrationHealth::GuidanceOnly);
        assert_eq!(hand_maintained.ownership, Ownership::Ambiguous);
        let guidance = rgt::hooks::registration::inspect_local_guidance(&home, &config);
        let cline = guidance.iter().find(|g| g.surface_id == "cline").unwrap();
        assert_eq!(
            cline.health,
            rgt::hooks::registration::GuidanceHealth::UsableUnmanaged
        );
        assert_eq!(
            rgt::cli::guidance_diagnostic(cline).status,
            DiagnosticStatus::Warning
        );
    }

    #[test]
    fn dual_surface_guidance_health_is_separate_from_hook_health() {
        use rgt::hooks::registration::{GuidanceHealth, RegistrationHealth};

        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        for agent in ["cursor", "gemini", "copilot"] {
            rgt::hooks::installer::detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                false,
                true,
                Some(agent),
                &executable,
            )
            .unwrap();
        }
        let guidance = || rgt::hooks::registration::inspect_local_guidance(&home, &config);
        for surface in ["cursor", "gemini", "copilot-chat", "copilot-cli"] {
            assert_eq!(
                guidance()
                    .iter()
                    .find(|g| g.surface_id == surface)
                    .unwrap()
                    .health,
                GuidanceHealth::Usable,
                "{surface}"
            );
        }
        let cursor = guidance()
            .into_iter()
            .find(|g| g.surface_id == "cursor")
            .unwrap();
        assert_eq!(
            cursor.artifact_path,
            std::path::Path::new(".cursor/rules/rgt.mdc")
        );
        assert_eq!(
            rgt::cli::guidance_diagnostic(&cursor).status,
            DiagnosticStatus::Healthy
        );

        std::fs::remove_file(".cursor/rules/rgt.mdc").unwrap();
        std::fs::write(
            ".cursor/rules/other.mdc",
            "---\nalwaysApply: true\n---\n## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        let missing = guidance()
            .into_iter()
            .find(|g| g.surface_id == "cursor")
            .unwrap();
        assert_eq!(missing.health, GuidanceHealth::Missing);
        assert_eq!(
            rgt::cli::guidance_diagnostic(&missing).status,
            DiagnosticStatus::Warning
        );
        let registration = rgt::hooks::registration::inspect_local_registrations(&home, &config)
            .into_iter()
            .find(|r| r.surface_id == "cursor")
            .unwrap();
        assert_eq!(registration.health, RegistrationHealth::Active);

        std::fs::write(".cursor/rules/rgt.mdc", "## RGT Integration\n").unwrap();
        let incomplete = guidance()
            .into_iter()
            .find(|g| g.surface_id == "cursor")
            .unwrap();
        assert_eq!(incomplete.health, GuidanceHealth::Incomplete);
        assert_eq!(
            rgt::cli::guidance_diagnostic(&incomplete).status,
            DiagnosticStatus::Error
        );

        std::fs::write(
            ".cursor/rules/rgt.mdc",
            "---\ndescription: RGT\n---\nalwaysApply: true\n## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        assert_eq!(
            guidance()
                .into_iter()
                .find(|g| g.surface_id == "cursor")
                .unwrap()
                .health,
            GuidanceHealth::Incomplete,
            "alwaysApply outside frontmatter cannot make the rule load automatically"
        );

        std::fs::write(".cursor/rules/rgt.mdc", "# User rule\n").unwrap();
        let unowned = guidance()
            .into_iter()
            .find(|g| g.surface_id == "cursor")
            .unwrap();
        assert_eq!(unowned.health, GuidanceHealth::Unowned);
        assert_eq!(
            rgt::cli::guidance_diagnostic(&unowned).status,
            DiagnosticStatus::Warning
        );
    }

    #[test]
    fn doctor_cli_names_each_instruction_only_guidance_state() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        DbStore::open_in_project(dir.path()).unwrap();
        std::fs::create_dir_all(".agents/rules").unwrap();
        std::fs::write(".agents/rules/antigravity-rgt-rules.md", "# Team notes\n").unwrap();

        let doctor = || {
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
                .arg("doctor")
                .current_dir(dir.path())
                .env("HOME", &home)
                .env("XDG_CONFIG_HOME", &config)
                .output()
                .unwrap();
            (
                output.status.code().unwrap(),
                String::from_utf8(output.stdout).unwrap(),
            )
        };

        let (code, output) = doctor();
        assert_eq!(code, 0);
        assert!(output.contains("cline: Missing guidance"), "{output}");
        assert!(output.contains("roo-code: Missing guidance"), "{output}");
        assert!(output.contains("antigravity: Unowned guidance"), "{output}");

        std::fs::write(".clinerules", "## RGT Integration\n").unwrap();
        let (code, output) = doctor();
        assert_eq!(code, 1);
        assert!(output.contains("cline: Incomplete guidance"), "{output}");
        assert!(output.contains("roo-code: Incomplete guidance"), "{output}");

        std::fs::write(
            ".clinerules",
            "## RGT Integration\nUse `rgt record <file>`.\n",
        )
        .unwrap();
        let (code, output) = doctor();
        assert_eq!(code, 0);
        assert!(
            output.contains("cline: Hand-maintained guidance"),
            "{output}"
        );

        std::fs::write(
            ".clinerules",
            "## RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        let (code, output) = doctor();
        assert_eq!(code, 0);
        assert!(output.contains("cline: Managed guidance"), "{output}");
        assert!(output.contains("Instruction-only"), "{output}");

        std::fs::write(
            ".agents/rules/antigravity-rgt-rules.md",
            "# RGT Integration\nUse `rgt record <file>`.\n<!-- /RGT Integration -->\n",
        )
        .unwrap();
        let (code, output) = doctor();
        assert_eq!(code, 0);
        assert!(output.contains("antigravity: Managed guidance"), "{output}");
    }

    #[test]
    fn doctor_cli_reports_pi_hermes_and_vibe_guidance_without_capture_claims() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        DbStore::open_in_project(dir.path()).unwrap();
        for agent in ["pi", "hermes", "vibe"] {
            rgt::hooks::installer::detect_and_configure_hooks_with_config_and_exe(
                &home,
                &config,
                false,
                false,
                Some(agent),
                &executable,
            )
            .unwrap();
        }
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .arg("doctor")
            .current_dir(dir.path())
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", &config)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        for surface in ["pi", "hermes", "vibe"] {
            assert!(
                stdout.contains(&format!("{surface}: Managed guidance")),
                "{stdout}"
            );
        }
        assert!(stdout.contains("ConfiguredUnverified"), "{stdout}");
        assert!(stdout.contains("InstructionOnly"), "{stdout}");
    }

    #[test]
    fn hermes_doctor_checks_project_enablement_separately_from_guidance() {
        use rgt::hooks::registration::{GuidanceHealth, RegistrationHealth};

        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config_dir = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        DbStore::open_in_project(dir.path()).unwrap();
        rgt::hooks::installer::detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            false,
            false,
            Some("hermes"),
            &executable,
        )
        .unwrap();
        let config_path = dir.path().join(".hermes/config.toml");
        let registration = || {
            rgt::hooks::registration::inspect_local_registrations(&home, &config_dir)
                .into_iter()
                .find(|item| item.surface_id == "hermes")
                .unwrap()
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
                output.status.code().unwrap(),
                String::from_utf8(output.stdout).unwrap(),
            )
        };
        assert_eq!(registration().health, RegistrationHealth::Active);
        assert_eq!(doctor().0, 0);

        for (config_text, expected) in [
            (
                Some("[plugins]\nenabled = [\"other\"]\n"),
                RegistrationHealth::Obsolete,
            ),
            (Some("[plugins]\n"), RegistrationHealth::Obsolete),
            (Some("[plugins\n"), RegistrationHealth::Malformed),
            (
                Some("[plugins]\nenabled = \"rgt\"\n"),
                RegistrationHealth::Malformed,
            ),
            (None, RegistrationHealth::Obsolete),
        ] {
            if let Some(text) = config_text {
                std::fs::write(&config_path, text).unwrap();
            } else {
                std::fs::remove_file(&config_path).unwrap();
            }
            assert_eq!(registration().health, expected);
            let (code, output) = doctor();
            assert_eq!(code, 1, "{output}");
            assert!(
                output.contains(&format!("hermes: {expected:?} registration")),
                "{output}"
            );
            assert!(output.contains(".hermes/config.toml"), "{output}");
            assert!(output.contains("rgt init --agent hermes"), "{output}");
            assert!(output.contains("hermes: Managed guidance"), "{output}");
            let guidance = rgt::hooks::registration::inspect_local_guidance(&home, &config_dir)
                .into_iter()
                .find(|item| item.surface_id == "hermes")
                .unwrap();
            assert_eq!(guidance.health, GuidanceHealth::Usable);
        }
        std::fs::write(
            &config_path,
            "plugins = { enabled = [\"other\", \"rgt\"] }\n",
        )
        .unwrap();
        assert_eq!(registration().health, RegistrationHealth::Active);
        assert_eq!(doctor().0, 0);
    }

    #[test]
    fn hermes_doctor_checks_user_enablement_at_user_scope() {
        use rgt::hooks::registration::{RegistrationHealth, RegistrationScope};

        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let home = dir.path().join("home");
        let config_dir = dir.path().join("config");
        let executable = std::env::current_exe().unwrap();
        DbStore::open_in_project(dir.path()).unwrap();
        rgt::hooks::installer::detect_and_configure_hooks_with_config_and_exe(
            &home,
            &config_dir,
            true,
            false,
            Some("hermes"),
            &executable,
        )
        .unwrap();
        let registration = || {
            rgt::hooks::registration::inspect_local_registrations(&home, &config_dir)
                .into_iter()
                .find(|item| item.surface_id == "hermes")
                .unwrap()
        };
        assert_eq!(registration().scope, RegistrationScope::User);
        assert_eq!(registration().health, RegistrationHealth::Active);
        std::fs::create_dir_all(".hermes").unwrap();
        std::fs::write(".hermes/config.toml", "[plugins]\nenabled = [\"rgt\"]\n").unwrap();
        std::fs::write(
            home.join(".hermes/config.toml"),
            "[plugins]\nenabled = []\n",
        )
        .unwrap();
        assert_eq!(registration().health, RegistrationHealth::Obsolete);
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
            .arg("doctor")
            .current_dir(dir.path())
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", &config_dir)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains(&home.join(".hermes/config.toml").display().to_string()),
            "{stdout}"
        );
    }
}
