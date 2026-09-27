use base64::Engine;
use rgt::hooks::installer::{
    detect_and_configure_hooks_with_config_and_exe, resolve_agent_name, valid_agent_names,
    AgentOutcome, InstallReport,
};
use rgt::hooks::paths::{copilot_cli_config_dir, copilot_user_settings, windsurf_user_hooks_json};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::tempdir;

static CWD_MUTEX: Mutex<()> = Mutex::new(());

/// Fixed absolute path injected into hook commands for deterministic tests.
fn injected_exe() -> PathBuf {
    PathBuf::from("/usr/local/bin/rgt")
}

fn set_cwd(dir: &Path) -> std::sync::MutexGuard<'static, ()> {
    let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_current_dir(dir).unwrap();
    guard
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

fn run(home: &Path, config_dir: &Path, global: bool, force: bool, agent: &str) -> InstallReport {
    detect_and_configure_hooks_with_config_and_exe(
        home,
        config_dir,
        global,
        force,
        Some(agent),
        &injected_exe(),
    )
    .unwrap()
}

fn run_none(home: &Path, config_dir: &Path, global: bool, force: bool) -> InstallReport {
    detect_and_configure_hooks_with_config_and_exe(
        home,
        config_dir,
        global,
        force,
        None,
        &injected_exe(),
    )
    .unwrap()
}

/// The shell-quoted absolute path prefix expected in command hooks.
fn exe_prefix() -> String {
    format!("'{}'", injected_exe().display())
}

/// Same as [`exe_prefix`]; JSON strings keep POSIX single quotes unescaped.
fn exe_prefix_json() -> String {
    format!("'{}'", injected_exe().display())
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

fn failed(report: &InstallReport) -> Vec<String> {
    report
        .outcomes
        .iter()
        .filter_map(|o| match o {
            AgentOutcome::Failed { agent, .. } | AgentOutcome::Conflict { agent, .. } => {
                Some(agent.clone())
            }
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// US1: preservation (T010) — existing user configuration is never destroyed
// ---------------------------------------------------------------------------

#[test]
fn claude_merge_preserves_foreign_hooks_settings_and_comments() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let settings = home.join(".claude").join("settings.json");

    write(
        &settings,
        r#"{
  // RTK's own hook group — must survive
  "hooks": {
    "PostToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rtk hook post" }] },
    ],
  },
  "keybindings": { "esc": "stop" },
}
"#,
    );

    let report = run(&home, &config_dir, true, false, "claude-code");
    assert_eq!(configured(&report), vec!["claude-code".to_string()]);

    let content = read(&settings);
    assert!(content.contains("// RTK's own hook group — must survive"));
    assert!(content.contains("\"rtk hook post\""));
    assert!(content.contains("\"keybindings\": { \"esc\": \"stop\" }"));
    assert!(content.contains("Read|Edit|Write|Bash"));
    assert!(content.contains(&format!("{} hook post", exe_prefix_json())));
    assert!(content.contains(&format!("{} hook pre", exe_prefix_json())));
    assert!(
        !content.contains("\"rgt hook post\""),
        "must not use bare rgt"
    );
    // Output is valid JSONC.
    jsonc_parser::parse_to_value(&content, &Default::default()).expect("must parse as JSONC");

    // Second run without --force: nothing changes (FR-005).
    let report2 = run(&home, &config_dir, true, false, "claude-code");
    assert_eq!(skipped(&report2), vec!["claude-code".to_string()]);
    assert_eq!(read(&settings), content);
}

#[test]
fn cursor_merge_preserves_existing_entries_and_version() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let hooks = home.join(".cursor").join("hooks.json");

    write(
        &hooks,
        r#"{
  "version": 2,
  "hooks": {
    "postToolUse": [ { "command": "notify --done", "matcher": "Bash" } ]
  }
}
"#,
    );

    let report = run(&home, &config_dir, true, false, "cursor");
    assert_eq!(configured(&report), vec!["cursor".to_string()]);

    let content = read(&hooks);
    assert!(content.contains("\"version\": 2"));
    assert!(content.contains("notify --done"));
    assert!(content.contains(&format!("{} hook post", exe_prefix_json())));
    assert!(content.contains(&format!("{} hook pre", exe_prefix_json())));
    assert!(!content.contains("\"rgt hook post\""));
    serde_json::from_str::<serde_json::Value>(&content).expect("must be valid JSON");
}

#[test]
fn copilot_writer_merges_chat_hooks_into_user_settings() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Pre-seed the user settings file with user-authored content.
    let settings = config_dir.join("Code").join("User").join("settings.json");
    write(
        &settings,
        "{\n  \"editor.fontSize\": 14,\n  \"github.copilot.chat.hooks\": { \"triggerNotifications\": true }\n}\n",
    );

    let report = run(&home, &config_dir, true, true, "copilot");
    assert_eq!(configured(&report), vec!["copilot".to_string()]);

    let content = read(&settings);
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["editor.fontSize"], 14); // user content preserved
    assert_eq!(
        parsed["github.copilot.chat.hooks"]["triggerNotifications"],
        true
    );
    let hooks = &parsed["github.copilot.chat.hooks"]["PostToolUse"][0];
    let cmd = hooks["hooks"][0]["command"].as_str().unwrap();
    assert_eq!(
        cmd,
        rgt::hooks::installer::direct_hook_command_for_platform(
            &injected_exe(),
            "post",
            "copilot",
            cfg!(windows),
        )
    );
}

