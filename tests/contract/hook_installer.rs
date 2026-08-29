use rgt::hooks::installer::{
    detect_and_configure_hooks_with_config, resolve_agent_name, valid_agent_names, AgentOutcome,
    InstallReport,
};
use rgt::hooks::paths::copilot_cli_config_dir;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::tempdir;

static CWD_MUTEX: Mutex<()> = Mutex::new(());

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
    detect_and_configure_hooks_with_config(home, config_dir, global, force, Some(agent)).unwrap()
}

fn run_none(home: &Path, config_dir: &Path, global: bool, force: bool) -> InstallReport {
    detect_and_configure_hooks_with_config(home, config_dir, global, force, None).unwrap()
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

fn failed(report: &InstallReport) -> Vec<String> {
    report
        .outcomes
        .iter()
        .filter_map(|o| match o {
            AgentOutcome::Failed { agent, .. } => Some(agent.clone()),
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
    assert!(content.contains("\"rgt hook post\""));
    assert!(content.contains("\"rgt hook pre\""));
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
    assert!(content.contains("rgt hook post"));
    assert!(content.contains("rgt hook pre"));
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
    assert_eq!(
        hooks["hooks"][0]["command"],
        "rgt hook post --agent copilot"
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
    assert!(claude.contains("\"rgt hook post\""));
    assert!(claude.contains("\"rgt hook pre\""));
    serde_json::from_str::<serde_json::Value>(&claude).unwrap();

    let cursor = read(&home.join(".cursor").join("hooks.json"));
    assert!(cursor.contains("rgt hook post"));
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
    assert!(toml.contains("rgt hook post --agent gemini"));
    let _: toml_edit::DocumentMut = toml.parse().unwrap();
}

#[test]
fn vibe_writer_creates_hooks_toml_and_prompt() {
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
    assert!(toml.contains("rgt hook post --agent vibe"));
    let _: toml_edit::DocumentMut = toml.parse().unwrap();

    let prompt = read(&home.join(".vibe").join("prompts").join("rgt.md"));
    assert!(prompt.contains("RGT Integration"));
    assert!(prompt.contains("<!-- /RGT Integration -->"));
}

#[test]
fn opencode_writer_project_and_global() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, false, true, "opencode");
    let project_plugin = read(&PathBuf::from(".opencode").join("plugin").join("rgt.ts"));
    assert_thin_glue(&project_plugin, "opencode");

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
            AgentOutcome::Failed { agent, reason } if agent == "claude-code" => {
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
            AgentOutcome::Failed { agent, reason } if agent == "cline" => Some(reason.clone()),
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
fn legacy_block_without_force_is_skipped_not_modified() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules_file = PathBuf::from(".clinerules");

    let legacy = "## RGT Integration\nSTALE CONTENT\n";
    write(&rules_file, legacy);

    let report = run(&home, &config_dir, false, false, "cline");
    assert!(skipped(&report).contains(&"cline".to_string()));
    assert_eq!(read(&rules_file), legacy);
}

#[test]
fn plugin_file_not_overwritten_without_force() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let plugin_file = PathBuf::from(".opencode").join("plugin").join("rgt.ts");

    write(&plugin_file, "// user-authored plugin\n");
    let report = run(&home, &config_dir, false, false, "opencode");
    assert!(skipped(&report).contains(&"opencode".to_string()));
    assert_eq!(read(&plugin_file), "// user-authored plugin\n");

    // --force overwrites.
    run(&home, &config_dir, false, true, "opencode");
    assert_thin_glue(&read(&plugin_file), "opencode");
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

    let report =
        detect_and_configure_hooks_with_config(&home, &config_dir, true, true, Some("nope"))
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
    assert_thin_glue(rgt::hooks::glue::OPENCODE_TS_PLUGIN, "opencode");
    assert_thin_glue(rgt::hooks::glue::PI_TS_EXTENSION, "pi");
    assert_thin_glue(rgt::hooks::glue::HERMES_PYTHON_PLUGIN, "hermes");
}
