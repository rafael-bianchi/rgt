//! Agent hook installer and registry.
//!
//! Registers the full 13-agent RTK-parity set (Claude Code, Cursor, Codex CLI,
//! Windsurf, Copilot, Gemini, Mistral Vibe, OpenCode, Pi, Hermes, Cline/Roo
//! Code, Antigravity, Kilo) with canonical names, aliases, and integration
//! tiers, writes each agent's native hook config/plugin/rules artifact at
//! `rgt init` time, and auto-detects installed agents for one-command setup.

use crate::hooks::glue;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct DetectedTool {
    pub name: String,
    pub config_path: PathBuf,
    pub is_configured: bool,
}

/// Integration tier for a supported agent (see `data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Agent invokes `rgt hook --agent` directly via its native hook API.
    Hook,
    /// Thin TS/Python glue plugin written by the installer.
    Plugin,
    /// Prompt-level rules/instruction file, no programmatic hook.
    Rules,
}

/// Static registry entry for one supported agent.
pub struct AgentDef {
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
    /// Integration tier (metadata; writers implement tier behavior directly).
    #[allow(dead_code)]
    pub tier: Tier,
}

/// The 13-agent registry (full RTK parity): canonical name, accepted aliases,
/// and integration tier. Alias resolution happens via [`resolve_agent_name`].
pub const AGENTS: &[AgentDef] = &[
    AgentDef {
        canonical: "claude-code",
        aliases: &["claude"],
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "cursor",
        aliases: &[],
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "codex",
        aliases: &[],
        tier: Tier::Rules,
    },
    AgentDef {
        canonical: "windsurf",
        aliases: &[],
        tier: Tier::Rules,
    },
    AgentDef {
        canonical: "copilot",
        aliases: &[],
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "gemini",
        aliases: &[],
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "vibe",
        aliases: &[],
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "opencode",
        aliases: &[],
        tier: Tier::Plugin,
    },
    AgentDef {
        canonical: "pi",
        aliases: &[],
        tier: Tier::Plugin,
    },
    AgentDef {
        canonical: "hermes",
        aliases: &[],
        tier: Tier::Plugin,
    },
    AgentDef {
        canonical: "cline",
        aliases: &["roo-code"],
        tier: Tier::Rules,
    },
    AgentDef {
        canonical: "antigravity",
        aliases: &[],
        tier: Tier::Rules,
    },
    AgentDef {
        canonical: "kilocode",
        aliases: &["kilo"],
        tier: Tier::Rules,
    },
];

/// Resolves an `--agent` spelling (canonical or alias) to its canonical name.
/// Returns `None` for unknown names (FR-001: rejected by the CLI with exit 2).
pub fn resolve_agent_name(name: &str) -> Option<String> {
    for def in AGENTS {
        if def.canonical == name || def.aliases.contains(&name) {
            return Some(def.canonical.to_string());
        }
    }
    None
}

/// Whether `name` is an accepted `--agent` spelling.
/// All accepted `--agent` spellings (13 canonical + 3 aliases) for error
/// messages and documentation parity (SC-005).
pub fn valid_agent_names() -> Vec<&'static str> {
    crate::hooks::paths::ALL_AGENT_NAMES.to_vec()
}