#[test]
fn hermes_config_keeps_other_plugins_and_single_table() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let config = home.join(".hermes").join("config.toml");

    write(
        &config,
        "# user comment\n[plugins]\nenabled = [  # keep me\n  \"git\",\n]\n",
    );

    let report = run(&home, &config_dir, true, false, "hermes");
    assert_eq!(configured(&report), vec!["hermes".to_string()]);

    let content = read(&config);
    assert!(content.contains("# user comment"));
    assert!(content.contains("# keep me"));
    assert!(content.contains("\"git\""));
    assert!(content.contains("\"rgt\""));
    assert_eq!(
        content.matches("[plugins]").count(),
        1,
        "no duplicate table"
    );
    let _: toml_edit::DocumentMut = content.parse().expect("must parse as TOML");
}

// ---------------------------------------------------------------------------
// Writer smoke tests (adapted to the report API)
// ---------------------------------------------------------------------------

#[test]
fn existing_four_agents_write_with_markers() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, true, true, "claude-code");
    run(&home, &config_dir, true, true, "cursor");
    run(&home, &config_dir, true, true, "codex");
    run(&home, &config_dir, true, true, "windsurf");

    let claude = read(&home.join(".claude").join("settings.json"));
    assert!(claude.contains("Read|Edit|Write|Bash"));
    assert!(claude.contains(&format!("{} hook post", exe_prefix_json())));
    assert!(claude.contains(&format!("{} hook pre", exe_prefix_json())));
    serde_json::from_str::<serde_json::Value>(&claude).unwrap();

    let cursor = read(&home.join(".cursor").join("hooks.json"));
    assert!(cursor.contains(&format!("{} hook post", exe_prefix_json())));
    serde_json::from_str::<serde_json::Value>(&cursor).unwrap();

    let agents_md = read(&PathBuf::from("AGENTS.md"));
    assert!(agents_md.starts_with("# RGT Agents Instructions"));
    assert!(agents_md.contains("## RGT Integration"));
    assert!(agents_md.contains("rgt record <file>"));
    assert!(agents_md.contains("<!-- /RGT Integration -->"));

    let windsurf = read(&PathBuf::from(".windsurfrules"));
    assert!(windsurf.starts_with("# RGT Integration\n"));
    assert!(windsurf.contains("rgt derive --parents"));
    assert!(windsurf.contains("<!-- /RGT Integration -->"));
}

#[test]
fn copilot_writer_creates_cli_rules_file() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, true, true, "copilot");

    let rules = read(&copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md"));
    assert!(rules.contains("## RGT Integration"));
    assert!(rules.contains("rgt record <file>"));
    assert!(rules.contains("<!-- /RGT Integration -->"));

    // Idempotent re-run without force: no change.
    let report = run(&home, &config_dir, true, false, "copilot");
    assert!(skipped(&report).contains(&"copilot".to_string()));
    let second = read(&copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md"));
    assert_eq!(rules, second);
}

#[test]
fn gemini_writer_creates_hooks_toml() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let report = run(&home, &config_dir, true, true, "gemini");
    assert_eq!(configured(&report), vec!["gemini".to_string()]);

    let toml = read(&home.join(".gemini").join("hooks.toml"));
    assert!(toml.contains("[PostToolUse]"));
    assert!(toml.contains(&format!("{} hook post --agent gemini", exe_prefix())));
    let _: toml_edit::DocumentMut = toml.parse().unwrap();
}

#[test]
fn vibe_writer_creates_hooks_toml_and_loaded_guidance() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let report = run(&home, &config_dir, true, true, "vibe");
    assert_eq!(configured(&report), vec!["vibe".to_string()]);

    let toml = read(&home.join(".vibe").join("hooks.toml"));
    assert!(toml.contains("[[pre_tool]]"));
    assert!(toml.contains("match = \"bash\""));
    assert!(toml.contains("strict = false"));
    assert!(toml.contains(&format!("{} hook pre --agent vibe", exe_prefix())));
    let _: toml_edit::DocumentMut = toml.parse().unwrap();

    let guidance = read(&home.join(".vibe").join("AGENTS.md"));
    assert!(guidance.contains("RGT Integration"));
    assert!(guidance.contains("<!-- /RGT Integration -->"));
    assert!(!home.join(".vibe/prompts/rgt.md").exists());
}

#[test]
fn opencode_writer_project_and_global() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, false, true, "opencode");
    let project_plugin = read(&PathBuf::from(".opencode").join("plugins").join("rgt.ts"));
    assert_thin_glue(&project_plugin, "opencode");
    assert!(project_plugin.contains("export const RgtPlugin = async"));
    assert!(project_plugin.contains("\"tool.execute.after\""));

    run(&home, &config_dir, true, true, "opencode");
    let global_plugin = read(
        &home
            .join(".config")
            .join("opencode")
            .join("plugins")
            .join("rgt.ts"),
    );
    assert_thin_glue(&global_plugin, "opencode");
}

