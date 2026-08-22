use rgt::hooks::installer::{
    detect_and_configure_hooks_with_config, resolve_agent_name, valid_agent_names,
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

fn run(home: &Path, config_dir: &Path, global: bool, force: bool, agent: &str) -> Vec<String> {
    detect_and_configure_hooks_with_config(home, config_dir, global, force, Some(agent)).unwrap()
}

fn run_none(home: &Path, config_dir: &Path, global: bool, force: bool) -> Vec<String> {
    detect_and_configure_hooks_with_config(home, config_dir, global, force, None).unwrap()
}

// ---------------------------------------------------------------------------
// SC-003: existing 4 integrations remain byte-identical to the Phase 1 baseline
// ---------------------------------------------------------------------------

const CLAUDE_BASELINE: &str = "{\n  \"hooks\": {\n    \"PostToolUse\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"rgt hook post\",\n            \"type\": \"command\"\n          }\n        ],\n        \"matcher\": \"\"\n      }\n    ],\n    \"PreToolUse\": [\n      {\n        \"hooks\": [\n          {\n            \"command\": \"rgt hook pre\",\n            \"type\": \"command\"\n          }\n        ],\n        \"matcher\": \"\"\n      }\n    ]\n  }\n}";

const CURSOR_BASELINE: &str = "{\n  \"hooks\": {\n    \"postToolUse\": [\n      {\n        \"command\": \"rgt hook post\",\n        \"matcher\": \"Shell\"\n      }\n    ],\n    \"preToolUse\": [\n      {\n        \"command\": \"rgt hook pre\",\n        \"matcher\": \"Shell\"\n      }\n    ]\n  },\n  \"version\": 1\n}";

#[test]
fn existing_four_agents_byte_identical_sc003() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    run(&home, &config_dir, true, true, "claude-code");
    run(&home, &config_dir, true, true, "cursor");
    run(&home, &config_dir, true, true, "codex");
    run(&home, &config_dir, true, true, "windsurf");

    assert_eq!(
        read(&home.join(".claude").join("settings.json")),
        CLAUDE_BASELINE
    );
    assert_eq!(
        read(&home.join(".cursor").join("hooks.json")),
        CURSOR_BASELINE
    );

    let agents_md = read(&PathBuf::from("AGENTS.md"));
    assert!(agents_md.starts_with("# RGT Agents Instructions\n\n## RGT Integration"));
    assert!(agents_md.contains("rgt record <file>"));
    assert!(agents_md.contains("rgt derive --parents"));
    assert!(agents_md
        .trim_end()
        .ends_with("Run `rgt --help` for all available commands."));

    let windsurf = read(&PathBuf::from(".windsurfrules"));
    assert!(windsurf.starts_with("# RGT Integration\nRGT tracks numeric and date provenance"));
    assert!(windsurf.contains("rgt derive --parents"));
    assert!(windsurf.contains("rgt --help"));
}

// ---------------------------------------------------------------------------
// New writers (US1)
// ---------------------------------------------------------------------------

#[test]
fn copilot_writer_merges_chat_hooks_into_user_settings() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Pre-seed the user settings file with user-authored content.
    let settings = config_dir.join("Code").join("User").join("settings.json");
    write(&settings, "{\n  \"editor.fontSize\": 14\n}\n");

    let result = run(&home, &config_dir, true, true, "copilot");
    assert!(!result.is_empty());
    assert!(result.iter().any(|r| r.contains("Copilot")));

    let content = read(&settings);
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["editor.fontSize"], 14); // user content preserved
    let hooks = &parsed["github.copilot.chat.hooks"]["PostToolUse"][0];
    assert_eq!(
        hooks["hooks"][0]["command"],
        "rgt hook post --agent copilot"
    );
}

#[test]
fn copilot_writer_creates_cli_rules_file() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let result = run(&home, &config_dir, true, true, "copilot");
    assert!(result.iter().any(|r| r.contains("Copilot CLI")));

    let rules = read(&copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md"));
    assert!(rules.contains("## RGT Integration"));
    assert!(rules.contains("rgt record <file>"));

    // Idempotent re-run: no duplicate section.
    run(&home, &config_dir, true, false, "copilot");
    let second = read(&copilot_cli_config_dir(&home, &config_dir).join("AGENTS.md"));
    assert_eq!(rules, second);
    assert_eq!(second.matches("## RGT Integration").count(), 1);
}