/// Detects installed AI coding tools and configures their hooks. Supports the
/// full 13-agent RTK set, global installs, `--force` overwrite, and per-agent
/// targeting. Without `--agent`, only agents whose detection trigger is present
/// are configured (US2).
/// # Returns
/// A list of human-readable strings describing each configured agent+config file.
pub fn detect_and_configure_hooks(
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> io::Result<Vec<String>> {
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;
    detect_and_configure_hooks_in_home(&home, global, force, agent)
}

/// Same as [`detect_and_configure_hooks`], but resolves the home directory from an
/// explicit path. Exposed for testability so tests can isolate global hook configs
/// to a temp directory on any platform (the `dirs` crate resolves home via the
/// Windows Shell API, which ignores environment variables).
pub fn detect_and_configure_hooks_in_home(
    home: &Path,
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> io::Result<Vec<String>> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
    configure(home, &config_dir, global, force, agent)
}

/// Same as [`detect_and_configure_hooks_in_home`], with an explicit config
/// directory override (for cross-platform path tests of VS Code/Copilot).
/// Test-support only; the binary resolves both directories from `dirs`.
#[allow(dead_code)]
pub fn detect_and_configure_hooks_with_config(
    home: &Path,
    config_dir: &Path,
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> io::Result<Vec<String>> {
    configure(home, config_dir, global, force, agent)
}

fn configure(
    home: &Path,
    config_dir: &Path,
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> io::Result<Vec<String>> {
    let mut configured = Vec::new();

    // Resolve the targeted canonical agent. Unknown names configure nothing;
    // the CLI layer enforces FR-001 (exit code 2 + valid-name list).
    let target = match agent {
        None => None,
        Some(name) => resolve_agent_name(name),
    };
    if agent.is_some() && target.is_none() {
        return Ok(configured);
    }

    // Without `--agent`, configure only agents detected as installed (US2).
    let targets: Vec<String> = match &target {
        Some(canonical) => vec![canonical.clone()],
        None => detect_installed_agents(home, config_dir),
    };
    let wants = |name: &str| targets.iter().any(|t| t == name);

    // ---- Existing 4 integrations (byte-identical baseline, SC-003) ----

    // 1. Claude Code (~/.claude/settings.json)
    if wants("claude-code") {
        write_claude_code(home, global, force, &mut configured)?;
    }

    // 2. Cursor (~/.cursor/hooks.json)
    if wants("cursor") {
        write_cursor(home, global, force, &mut configured)?;
    }

    // 3. Codex CLI (AGENTS.md integration)
    if wants("codex") {
        write_codex(force, &mut configured)?;
    }

    // 4. Windsurf (.windsurfrules)
    if wants("windsurf") {
        write_windsurf(force, &mut configured)?;
    }

    // ---- New agents (US1) ----

    if wants("copilot") {
        write_copilot(home, config_dir, force, &mut configured)?;
    }
    if wants("gemini") {
        write_gemini(home, force, &mut configured)?;
    }
    if wants("vibe") {
        write_vibe(home, force, &mut configured)?;
    }
    if wants("opencode") {
        write_opencode(home, global, force, &mut configured)?;
    }
    if wants("pi") {
        write_pi(home, global, force, &mut configured)?;
    }
    if wants("hermes") {
        write_hermes(home, global, force, &mut configured)?;
    }
    if wants("cline") {
        write_cline(force, &mut configured)?;
    }
    if wants("antigravity") {
        write_antigravity(force, &mut configured)?;
    }
    if wants("kilocode") {
        write_kilocode(force, &mut configured)?;
    }

    Ok(configured)
}

// ---------------------------------------------------------------------------
// Existing integrations (SC-003: byte-identical output)
// ---------------------------------------------------------------------------

fn write_claude_code(
    home: &Path,
    global: bool,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let claude_dir = if global {
        home.join(".claude")
    } else {
        PathBuf::from(".claude")
    };
    let settings_file = claude_dir.join("settings.json");
    let old_hooks_file = claude_dir.join("hooks.json");

    if force || !settings_file.exists() {
        if let Some(parent) = settings_file.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut existing: serde_json::Value = if settings_file.exists() {
            let content = fs::read_to_string(&settings_file).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let rgt_hooks = serde_json::json!({
            "PostToolUse": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "rgt hook post"
                        }
                    ]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "rgt hook pre"
                        }
                    ]
                }
            ]
        });

        existing["hooks"] = rgt_hooks;
        fs::write(&settings_file, serde_json::to_string_pretty(&existing)?)?;
        configured.push(format!(
            "Claude Code (full hook) -> {}",
            settings_file.display()
        ));

        if force && old_hooks_file.exists() {
            let _ = fs::remove_file(&old_hooks_file);
        }
    }
    Ok(())
}