#[test]
fn pi_writer_project_and_global() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, false, true, "pi");
    let project_ext = read(&PathBuf::from(".pi").join("extensions").join("rgt.ts"));
    assert_thin_glue(&project_ext, "pi");

    run(&home, &config_dir, true, true, "pi");
    let global_ext = read(
        &home
            .join(".pi")
            .join("agent")
            .join("extensions")
            .join("rgt.ts"),
    );
    assert_thin_glue(&global_ext, "pi");
}

#[test]
fn hermes_writer_creates_plugin_and_enables_it() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let report = run(&home, &config_dir, true, true, "hermes");
    assert_eq!(configured(&report), vec!["hermes".to_string()]);

    let plugin = read(
        &home
            .join(".hermes")
            .join("plugins")
            .join("rgt")
            .join("plugin.py"),
    );
    assert_thin_glue(&plugin, "hermes");

    let config = read(&home.join(".hermes").join("config.toml"));
    assert!(config.contains("plugins"));
    assert!(config.contains("\"rgt\""));
}

#[test]
fn rules_writers_create_files() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, false, true, "cline");
    assert!(read(&PathBuf::from(".clinerules")).contains("## RGT Integration"));

    run(&home, &config_dir, false, true, "antigravity");
    let ag = read(
        &PathBuf::from(".agents")
            .join("rules")
            .join("antigravity-rgt-rules.md"),
    );
    assert!(ag.contains("RGT Integration"));
    assert!(ag.contains("<!-- /RGT Integration -->"));

    run(&home, &config_dir, false, true, "kilocode");
    let kilo = read(
        &PathBuf::from(".kilocode")
            .join("rules")
            .join("rgt-rules.md"),
    );
    assert!(kilo.contains("RGT Integration"));
    assert!(kilo.contains("<!-- /RGT Integration -->"));
}

// ---------------------------------------------------------------------------
// US2: loud failures (T016) — malformed config is never silently overwritten
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_fails_loudly_and_file_is_untouched() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let settings = home.join(".claude").join("settings.json");

    let malformed = "{ \"hooks\": {\n";
    write(&settings, malformed);

    let report = run(&home, &config_dir, true, true, "claude-code");
    let reason = report
        .outcomes
        .iter()
        .find_map(|o| match o {
            AgentOutcome::Failed { agent, reason, .. } if agent == "claude-code" => {
                Some(reason.clone())
            }
            _ => None,
        })
        .expect("claude-code must fail");
    assert!(
        reason.contains("settings.json"),
        "reason must name the file: {reason}"
    );
    assert_eq!(read(&settings), malformed, "file must be byte-identical");
}

#[test]
fn malformed_toml_fails_loudly() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let config = home.join(".hermes").join("config.toml");

    let malformed = "[plugins\nenabled = [\n";
    write(&config, malformed);

    let report = run(&home, &config_dir, true, true, "hermes");
    assert!(failed(&report).contains(&"hermes".to_string()));
    assert_eq!(read(&config), malformed);
}

#[test]
fn jsonc_with_comments_is_tolerated() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let settings = home.join(".claude").join("settings.json");

    write(
        &settings,
        "{\n  // keep me\n  \"permissions\": { \"defaultMode\": \"acceptEdits\" },\n}\n",
    );

    let report = run(&home, &config_dir, true, true, "claude-code");
    assert_eq!(configured(&report), vec!["claude-code".to_string()]);
    let content = read(&settings);
    assert!(content.contains("// keep me"), "comments preserved");
    assert!(content.contains("\"permissions\""));
    jsonc_parser::parse_to_value(&content, &Default::default()).expect("must parse as JSONC");
}

#[test]
fn partial_failure_still_configures_healthy_agents() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Claude trigger present but malformed; Gemini trigger present and healthy.
    write(&home.join(".claude").join("settings.json"), "{ broken");
    write(&home.join(".gemini").join("hooks.toml"), "");

    let report = run_none(&home, &config_dir, true, true);
    assert!(failed(&report).contains(&"claude-code".to_string()));
    assert!(configured(&report).contains(&"gemini".to_string()));
}

#[test]
fn backup_created_before_any_write_to_existing_file() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let settings = home.join(".claude").join("settings.json");

    write(&settings, "{\"permissions\": {}}");
    run(&home, &config_dir, true, false, "claude-code");

    let backup = settings.with_file_name("settings.json.rgt.bak");
    assert!(
        backup.exists(),
        "backup must exist before a write to an existing file"
    );
    assert_eq!(read(&backup), "{\"permissions\": {}}");
}

// ---------------------------------------------------------------------------
// US3: re-init safety (T021 contract-level) — idempotency & --force block
// ---------------------------------------------------------------------------

#[test]
fn rerun_without_force_is_idempotent_for_toml() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, true, true, "gemini");
    let first = read(&home.join(".gemini").join("hooks.toml"));

    let report = run(&home, &config_dir, true, false, "gemini");
    assert!(skipped(&report).contains(&"gemini".to_string()));
    assert_eq!(read(&home.join(".gemini").join("hooks.toml")), first);
}