#[test]
fn gemini_writer_creates_hooks_toml() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let result = run(&home, &config_dir, true, true, "gemini");
    assert_eq!(result.len(), 1);

    let toml = read(&home.join(".gemini").join("hooks.toml"));
    assert!(toml.contains("[PostToolUse]"));
    assert!(toml.contains("rgt hook post --agent gemini"));
}

#[test]
fn vibe_writer_creates_hooks_toml_and_prompt() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let result = run(&home, &config_dir, true, true, "vibe");
    assert_eq!(result.len(), 1);

    let toml = read(&home.join(".vibe").join("hooks.toml"));
    assert!(toml.contains("[[pre_tool]]"));
    assert!(toml.contains("match = \"bash\""));
    assert!(toml.contains("strict = false"));
    assert!(toml.contains("rgt hook post --agent vibe"));

    let prompt = read(&home.join(".vibe").join("prompts").join("rgt.md"));
    assert!(prompt.contains("RGT Integration"));
}

#[test]
fn opencode_writer_project_and_global() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // Project scope
    let result = run(&home, &config_dir, false, true, "opencode");
    assert_eq!(result.len(), 1);
    let project_plugin = read(&PathBuf::from(".opencode").join("plugin").join("rgt.ts"));
    assert_thin_glue(&project_plugin, "opencode");

    // Global scope
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

    let result = run(&home, &config_dir, true, true, "hermes");
    assert_eq!(result.len(), 1);

    let plugin = read(
        &home
            .join(".hermes")
            .join("plugins")
            .join("rgt")
            .join("plugin.py"),
    );
    assert_thin_glue(&plugin, "hermes");
    assert!(plugin.contains("shutil.which"));

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

    run(&home, &config_dir, false, true, "kilocode");
    let kilo = read(
        &PathBuf::from(".kilocode")
            .join("rules")
            .join("rgt-rules.md"),
    );
    assert!(kilo.contains("RGT Integration"));
}

// ---------------------------------------------------------------------------
// Idempotency and --force semantics (FR-006)
// ---------------------------------------------------------------------------

#[test]
fn rerun_without_force_is_idempotent() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    // First run (force) creates artifacts.
    run(&home, &config_dir, true, true, "gemini");
    let first = read(&home.join(".gemini").join("hooks.toml"));

    // Second run without force: nothing written, file unchanged.
    let result = run(&home, &config_dir, true, false, "gemini");
    assert!(result.is_empty());
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

    // Re-run without force: no duplicate section.
    run(&home, &config_dir, false, false, "cline");
    assert_eq!(read(&PathBuf::from(".clinerules")), first);
    assert_eq!(first.matches("## RGT Integration").count(), 1);
}

#[test]
fn force_replaces_rgt_block_preserving_user_content() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let rules_file = PathBuf::from(".clinerules");

    // Seed a user file with a stale RGT block appended after user content.
    write(
        &rules_file,
        "# User rules\n# More user rules\n## RGT Integration\nSTALE CONTENT\n",
    );
    run(&home, &config_dir, false, true, "cline");
    let content = read(&rules_file);
    assert!(content.starts_with("# User rules\n"));
    assert!(content.contains("# More user rules\n"));
    assert!(!content.contains("STALE CONTENT"));
    assert_eq!(content.matches("## RGT Integration").count(), 1);
    assert!(content.contains("rgt record <file>"));
}

#[test]
fn plugin_file_not_overwritten_without_force() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");
    let plugin_file = PathBuf::from(".opencode").join("plugin").join("rgt.ts");

    write(&plugin_file, "// user-authored plugin\n");
    let result = run(&home, &config_dir, false, false, "opencode");
    assert!(result.is_empty());
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

    let result = run_none(&home, &config_dir, false, true);
    assert!(result.iter().any(|r| r.contains("Claude Code")));
    assert!(result.iter().any(|r| r.contains("Mistral Vibe")));
    assert!(result.iter().any(|r| r.contains("Kilo")));
    assert!(result.iter().any(|r| r.contains("Cline")));
    assert!(result.iter().any(|r| r.contains("Copilot")));
}

#[test]
fn init_no_agent_with_nothing_detected_reports_empty() {
    let dir = tempdir().unwrap();
    let _guard = set_cwd(dir.path());
    let home = dir.path().join("home");
    let config_dir = dir.path().join("config");

    let result = run_none(&home, &config_dir, false, true);
    assert!(result.is_empty());
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

    let result =
        detect_and_configure_hooks_with_config(&home, &config_dir, true, true, Some("nope"))
            .unwrap();
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Fail-open guards (T026 / T027): glue invokes only the CLI and never errors
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