fn write_cursor(
    home: &Path,
    global: bool,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let cursor_dir = if global {
        home.join(".cursor")
    } else {
        PathBuf::from(".cursor")
    };
    let cursor_hooks_file = cursor_dir.join("hooks.json");
    if force || !cursor_hooks_file.exists() {
        if let Some(parent) = cursor_hooks_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = serde_json::json!({
            "version": 1,
            "hooks": {
                "preToolUse": [
                    {
                        "command": "rgt hook pre",
                        "matcher": "Shell"
                    }
                ],
                "postToolUse": [
                    {
                        "command": "rgt hook post",
                        "matcher": "Shell"
                    }
                ]
            }
        });
        fs::write(&cursor_hooks_file, serde_json::to_string_pretty(&config)?)?;
        configured.push(format!(
            "Cursor (full hook) -> {}",
            cursor_hooks_file.display()
        ));
    }
    Ok(())
}

fn codex_section() -> String {
    "\n## RGT Integration\n\nRGT tracks numeric and date provenance for AI coding agents.\n\n### Recording Values\nAfter reading a data file containing numbers or dates, record its values:\n- `rgt record <file>`: extracts and tracks numeric/date values from file content\n- Run `rgt status` to check what's tracked\n\n### Recording Derivations\nAfter computing a derived value from tracked root nodes:\n- `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n- The derivation is verified before recording; wrong results are rejected (exit 1)\n\n### Inspecting the Graph\n- `rgt status`: see all tracked nodes and staleness state\n- `rgt query <node_id>`: trace provenance lineage for a value\n- `rgt graph`: export the dependency graph as text or mermaid\n\nRun `rgt --help` for all available commands.\n"
        .to_string()
}

fn write_codex(force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let agents_file = PathBuf::from("AGENTS.md");

    let write_file = force
        || !agents_file.exists()
        || !fs::read_to_string(&agents_file)
            .unwrap_or_default()
            .contains("## RGT Integration");

    if write_file {
        if force || !agents_file.exists() {
            fs::write(
                &agents_file,
                format!("# RGT Agents Instructions\n{}", codex_section()),
            )?;
        } else {
            use std::io::Write;
            let mut file = fs::OpenOptions::new().append(true).open(&agents_file)?;
            file.write_all(codex_section().as_bytes())?;
        }
    }

    configured.push(format!(
        "Codex CLI (rules-file) -> {}",
        agents_file.display()
    ));
    Ok(())
}