#[test]
fn rules_file_appends_once_not_duplicated() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, false, true, "cline");
    let first = read(&PathBuf::from(".clinerules"));

    let report = run(&home, &config_dir, false, false, "cline");
    assert!(skipped(&report).contains(&"cline".to_string()));
    assert_eq!(read(&PathBuf::from(".clinerules")), first);
    assert_eq!(first.matches("## RGT Integration").count(), 1);
}

#[test]
fn force_replaces_rgt_block_preserving_user_content_below() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules_file = PathBuf::from(".clinerules");

    // RGT block (with end marker) followed by user-authored notes.
    write(
        &rules_file,
        "# User rules\n# More user rules\n## RGT Integration\nSTALE CONTENT\n<!-- /RGT Integration -->\n## My personal notes\nkeep me\n",
    );
    run(&home, &config_dir, false, true, "cline");
    let content = read(&rules_file);
    assert!(content.starts_with("# User rules\n"));
    assert!(content.contains("# More user rules\n"));
    assert!(!content.contains("STALE CONTENT"));
    assert!(
        content.contains("## My personal notes\nkeep me\n"),
        "notes below block must survive"
    );
    assert_eq!(content.matches("## RGT Integration").count(), 1);
    assert!(content.contains("rgt record <file>"));
}

#[test]
fn normal_init_repairs_a_recognized_old_instruction_block_with_backup() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules = PathBuf::from(".clinerules");
    let legacy = "# Team rules\n## RGT Integration\nold instructions\n<!-- /RGT Integration -->\n# Keep below\nuser text\n";
    write(&rules, legacy);

    let report = run(&home, &config_dir, false, false, "cline");
    assert!(configured(&report).contains(&"cline".to_string()));
    assert_eq!(read(&rules.with_file_name(".clinerules.rgt.bak")), legacy);
    let migrated = read(&rules);
    assert!(!migrated.contains("old instructions"));
    assert!(migrated.contains("# Team rules"));
    assert!(migrated.contains("# Keep below\nuser text"));
}

#[test]
fn force_on_legacy_block_without_end_marker_errors_and_untouches() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules_file = PathBuf::from(".clinerules");

    let legacy = "## RGT Integration\nSTALE CONTENT\nuser notes\n";
    write(&rules_file, legacy);

    let report = run(&home, &config_dir, false, true, "cline");
    let reason = report
        .outcomes
        .iter()
        .find_map(|o| match o {
            AgentOutcome::Failed { agent, reason, .. } if agent == "cline" => Some(reason.clone()),
            _ => None,
        })
        .expect("cline must fail on a legacy block");
    assert!(
        reason.contains(".clinerules"),
        "reason must name the file: {reason}"
    );
    assert!(reason.contains("end marker"));
    assert_eq!(
        read(&rules_file),
        legacy,
        "legacy block must be left untouched"
    );
}

#[test]
fn malformed_legacy_block_without_force_reports_conflict_and_is_not_modified() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules_file = PathBuf::from(".clinerules");

    let legacy = "## RGT Integration\nSTALE CONTENT\n";
    write(&rules_file, legacy);

    let report = run(&home, &config_dir, false, false, "cline");
    assert!(failed(&report).contains(&"cline".to_string()));
    assert_eq!(read(&rules_file), legacy);
}

#[test]
fn plugin_file_with_unknown_ownership_is_preserved_even_with_force() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let plugin_file = PathBuf::from(".opencode").join("plugins").join("rgt.ts");

    write(&plugin_file, "// user-authored plugin\n");
    let report = run(&home, &config_dir, false, false, "opencode");
    assert!(failed(&report).contains(&"opencode".to_string()));
    assert_eq!(read(&plugin_file), "// user-authored plugin\n");

    let forced = run(&home, &config_dir, false, true, "opencode");
    assert!(failed(&forced).contains(&"opencode".to_string()));
    assert_eq!(read(&plugin_file), "// user-authored plugin\n");
}

// ---------------------------------------------------------------------------
// Auto-detection + one-command setup (US2)
// ---------------------------------------------------------------------------

#[test]
fn init_no_agent_configures_all_detected() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Emulate several agents installed via their triggers.
    write(&home.join(".claude").join("settings.json"), "{}");
    write(&home.join(".vibe").join("hooks.toml"), "");
    write(&PathBuf::from(".kilocode").join("rules").join("x"), "");
    write(&PathBuf::from(".clinerules"), "# rules\n");
    write(
        &config_dir.join("Code").join("User").join("settings.json"),
        "{}",
    );

    let report = run_none(&home, &config_dir, false, true);
    let c = configured(&report);
    for agent in ["claude-code", "vibe", "kilocode", "cline", "copilot"] {
        assert!(c.contains(&agent.to_string()), "missing {}", agent);
    }
}

#[test]
fn init_no_agent_with_nothing_detected_reports_empty() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let report = run_none(&home, &config_dir, false, true);
    assert!(report.is_empty());
}

// ---------------------------------------------------------------------------
// Agent name validation (FR-001)
// ---------------------------------------------------------------------------

#[test]
fn aliases_resolve_to_canonical_names() {
    assert_eq!(resolve_agent_name("claude").as_deref(), Some("claude-code"));
    assert_eq!(
        resolve_agent_name("claude-code").as_deref(),
        Some("claude-code")
    );
    assert_eq!(resolve_agent_name("roo-code").as_deref(), Some("cline"));
    assert_eq!(resolve_agent_name("cline").as_deref(), Some("cline"));
    assert_eq!(resolve_agent_name("kilo").as_deref(), Some("kilocode"));
    assert_eq!(resolve_agent_name("kilocode").as_deref(), Some("kilocode"));
    assert_eq!(resolve_agent_name("nonexistent"), None);
}

#[test]
fn direct_hook_fallback_commands_are_generated_for_unix_and_windows() {
    use rgt::hooks::installer::direct_hook_command_for_platform;
    let path = PathBuf::from("/Users/example path/rgt");
    let unix = direct_hook_command_for_platform(&path, "post", "codex", false);
    assert!(unix.contains("'/Users/example path/rgt' hook post --agent codex --rgt-managed"));
    assert!(unix.contains("mktemp -d"));
    assert!(unix.contains("sleep 0.8"));
    assert!(unix.contains("timed-out"));
    assert!(unix.contains("kill -KILL"));
    assert!(unix.contains("cat \"$_rgt_tmp/stdout\""));
    assert!(unix.ends_with("2>/dev/null || true"));
    let windows = direct_hook_command_for_platform(
        Path::new("C:\\Program Files\\RGT\\rgt.exe"),
        "post",
        "codex",
        true,
    );
    assert!(
        windows.starts_with("powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand ")
    );
    let encoded = windows.split_whitespace().last().unwrap();
    let script = String::from_utf16_lossy(
        &base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap()
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    );
    assert!(script.contains("FileName='C:\\Program Files\\RGT\\rgt.exe'"));
    assert!(script.contains("Arguments='hook post --agent codex --rgt-managed'"));
    assert!(script.contains("catch { } finally {"));
    assert!(script.ends_with("exit 0"));
    assert!(script.contains("Stopwatch]::StartNew()"));
    assert!(script.contains("CopyToAsync($p.StandardInput.BaseStream,81920,$cts.Token)"));
    assert!(script.contains("WaitForExit([int]$remaining)"));
    assert!(script.contains("$p.Kill()"));
    assert!(script.contains("$p.WaitForExit(50)"));
    assert!(script.contains("[Console]::Out.Write($stdoutTask.Result)"));
    assert!(!script.contains("[Console]::OpenStandardInput().CopyTo("));
    assert!(!script.contains("$stdoutTask.Wait()"));
    assert!(!script.contains("$p.WaitForExit()"));

    let windows_special = direct_hook_command_for_platform(
        Path::new("C:\\Program Files\\RGT\\%TEMP%! 'quoted' $value.exe"),
        "post",
        "codex",
        true,
    );
    let encoded = windows_special.split_whitespace().last().unwrap();
    let special_script = String::from_utf16_lossy(
        &base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap()
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    );
    assert!(
        special_script.contains("FileName='C:\\Program Files\\RGT\\%TEMP%! ''quoted'' $value.exe'")
    );
    assert!(special_script.contains("catch { } finally {"));
    assert!(special_script.ends_with("exit 0"));
}

#[test]
fn vibe_init_registration_is_current_in_doctor_and_idempotent() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let executable = std::env::current_exe().unwrap();

    let first = detect_and_configure_hooks_with_config_and_exe(
        &home,
        &config_dir,
        true,
        false,
        Some("vibe"),
        &executable,
    )
    .unwrap();
    assert_eq!(configured(&first), vec!["vibe".to_string()]);
    let hooks_path = home.join(".vibe/hooks.toml");
    let installed = read(&hooks_path);
    assert!(installed.contains("hook pre --agent vibe"));

    let registrations = rgt::hooks::registration::inspect_local_registrations(&home, &config_dir);
    let vibe = registrations
        .iter()
        .find(|registration| registration.surface_id == "vibe")
        .unwrap();
    assert_eq!(
        vibe.health,
        rgt::hooks::registration::RegistrationHealth::Active
    );
    assert_eq!(
        vibe.capture_tier,
        rgt::hooks::registration::CaptureTier::InstructionOnly
    );
    assert!(vibe
        .event_phases
        .contains(&rgt::hooks::registration::EventPhase::Before));

    let second = detect_and_configure_hooks_with_config_and_exe(
        &home,
        &config_dir,
        true,
        false,
        Some("vibe"),
        &executable,
    )
    .unwrap();
    assert_eq!(read(&hooks_path), installed);
    assert!(
        second.outcomes.iter().any(|outcome| matches!(
            outcome,
            AgentOutcome::AlreadyCurrent { agent, .. } if agent == "vibe"
        )),
        "second initialization should report already current: {:?}",
        second.outcomes
    );
}