fn write_windsurf(force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let rules_file = PathBuf::from(".windsurfrules");
    let rules_content = "# RGT Integration\n\
        RGT tracks numeric and date provenance for AI coding agents.\n\
        After reading data files, record extracted values with `rgt record <file>`.\n\
        After computing derived values, record them with `rgt derive --parents <ids> --operation EXPRESSION --expression \"a - b\" --result <val>`.\n\
        Derivations are verified before recording; wrong results are rejected.\n\
        Use `rgt status` to check provenance graph state.\n\
        Run `rgt --help` for all available commands.\n";

    if force || !rules_file.exists() {
        fs::write(&rules_file, rules_content)?;
        configured.push(format!("Windsurf (rules-file) -> {}", rules_file.display()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// New integrations (US1): hook-tier agents
// ---------------------------------------------------------------------------

/// Copilot: VS Code Copilot Chat user settings `github.copilot.chat.hooks`
/// plus a Copilot CLI instructions file under the CLI's config directory.
fn write_copilot(
    home: &Path,
    config_dir: &Path,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let settings_file = crate::hooks::paths::copilot_user_settings(config_dir);
    let hooks = serde_json::json!({
        "PostToolUse": [
            {
                "matcher": ".*",
                "hooks": [
                    { "type": "command", "command": "rgt hook post --agent copilot" }
                ]
            }
        ]
    });

    let written = write_json_key(&settings_file, "github.copilot.chat.hooks", &hooks, force)?;
    if written {
        configured.push(format!(
            "Copilot (full hook) -> {}",
            settings_file.display()
        ));
    }

    // Copilot CLI: instructions file at the CLI's user config directory.
    let cli_rules_file =
        crate::hooks::paths::copilot_cli_config_dir(home, config_dir).join("AGENTS.md");
    if write_text_block(
        &cli_rules_file,
        glue::RGT_MARKER,
        glue::COPILOT_CLI_RULES,
        force,
    )? {
        configured.push(format!(
            "Copilot CLI (rules-file) -> {}",
            cli_rules_file.display()
        ));
    }
    Ok(())
}

/// Gemini CLI: `~/.gemini/hooks.toml` PostToolUse entry.
fn write_gemini(home: &Path, force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let hooks_file = home.join(".gemini").join("hooks.toml");
    let block = "\
        # RGT Integration\n\
        [PostToolUse]\n\
        command = \"rgt hook post --agent gemini\"\n";

    if write_toml_block(&hooks_file, "rgt hook post --agent gemini", block, force)? {
        configured.push(format!("Gemini (full hook) -> {}", hooks_file.display()));
    }
    Ok(())
}

/// Mistral Vibe: `~/.vibe/hooks.toml` pre_tool entry + `~/.vibe/prompts/rgt.md`.
fn write_vibe(home: &Path, force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let hooks_file = home.join(".vibe").join("hooks.toml");
    let block = "\
        # RGT Integration\n\
        [[pre_tool]]\n\
        match = \"bash\"\n\
        strict = false\n\
        command = \"rgt hook post --agent vibe\"\n";

    let written_hooks = write_toml_block(&hooks_file, "rgt hook post --agent vibe", block, force)?;
    let prompt_file = home.join(".vibe").join("prompts").join("rgt.md");
    let written_prompt =
        write_text_block(&prompt_file, glue::RGT_MARKER, glue::VIBE_PROMPT, force)?;

    if written_hooks || written_prompt {
        configured.push(format!(
            "Mistral Vibe (full hook) -> {}",
            hooks_file.display()
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// New integrations (US1): plugin-tier agents
// ---------------------------------------------------------------------------

/// OpenCode: embedded `rgt.ts` TS plugin.
fn write_opencode(
    home: &Path,
    global: bool,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let plugin_dir = if global {
        home.join(".config").join("opencode").join("plugins")
    } else {
        PathBuf::from(".opencode").join("plugin")
    };
    let plugin_file = plugin_dir.join("rgt.ts");
    if write_plugin_file(&plugin_file, glue::OPENCODE_TS_PLUGIN, force)? {
        configured.push(format!("OpenCode (plugin) -> {}", plugin_file.display()));
    }
    Ok(())
}

/// Pi: embedded `rgt.ts` TS extension.
fn write_pi(
    home: &Path,
    global: bool,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let ext_dir = if global {
        home.join(".pi").join("agent").join("extensions")
    } else {
        PathBuf::from(".pi").join("extensions")
    };
    let ext_file = ext_dir.join("rgt.ts");
    if write_plugin_file(&ext_file, glue::PI_TS_EXTENSION, force)? {
        configured.push(format!("Pi (plugin) -> {}", ext_file.display()));
    }
    Ok(())
}

/// Hermes: embedded `plugin.py` + `plugins.enabled` in the Hermes config.
fn write_hermes(
    home: &Path,
    global: bool,
    force: bool,
    configured: &mut Vec<String>,
) -> io::Result<()> {
    let plugin_dir = if global {
        home.join(".hermes").join("plugins").join("rgt")
    } else {
        PathBuf::from(".hermes").join("plugins").join("rgt")
    };
    let plugin_file = plugin_dir.join("plugin.py");
    let written_plugin = write_plugin_file(&plugin_file, glue::HERMES_PYTHON_PLUGIN, force)?;

    let config_file = if global {
        home.join(".hermes").join("config.toml")
    } else {
        PathBuf::from(".hermes").join("config.toml")
    };
    let written_config = enable_hermes_plugin(&config_file, force)?;

    if written_plugin || written_config {
        configured.push(format!("Hermes (plugin) -> {}", plugin_file.display()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// New integrations (US1): rules-tier agents
// ---------------------------------------------------------------------------

/// Cline / Roo Code: append RGT section to `.clinerules`.
fn write_cline(force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let rules_file = PathBuf::from(".clinerules");
    if write_text_block(&rules_file, glue::RGT_MARKER, glue::CLINE_RULES, force)? {
        configured.push(format!("Cline (rules-file) -> {}", rules_file.display()));
    }
    Ok(())
}

/// Antigravity: rules file under `.agents/rules/`.
fn write_antigravity(force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let rules_file = PathBuf::from(".agents")
        .join("rules")
        .join("antigravity-rgt-rules.md");
    if write_text_block(
        &rules_file,
        glue::RGT_MARKER,
        glue::ANTIGRAVITY_RULES,
        force,
    )? {
        configured.push(format!(
            "Antigravity (rules-file) -> {}",
            rules_file.display()
        ));
    }
    Ok(())
}

/// Kilo: rules file under `.kilocode/rules/`.
fn write_kilocode(force: bool, configured: &mut Vec<String>) -> io::Result<()> {
    let rules_file = PathBuf::from(".kilocode")
        .join("rules")
        .join("rgt-rules.md");
    if write_text_block(&rules_file, glue::RGT_MARKER, glue::KILO_RULES, force)? {
        configured.push(format!("Kilo (rules-file) -> {}", rules_file.display()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Idempotent write helpers (FR-006)
// ---------------------------------------------------------------------------

/// Writes a top-level JSON key into a settings file, merging user-authored
/// content. Idempotent: re-runs with the key present are skipped unless
/// `--force`. Returns whether a write occurred.
fn write_json_key(
    file: &Path,
    key: &str,
    value: &serde_json::Value,
    force: bool,
) -> io::Result<bool> {
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let existing: serde_json::Value = if file.exists() {
        fs::read_to_string(file)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if existing.get(key).is_some() && !force {
        return Ok(false);
    }

    let mut updated = existing;
    updated[key] = value.clone();
    fs::write(file, serde_json::to_string_pretty(&updated)?)?;
    Ok(true)
}

/// Writes/appends an RGT-marked TOML block. Idempotent: skipped when the file
/// already contains `marker` unless `--force` (which replaces the marked block
/// in place). Returns whether a write occurred.
fn write_toml_block(file: &Path, marker: &str, block: &str, force: bool) -> io::Result<bool> {
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if !file.exists() {
        fs::write(file, block)?;
        return Ok(true);
    }

    let existing = fs::read_to_string(file)?;
    if existing.contains(marker) {
        if force {
            let base = truncate_from_marker(&existing, marker);
            fs::write(file, join_base_block(&base, block))?;
            return Ok(true);
        }
        return Ok(false);
    }

    let mut out = existing.trim_end().to_string();
    out.push('\n');
    out.push_str(block);
    fs::write(file, out)?;
    Ok(true)
}

/// Writes/appends an RGT-marked text block (rules files, prompts). Idempotent:
/// skipped when the file already contains `marker` unless `--force` (which
/// replaces the marked block in place). Returns whether a write occurred.
fn write_text_block(file: &Path, marker: &str, block: &str, force: bool) -> io::Result<bool> {
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if !file.exists() {
        fs::write(file, block)?;
        return Ok(true);
    }

    let existing = fs::read_to_string(file)?;
    if existing.contains(marker) {
        if force {
            let base = truncate_from_marker(&existing, marker);
            fs::write(file, join_base_block(&base, block))?;
            return Ok(true);
        }
        return Ok(false);
    }

    let mut out = existing.trim_end().to_string();
    out.push('\n');
    out.push_str(block);
    fs::write(file, out)?;
    Ok(true)
}

/// Writes a standalone plugin file. Idempotent: skipped when present unless
/// `--force`. Returns whether a write occurred.
fn write_plugin_file(file: &Path, content: &str, force: bool) -> io::Result<bool> {
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if file.exists() && !force {
        return Ok(false);
    }
    fs::write(file, content)?;
    Ok(true)
}

/// Ensures `rgt` is listed in the Hermes config `plugins.enabled`. Creates the
/// config when absent; appends `[plugins]` when no such key exists. Idempotent.
fn enable_hermes_plugin(file: &Path, _force: bool) -> io::Result<bool> {
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if !file.exists() {
        fs::write(file, "[plugins]\nenabled = [\"rgt\"]\n")?;
        return Ok(true);
    }

    let existing = fs::read_to_string(file)?;
    if existing.contains("\"rgt\"") {
        return Ok(false); // idempotent skip
    }

    let mut out = existing.trim_end().to_string();
    if existing.contains("plugins.enabled") {
        // Insert into the existing enabled array.
        out = insert_into_enabled(&out);
        out.push('\n');
    } else {
        out.push_str("\n\n[plugins]\nenabled = [\"rgt\"]\n");
    }
    fs::write(file, out)?;
    Ok(true)
}

/// Inserts `"rgt"` into the first `enabled = [...]` array in a TOML config.
fn insert_into_enabled(content: &str) -> String {
    const ARRAY_MARKER: &str = "enabled = [";
    match content.find(ARRAY_MARKER) {
        Some(idx) => {
            let (before, rest) = content.split_at(idx + ARRAY_MARKER.len());
            let mut out = before.to_string();
            out.push_str("\"rgt\", ");
            out.push_str(rest);
            out
        }
        None => {
            let mut out = content.to_string();
            out.push_str("\n[plugins]\nenabled = [\"rgt\"]\n");
            out
        }
    }
}

/// Truncates `content` at the first line containing `marker`, returning the
/// text before it (trimmed of trailing whitespace) so an RGT-marked block can
/// be replaced in place. Returns the whole trimmed content when no marker.
fn truncate_from_marker(content: &str, marker: &str) -> String {
    match content.lines().position(|l| l.contains(marker)) {
        Some(idx) => content
            .lines()
            .take(idx)
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string(),
        None => content.trim_end().to_string(),
    }
}

/// Joins a (possibly empty) prefix and an RGT-marked block with a single
/// separating newline.
fn join_base_block(base: &str, block: &str) -> String {
    if base.trim().is_empty() {
        block.to_string()
    } else {
        format!("{}\n{}", base, block)
    }
}

// ---------------------------------------------------------------------------
// Auto-detection (US2, FR-003)
// ---------------------------------------------------------------------------

/// Returns the canonical names of supported agents detected as installed, per
/// `data-model.md` `detection_trigger`. Both host-level and project-level
/// trigger paths are checked; either indicates the agent is installed (US2).
pub fn detect_installed_agents(home: &Path, config_dir: &Path) -> Vec<String> {
    let mut detected = Vec::new();

    let home_dir = |name: &str| home.join(name);
    let project = |name: &str| PathBuf::from(name);

    for def in AGENTS {
        let trigger_home = match def.canonical {
            "claude-code" => home_dir(".claude"),
            "cursor" => home_dir(".cursor"),
            "codex" => home_dir(".codex"),
            "windsurf" => home_dir(".windsurf"),
            "copilot" => crate::hooks::paths::copilot_user_settings(config_dir),
            "gemini" => home_dir(".gemini"),
            "vibe" => home_dir(".vibe"),
            "opencode" => home_dir(".config").join("opencode"),
            "pi" => home_dir(".pi"),
            "hermes" => home_dir(".hermes"),
            "cline" => home_dir(".cline"),
            "antigravity" => home_dir(".agents"),
            "kilocode" => home_dir(".kilocode"),
            _ => home_dir(""),
        };
        let trigger_project = match def.canonical {
            "claude-code" => project(".claude"),
            "cursor" => project(".cursor"),
            "codex" => project("AGENTS.md"),
            "windsurf" => project(".windsurfrules"),
            "opencode" => project(".opencode"),
            "pi" => project(".pi"),
            "hermes" => project(".hermes"),
            "cline" => project(".clinerules"),
            "antigravity" => project(".agents"),
            "kilocode" => project(".kilocode"),
            _ => project(""),
        };

        let mut present = trigger_home.exists() || trigger_project.exists();
        // Copilot is installed if either VS Code Chat or the Copilot CLI
        // config directory exists (T010: detect both surfaces).
        if def.canonical == "copilot" {
            present =
                present || crate::hooks::paths::copilot_cli_config_dir(home, config_dir).exists();
        }
        if present {
            detected.push(def.canonical.to_string());
        }
    }

    detected
}