#[cfg(unix)]
#[test]
fn direct_hook_command_safely_quotes_shell_expansion_characters_and_fails_open() {
    use rgt::hooks::installer::direct_hook_command_for_platform;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let dir = tempdir().unwrap();
    let marker = dir.path().join("expanded");
    let executable = dir.path().join("rgt $HOME `touch expanded`; 'quoted'");
    std::fs::write(&executable, "#!/bin/sh\nprintf 'hook-output'\n").unwrap();
    let mut permissions = std::fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&executable, permissions).unwrap();

    let command = direct_hook_command_for_platform(&executable, "post", "codex", false);
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"hook-output");
    assert!(
        !marker.exists(),
        "quoted path must not execute shell substitutions"
    );

    let missing = direct_hook_command_for_platform(
        &dir.path().join("missing $HOME `false`; 'rgt'"),
        "post",
        "codex",
        false,
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(missing)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "a missing CLI must fail open");
}

#[test]
fn opencode_legacy_plugin_is_backed_up_migrated_and_idempotent() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let legacy_plugin = dir.path().join(".opencode/plugin/rgt.ts");
    let plugin = dir.path().join(".opencode/plugins/rgt.ts");
    let legacy = r#"import { plugin } from "@opencode-ai/plugin";
export const rgt = plugin("rgt", { tool: { execute: { after: async (args) => {
  spawnSync("/old/rgt", ["hook", "post", "--agent", "opencode"]);
}}}});
"#;
    write(&legacy_plugin, legacy);

    let first = run(&home, &config_dir, false, false, "opencode");
    assert_eq!(configured(&first), vec!["opencode"]);
    assert!(
        matches!(first.outcomes.as_slice(), [AgentOutcome::Migrated { backups, .. }] if backups == &vec![PathBuf::from(".opencode/plugin/rgt.ts.rgt.bak")])
    );
    assert_eq!(read(&legacy_plugin.with_extension("ts.rgt.bak")), legacy);
    assert!(!legacy_plugin.exists());
    assert!(read(&plugin).contains("RGT-managed integration"));
    assert!(read(&plugin).contains("--rgt-managed"));

    let installed = read(&plugin);
    let second = run(&home, &config_dir, false, false, "opencode");
    assert_eq!(skipped(&second), vec!["opencode"]);
    assert_eq!(read(&plugin), installed);
}

#[test]
fn native_json_surface_migrations_keep_unrelated_settings_and_are_idempotent() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let cases = [
        (
            "claude-code",
            home.join(".claude/settings.json"),
            serde_json::json!({"settings":{"keep":true},"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"\"/old/rgt\" hook post --agent claude-code"}]}]}}),
        ),
        (
            "cursor",
            home.join(".cursor/hooks.json"),
            serde_json::json!({"settings":{"keep":true},"hooks":{"postToolUse":[{"command":"\"/old/rgt\" hook post --agent cursor"}]}}),
        ),
        (
            "copilot",
            copilot_user_settings(&config_dir),
            serde_json::json!({"settings":{"keep":true},"github.copilot.chat.hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":"\"/old/rgt\" hook post --agent copilot"}]}]}}),
        ),
        (
            "windsurf",
            windsurf_user_hooks_json(&home),
            serde_json::json!({"settings":{"keep":true},"hooks":{"post_read_code":[{"command":"\"/old/rgt\" hook post --agent windsurf"}]}}),
        ),
    ];

    for (agent, path, legacy) in cases {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = serde_json::to_string_pretty(&legacy).unwrap();
        std::fs::write(&path, &original).unwrap();

        let first = run(&home, &config_dir, true, false, agent);
        let expected_backup = PathBuf::from(format!("{}.rgt.bak", path.display()));
        assert!(
            matches!(
                first.outcomes.as_slice(),
                [AgentOutcome::Migrated { backups, .. }]
                    if backups == &vec![expected_backup.clone()]
            ),
            "{agent}: {:?}",
            first.outcomes
        );
        let upgraded: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(upgraded["settings"]["keep"], true, "{agent}");
        let expected_callbacks = if matches!(agent, "claude-code" | "cursor") {
            2
        } else {
            1
        };
        assert_eq!(
            upgraded.to_string().matches("--rgt-managed").count(),
            expected_callbacks,
            "{agent}"
        );
        assert_eq!(
            std::fs::read_to_string(&expected_backup).unwrap(),
            original,
            "{agent}"
        );

        let second = run(&home, &config_dir, true, false, agent);
        assert!(
            matches!(
                second.outcomes.as_slice(),
                [AgentOutcome::AlreadyCurrent { .. }]
            ),
            "{agent}: {:?}",
            second.outcomes
        );
    }
}

#[test]
fn plugin_with_ambiguous_ownership_is_preserved_even_with_force() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let plugin = dir.path().join(".pi/extensions/rgt.ts");
    let foreign = "// user-owned plugin\nexport const extension = { name: \"custom\" };\n";
    write(&plugin, foreign);

    let report = run(&home, &config_dir, false, true, "pi");
    assert_eq!(failed(&report), vec!["pi"]);
    assert!(
        matches!(report.outcomes.as_slice(), [AgentOutcome::Conflict { artifact: Some(path), backup: None, .. }] if path == &PathBuf::from(".pi/extensions/rgt.ts"))
    );
    assert_eq!(read(&plugin), foreign);
    assert!(!plugin.with_extension("ts.rgt.bak").exists());
}

#[test]
fn valid_agent_names_has_16_spellings() {
    let names = valid_agent_names();
    assert_eq!(names.len(), 16);
    for n in [
        "claude-code",
        "cursor",
        "codex",
        "windsurf",
        "copilot",
        "gemini",
        "vibe",
        "opencode",
        "pi",
        "hermes",
        "cline",
        "roo-code",
        "antigravity",
        "kilocode",
        "claude",
        "kilo",
    ] {
        assert!(names.contains(&n), "missing {}", n);
    }
}

#[test]
fn unknown_agent_configures_nothing() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let report = detect_and_configure_hooks_with_config_and_exe(
        &home,
        &config_dir,
        true,
        true,
        Some("nope"),
        &injected_exe(),
    )
    .unwrap();
    assert!(report.is_empty());
}

// ---------------------------------------------------------------------------
// Fail-open guards: glue invokes only the CLI and never errors
// ---------------------------------------------------------------------------

fn assert_thin_glue(content: &str, agent: &str) {
    assert!(content.contains("hook"), "missing hook subcommand");
    assert!(content.contains("post"), "missing post subcommand");
    assert!(content.contains("--agent"), "missing --agent flag");
    assert!(content.contains(agent), "missing agent {}", agent);
    assert!(
        content.contains("catch") || content.contains("except"),
        "missing fail-open guard"
    );
}

#[test]
fn glue_templates_are_thin_delegates() {
    let opencode = rgt::hooks::glue::opencode_plugin("/usr/local/bin/rgt");
    let pi = rgt::hooks::glue::pi_extension("/usr/local/bin/rgt");
    let hermes = rgt::hooks::glue::hermes_plugin("/usr/local/bin/rgt");
    assert_thin_glue(&opencode, "opencode");
    assert_thin_glue(&pi, "pi");
    assert_thin_glue(&hermes, "hermes");
    assert!(opencode.contains("timeout: 800"));
    assert!(pi.contains("timeout: 800"));
    assert!(hermes.contains("timeout=0.8"));
}

// ---------------------------------------------------------------------------
// 023: non-text-source capture instruction (T005/T007/T009)
// ---------------------------------------------------------------------------

const NON_TEXT_ANCHOR: &str = "Recording Values When Automatic Capture Is Unavailable";

fn count_anchor(path: &Path, anchor: &str) -> usize {
    match std::fs::read_to_string(path) {
        Ok(content) => content.matches(anchor).count(),
        Err(_) => 0,
    }
}

#[test]
fn instructions_carry_non_text_source_instruction_once() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // codex, windsurf, cline, antigravity, kilocode (project-scoped rules files)
    run(&home, &config_dir, false, true, "codex");
    run(&home, &config_dir, false, true, "windsurf");
    run(&home, &config_dir, false, true, "cline");
    run(&home, &config_dir, false, true, "antigravity");
    run(&home, &config_dir, false, true, "kilocode");
    // copilot CLI and vibe (home-scoped instructions files)
    run(&home, &config_dir, true, true, "copilot");
    run(&home, &config_dir, true, true, "vibe");

    let cases: Vec<(PathBuf, &str)> = vec![
        (PathBuf::from("AGENTS.md"), "codex"),
        (PathBuf::from(".windsurfrules"), "windsurf"),
        (PathBuf::from(".clinerules"), "cline"),
        (
            PathBuf::from(".agents/rules/antigravity-rgt-rules.md"),
            "antigravity",
        ),
        (PathBuf::from(".kilocode/rules/rgt-rules.md"), "kilocode"),
        (
            copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md"),
            "copilot CLI",
        ),
        (home.join(".vibe/AGENTS.md"), "vibe"),
    ];

    for (path, agent) in cases {
        let content = read(&path);
        assert_eq!(
            count_anchor(&path, NON_TEXT_ANCHOR),
            1,
            "{} must contain the non-text instruction exactly once:\n{}",
            agent,
            content
        );
        assert!(
            content.contains("does not prove automatic capture"),
            "{} must distinguish local setup from automatic capture:\n{}",
            agent,
            content
        );
    }
}

#[test]
fn claude_code_writes_claude_md_with_instruction() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Project scope.
    run(&home, &config_dir, false, true, "claude-code");
    let project_md = PathBuf::from("CLAUDE.md");
    assert_eq!(count_anchor(&project_md, NON_TEXT_ANCHOR), 1);

    // Global scope.
    run(&home, &config_dir, true, true, "claude-code");
    let global_md = home.join(".claude").join("CLAUDE.md");
    assert_eq!(count_anchor(&global_md, NON_TEXT_ANCHOR), 1);

    // Idempotent re-run without force.
    run(&home, &config_dir, false, false, "claude-code");
    assert_eq!(count_anchor(&project_md, NON_TEXT_ANCHOR), 1);
}

#[test]
fn claude_code_force_preserves_user_claude_md_content() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    write(
        &PathBuf::from("CLAUDE.md"),
        "# Project memory\nuser notes\n",
    );
    run(&home, &config_dir, false, true, "claude-code");
    let content = read(&PathBuf::from("CLAUDE.md"));
    assert!(
        content.contains("# Project memory\nuser notes\n"),
        "user content must survive:\n{}",
        content
    );
    assert!(content.contains(NON_TEXT_ANCHOR));
}

#[test]
fn unverified_surfaces_get_loadable_guidance_without_automatic_capture_claims() {
    use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth, RegistrationScope};
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    for agent in ["cursor", "gemini", "copilot"] {
        run(&home, &config_dir, true, true, agent);
    }
    for path in [
        PathBuf::from(".cursor/rules/rgt.mdc"),
        home.join(".gemini/GEMINI.md"),
        PathBuf::from(".github/copilot-instructions.md"),
    ] {
        assert_eq!(
            count_anchor(&path, NON_TEXT_ANCHOR),
            1,
            "{}",
            path.display()
        );
    }
    let inspected = inspect_local_guidance(&home, &config_dir);
    for (surface, scope) in [
        ("cursor", RegistrationScope::Project),
        ("gemini", RegistrationScope::User),
        ("copilot-chat", RegistrationScope::Project),
        ("copilot-cli", RegistrationScope::User),
    ] {
        let guidance = inspected.iter().find(|g| g.surface_id == surface).unwrap();
        assert_eq!(guidance.health, GuidanceHealth::Usable, "{surface}");
        assert_eq!(guidance.scope, scope, "{surface}");
    }
    assert!(read(Path::new(".cursor/rules/rgt.mdc")).contains("alwaysApply: true"));
    let cli_md = copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md");
    assert_eq!(
        count_anchor(&cli_md, NON_TEXT_ANCHOR),
        1,
        "Copilot CLI AGENTS.md is in scope"
    );
    let chat_settings = config_dir.join("Code").join("User").join("settings.json");
    assert_eq!(
        count_anchor(&chat_settings, NON_TEXT_ANCHOR),
        0,
        "guidance belongs in the documented project instruction file"
    );
    run(&home, &config_dir, true, true, "cursor");
    assert_eq!(
        count_anchor(Path::new(".cursor/rules/rgt.mdc"), NON_TEXT_ANCHOR),
        1
    );
}

#[test]
fn instruction_only_installers_create_complete_guidance_at_local_load_paths() {
    use rgt::hooks::registration::{inspect_local_guidance, GuidanceHealth};

    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    for agent in ["cline", "antigravity", "kilocode"] {
        assert!(failed(&run(&home, &config_dir, false, true, agent)).is_empty());
    }
    let inspected = inspect_local_guidance(&home, &config_dir);
    for surface in ["cline", "roo-code", "antigravity", "kilocode"] {
        assert_eq!(
            inspected
                .iter()
                .find(|g| g.surface_id == surface)
                .unwrap()
                .health,
            GuidanceHealth::Usable,
            "{surface}"
        );
    }
}

#[test]
fn cursor_does_not_replace_an_unowned_rule_or_write_hooks_on_conflict() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rule = Path::new(".cursor/rules/rgt.mdc");
    write(rule, "---\nalwaysApply: false\n---\nUser rule\n");
    let report = run(&home, &config_dir, false, true, "cursor");
    assert_eq!(failed(&report), vec!["cursor"]);
    assert_eq!(read(rule), "---\nalwaysApply: false\n---\nUser rule\n");
    assert!(!Path::new(".cursor/hooks.json").exists());
}

// ---------------------------------------------------------------------------
// 028 / US4 (T013): restart reminder gating
// ---------------------------------------------------------------------------

#[test]
fn restart_reminder_true_when_any_hook_configured() {
    let report = InstallReport {
        outcomes: vec![AgentOutcome::Configured {
            agent: "codex".into(),
            artifact: PathBuf::from("AGENTS.md"),
            backups: vec![],
        }],
    };
    assert!(rgt::cli::needs_restart_reminder(&report));
}

#[test]
fn restart_reminder_true_when_a_hook_was_migrated() {
    let report = InstallReport {
        outcomes: vec![AgentOutcome::Migrated {
            agent: "codex".into(),
            artifact: PathBuf::from(".codex/hooks.json"),
            backups: vec![PathBuf::from(".codex/hooks.json.rgt.bak")],
        }],
    };
    assert!(rgt::cli::needs_restart_reminder(&report));
}

#[test]
fn restart_reminder_false_when_all_skipped() {
    let report = InstallReport {
        outcomes: vec![
            AgentOutcome::AlreadyCurrent {
                agent: "codex".into(),
                artifact: PathBuf::from("hooks.json"),
            },
            AgentOutcome::AlreadyCurrent {
                agent: "cursor".into(),
                artifact: PathBuf::from("hooks.json"),
            },
        ],
    };
    assert!(!rgt::cli::needs_restart_reminder(&report));
}

#[test]
fn restart_reminder_false_when_empty_report() {
    let report = InstallReport::default();
    assert!(!rgt::cli::needs_restart_reminder(&report));
}
