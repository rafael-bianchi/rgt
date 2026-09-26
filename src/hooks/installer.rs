//! Agent hook installer and registry.
//!
//! Registers the full 13-agent RTK-parity set (Claude Code, Cursor, Codex CLI,
//! Windsurf, Copilot, Gemini, Mistral Vibe, OpenCode, Pi, Hermes, Cline/Roo
//! Code, Antigravity, Kilo) with canonical names, aliases, and integration
//! tiers, writes each agent's native hook config/plugin/rules artifact at
//! `rgt init` time, and auto-detects installed agents for one-command setup.
//!
//! Every writer here preserves existing user configuration (spec FR-001/FR-002):
//! JSON/JSONC configs are merged via [`crate::hooks::editor`]'s byte-for-byte
//! splice, TOML configs via `toml_edit`, and text/rules blocks are replaced only
//! between paired markers. Unparsable or unmergeable files are reported loudly
//! (FR-003/FR-009) and per-agent failures never abort the other agents
//! (FR-008).

use crate::hooks::editor::{self, ConfigEditError};
use crate::hooks::glue;
use serde_json::json;
use std::fs;
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

/// Whether an already-canonical agent has an event-capable hook or plugin
/// integration and can therefore be attached to a captured value.
pub fn is_capture_capable_agent(name: &str) -> bool {
    AGENTS
        .iter()
        .any(|agent| agent.canonical == name && matches!(agent.tier, Tier::Hook | Tier::Plugin))
}

/// Whether `name` is an accepted `--agent` spelling.
/// All accepted `--agent` spellings (13 canonical + 3 aliases) for error
/// messages and documentation parity (SC-005).
pub fn valid_agent_names() -> Vec<&'static str> {
    crate::hooks::paths::ALL_AGENT_NAMES.to_vec()
}

/// Result of configuring a single agent's artifacts.
#[derive(Debug)]
enum WriteOutcome {
    /// The agent's config was written/updated; carries the artifact path.
    Configured(PathBuf),
    /// The agent is already configured; nothing was written (FR-005).
    Skipped,
}

/// Per-agent outcome of an `rgt init` run (FR-008).
#[derive(Debug)]
pub enum AgentOutcome {
    Configured { agent: String, artifact: PathBuf },
    Skipped { agent: String },
    Failed { agent: String, reason: String },
}

/// Full per-agent report of an `rgt init` run.
#[derive(Debug, Default)]
pub struct InstallReport {
    pub outcomes: Vec<AgentOutcome>,
}

impl InstallReport {
    pub fn is_empty(&self) -> bool {
        self.outcomes.is_empty()
    }
}

/// Detects installed AI coding tools and configures their hooks. Supports the
/// full 13-agent RTK set, global installs, `--force` overwrite, and per-agent
/// targeting. Without `--agent`, only agents whose detection trigger is present
/// are configured (US2).
///
/// Failures are collected per agent (never short-circuited) so a single
/// malformed config never prevents healthy agents from being configured
/// (FR-008).
pub fn detect_and_configure_hooks(
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> Result<InstallReport, ConfigEditError> {
    let home = dirs::home_dir().ok_or_else(|| ConfigEditError::msg("Home directory not found"))?;
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
) -> Result<InstallReport, ConfigEditError> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rgt"));
    configure(home, &config_dir, global, force, agent, &exe)
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
) -> Result<InstallReport, ConfigEditError> {
    // FR-003: subprocess hooks reference the CLI by absolute path so capture
    // does not depend on PATH resolution at hook-invocation time. `current_exe`
    // effectively never fails; fall back to the bare name only as a last resort.
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("rgt"));
    detect_and_configure_hooks_with_config_and_exe(home, config_dir, global, force, agent, &exe)
}

/// Same as [`detect_and_configure_hooks_with_config`], but with an explicit
/// CLI executable path injected (deterministic tests inject a fixed path).
pub fn detect_and_configure_hooks_with_config_and_exe(
    home: &Path,
    config_dir: &Path,
    global: bool,
    force: bool,
    agent: Option<&str>,
    exe: &Path,
) -> Result<InstallReport, ConfigEditError> {
    configure(home, config_dir, global, force, agent, exe)
}

/// Shell-quotes an absolute CLI path for embedding in a hook command string
/// (handles paths containing spaces).
fn shell_quote(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

fn configure(
    home: &Path,
    config_dir: &Path,
    global: bool,
    force: bool,
    agent: Option<&str>,
    rgt_path: &Path,
) -> Result<InstallReport, ConfigEditError> {
    let mut outcomes = Vec::new();

    // Resolve the targeted canonical agent. Unknown names configure nothing;
    // the CLI layer enforces FR-001 (exit code 2 + valid-name list).
    let target = match agent {
        None => None,
        Some(name) => resolve_agent_name(name),
    };
    if agent.is_some() && target.is_none() {
        return Ok(InstallReport { outcomes });
    }

    // Without `--agent`, configure only agents detected as installed (US2).
    let targets: Vec<String> = match &target {
        Some(canonical) => vec![canonical.clone()],
        None => detect_installed_agents(home, config_dir),
    };
    let wants = |name: &str| targets.iter().any(|t| t == name);

    macro_rules! configure_agent {
        ($agent:expr, $writer:expr) => {
            if wants($agent) {
                match $writer {
                    Ok(WriteOutcome::Configured(artifact)) => {
                        outcomes.push(AgentOutcome::Configured {
                            agent: $agent.to_string(),
                            artifact,
                        })
                    }
                    Ok(WriteOutcome::Skipped) => outcomes.push(AgentOutcome::Skipped {
                        agent: $agent.to_string(),
                    }),
                    Err(e) => outcomes.push(AgentOutcome::Failed {
                        agent: $agent.to_string(),
                        reason: e.to_string(),
                    }),
                }
            }
        };
    }

    configure_agent!(
        "claude-code",
        write_claude_code(home, global, force, rgt_path)
    );
    configure_agent!("cursor", write_cursor(home, global, rgt_path));
    configure_agent!("codex", write_codex(force));
    configure_agent!("windsurf", write_windsurf(force));
    configure_agent!("copilot", write_copilot(home, config_dir, force, rgt_path));
    configure_agent!("gemini", write_gemini(home, rgt_path));
    configure_agent!("vibe", write_vibe(home, force, rgt_path));
    configure_agent!("opencode", write_opencode(home, global, force, rgt_path));
    configure_agent!("pi", write_pi(home, global, force, rgt_path));
    configure_agent!("hermes", write_hermes(home, global, force, rgt_path));
    configure_agent!("cline", write_cline(force));
    configure_agent!("antigravity", write_antigravity(force));
    configure_agent!("kilocode", write_kilocode(force));

    Ok(InstallReport { outcomes })
}

// ---------------------------------------------------------------------------
// Shared edit helpers (preservation guarantees live in `editor`)
// ---------------------------------------------------------------------------

/// Attaches `path` to a [`ConfigEditError`] produced by a text-level editor, so
/// user-facing failure reasons always name the offending file (FR-009).
fn with_path(path: &Path, mut e: ConfigEditError) -> ConfigEditError {
    if e.path.is_none() {
        e.path = Some(path.display().to_string());
    }
    e
}

/// Ensures the RGT `desired` structure is present in the JSONC config at `path`,
/// merging byte-for-byte and preserving all non-RGT content (FR-001/FR-002).
fn ensure_json_hooks(
    path: &Path,
    desired: &serde_json::Value,
) -> Result<WriteOutcome, ConfigEditError> {
    let text = editor::read_config(path)?;
    if text.trim().is_empty() {
        let content = serde_json::to_string_pretty(desired).map_err(|e| {
            with_path(
                path,
                ConfigEditError::msg(format!("failed to serialize config: {e}")),
            )
        })?;
        editor::write_config(path, &content)?;
        return Ok(WriteOutcome::Configured(path.to_path_buf()));
    }
    let (edited, changed) =
        editor::jsonc_merge_hook_entries(&text, desired).map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::Skipped);
    }
    editor::write_config_with_backup(path, &edited)?;
    Ok(WriteOutcome::Configured(path.to_path_buf()))
}

/// Runs a TOML edit closure against the config at `path`, writing with a backup
/// when anything changed (FR-004).
fn ensure_toml<F>(path: &Path, edit: F) -> Result<WriteOutcome, ConfigEditError>
where
    F: FnOnce(&str) -> Result<(String, bool), ConfigEditError>,
{
    let text = editor::read_config(path)?;
    let (edited, changed) = edit(&text).map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::Skipped);
    }
    editor::write_config_with_backup(path, &edited)?;
    Ok(WriteOutcome::Configured(path.to_path_buf()))
}

/// Ensures an RGT-marked text block in `path` using paired markers. Without
/// `--force`, an existing marker is a no-op (FR-005); with `--force` the block
/// is replaced in place, preserving content above and below it (FR-006). Legacy
/// blocks (marker without end marker) are a hard error (FR-006).
fn ensure_text_block(
    path: &Path,
    open_marker: &str,
    end_marker: &str,
    block: &str,
    force: bool,
) -> Result<WriteOutcome, ConfigEditError> {
    let text = editor::read_config(path)?;
    let has_open = text
        .lines()
        .any(|l| l.trim_start().starts_with(open_marker));
    if has_open && !force {
        return Ok(WriteOutcome::Skipped);
    }
    let (edited, changed) = editor::text_replace_block(&text, open_marker, end_marker, block)
        .map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::Skipped);
    }
    editor::write_config_with_backup(path, &edited)?;
    Ok(WriteOutcome::Configured(path.to_path_buf()))
}

// ---------------------------------------------------------------------------
// Existing integrations
// ---------------------------------------------------------------------------

fn write_claude_code(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let claude_dir = if global {
        home.join(".claude")
    } else {
        PathBuf::from(".claude")
    };
    let settings_file = claude_dir.join("settings.json");

    // FR-003: absolute CLI path. FR-004: matcher scoped to the tools RGT
    // captures (Read|Edit|Write|Bash) so the hook does not spawn on every call;
    // `Bash` is kept so Bash-read capture (026) keeps working.
    let exe = shell_quote(rgt_path);
    let desired = json!({
        "hooks": {
            "PostToolUse": [ { "matcher": "Read|Edit|Write|Bash", "hooks": [{ "type": "command", "command": format!("{} hook post", exe) }] } ],
            "PreToolUse": [ { "matcher": "Read|Edit|Write|Bash", "hooks": [{ "type": "command", "command": format!("{} hook pre", exe) }] } ],
        }
    });
    let outcome = ensure_json_hooks(&settings_file, &desired)?;

    // FR-003: write a CLAUDE.md instructions block so Claude Code agents get
    // the non-text-source reporting guidance (project CLAUDE.md, or
    // ~/.claude/CLAUDE.md with --global). Idempotent + content-preserving via
    // the paired-marker helper.
    let claude_md = if global {
        home.join(".claude").join("CLAUDE.md")
    } else {
        PathBuf::from("CLAUDE.md")
    };
    let claude_block = glue::with_instruction(glue::RGT_MARKER);
    let instructions = ensure_text_block(
        &claude_md,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &claude_block,
        force,
    )?;

    if matches!(outcome, WriteOutcome::Configured(_)) {
        // The legacy hooks.json artifact is superseded by settings.json hooks.
        let old_hooks_file = claude_dir.join("hooks.json");
        if old_hooks_file.exists() {
            let _ = fs::remove_file(&old_hooks_file);
        }
    }
    Ok(match (outcome, instructions) {
        (WriteOutcome::Configured(_), _) | (_, WriteOutcome::Configured(_)) => {
            WriteOutcome::Configured(settings_file)
        }
        _ => WriteOutcome::Skipped,
    })
}

fn write_cursor(
    home: &Path,
    global: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let cursor_dir = if global {
        home.join(".cursor")
    } else {
        PathBuf::from(".cursor")
    };
    let cursor_hooks_file = cursor_dir.join("hooks.json");

    let exe = shell_quote(rgt_path);
    let desired = json!({
        "version": 1,
        "hooks": {
            "preToolUse": [ { "command": format!("{} hook pre", exe), "matcher": "Shell" } ],
            "postToolUse": [ { "command": format!("{} hook post", exe), "matcher": "Shell" } ],
        }
    });
    ensure_json_hooks(&cursor_hooks_file, &desired)
}

fn codex_section() -> String {
    "\n## RGT Integration\n\nRGT tracks numeric and date provenance for AI coding agents.\n\n### Recording Values\nAfter reading a data file containing numbers or dates, record its values:\n- `rgt record <file>`: extracts and tracks numeric/date values from file content\n- Run `rgt status` to check what's tracked\n\n### Recording Derivations\nAfter computing a derived value from tracked root nodes:\n- `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n- The derivation is verified before recording; wrong results are rejected (exit 1)\n\n### Inspecting the Graph\n- `rgt status`: see all tracked nodes and staleness state\n- `rgt query <node_id>`: trace provenance lineage for a value\n- `rgt graph`: export the dependency graph as text or mermaid\n\nRun `rgt --help` for all available commands.\n"
        .to_string()
}

fn write_codex(force: bool) -> Result<WriteOutcome, ConfigEditError> {
    let agents_file = PathBuf::from("AGENTS.md");
    let block = glue::with_instruction(&codex_section());

    let text = editor::read_config(&agents_file)?;
    if text.trim().is_empty() {
        let content = format!("# RGT Agents Instructions{}", block);
        editor::write_config(&agents_file, &content)?;
        return Ok(WriteOutcome::Configured(agents_file));
    }
    ensure_text_block(
        &agents_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &block,
        force,
    )
}

fn write_windsurf(force: bool) -> Result<WriteOutcome, ConfigEditError> {
    let rules_file = PathBuf::from(".windsurfrules");
    let base = "# RGT Integration\nRGT tracks numeric and date provenance for AI coding agents.\nAfter reading data files, record extracted values with `rgt record <file>`.\nAfter computing derived values, record them with `rgt derive --parents <ids> --operation EXPRESSION --expression \"a - b\" --result <val>`.\nDerivations are verified before recording; wrong results are rejected.\nUse `rgt status` to check provenance graph state.\nRun `rgt --help` for all available commands.\n";
    let rules_content = glue::with_instruction(base);
    ensure_text_block(
        &rules_file,
        "# RGT Integration",
        glue::RGT_END_MARKER,
        &rules_content,
        force,
    )
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
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let settings_file = crate::hooks::paths::copilot_user_settings(config_dir);
    let exe = shell_quote(rgt_path);
    let desired = json!({
        "github.copilot.chat.hooks": {
            "PostToolUse": [ { "matcher": ".*", "hooks": [{ "type": "command", "command": format!("{} hook post --agent copilot", exe) }] } ],
        }
    });
    let chat = ensure_json_hooks(&settings_file, &desired)?;

    let cli_rules_file =
        crate::hooks::paths::copilot_cli_config_dir(home, config_dir).join("AGENTS.md");
    let cli = ensure_text_block(
        &cli_rules_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(glue::COPILOT_CLI_RULES),
        force,
    )?;

    Ok(match (chat, cli) {
        (WriteOutcome::Configured(_), _) | (_, WriteOutcome::Configured(_)) => {
            WriteOutcome::Configured(settings_file)
        }
        _ => WriteOutcome::Skipped,
    })
}

/// Gemini CLI: `~/.gemini/hooks.toml` PostToolUse entry.
fn write_gemini(home: &Path, rgt_path: &Path) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = home.join(".gemini").join("hooks.toml");
    let exe = shell_quote(rgt_path);
    ensure_toml(&hooks_file, |t| {
        editor::toml_ensure_table_entry(
            t,
            &["PostToolUse"],
            "command",
            &format!("{} hook post --agent gemini", exe),
        )
    })
}

/// Mistral Vibe: `~/.vibe/hooks.toml` pre_tool entry + `~/.vibe/prompts/rgt.md`.
fn write_vibe(home: &Path, force: bool, rgt_path: &Path) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = home.join(".vibe").join("hooks.toml");
    let exe = shell_quote(rgt_path);
    let hooks = ensure_toml(&hooks_file, |t| {
        editor::toml_ensure_vibe_pre_tool(t, &format!("{} hook post --agent vibe", exe))
    })?;

    let prompt_file = home.join(".vibe").join("prompts").join("rgt.md");
    let prompt = ensure_text_block(
        &prompt_file,
        "# RGT Integration",
        glue::RGT_END_MARKER,
        &glue::with_instruction(glue::VIBE_PROMPT),
        force,
    )?;

    Ok(match (hooks, prompt) {
        (WriteOutcome::Configured(_), _) | (_, WriteOutcome::Configured(_)) => {
            WriteOutcome::Configured(hooks_file)
        }
        _ => WriteOutcome::Skipped,
    })
}

// ---------------------------------------------------------------------------
// New integrations (US1): plugin-tier agents
// ---------------------------------------------------------------------------

/// Writes a standalone plugin file. Idempotent: skipped when present unless
/// `--force`.
fn write_plugin_file(path: &Path, content: &str, force: bool) -> Result<bool, ConfigEditError> {
    if path.exists() && !force {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, content).map_err(|e| {
        ConfigEditError::new(
            Some(path.display().to_string()),
            format!("failed to write plugin file: {e}"),
        )
    })?;
    Ok(true)
}

/// OpenCode: embedded `rgt.ts` TS plugin.
fn write_opencode(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let plugin_dir = if global {
        home.join(".config").join("opencode").join("plugins")
    } else {
        PathBuf::from(".opencode").join("plugin")
    };
    let plugin_file = plugin_dir.join("rgt.ts");
    let content = glue::opencode_plugin(&rgt_path.display().to_string());
    if write_plugin_file(&plugin_file, &content, force)? {
        Ok(WriteOutcome::Configured(plugin_file))
    } else {
        Ok(WriteOutcome::Skipped)
    }
}

/// Pi: embedded `rgt.ts` TS extension.
fn write_pi(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let ext_dir = if global {
        home.join(".pi").join("agent").join("extensions")
    } else {
        PathBuf::from(".pi").join("extensions")
    };
    let ext_file = ext_dir.join("rgt.ts");
    let content = glue::pi_extension(&rgt_path.display().to_string());
    if write_plugin_file(&ext_file, &content, force)? {
        Ok(WriteOutcome::Configured(ext_file))
    } else {
        Ok(WriteOutcome::Skipped)
    }
}

/// Hermes: embedded `plugin.py` + `plugins.enabled` in the Hermes config.
fn write_hermes(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let plugin_dir = if global {
        home.join(".hermes").join("plugins").join("rgt")
    } else {
        PathBuf::from(".hermes").join("plugins").join("rgt")
    };
    let plugin_file = plugin_dir.join("plugin.py");
    let content = glue::hermes_plugin(&rgt_path.display().to_string());
    let written_plugin = write_plugin_file(&plugin_file, &content, force)?;

    let config_file = if global {
        home.join(".hermes").join("config.toml")
    } else {
        PathBuf::from(".hermes").join("config.toml")
    };
    let config = ensure_toml(&config_file, |t| {
        editor::toml_ensure_array_value(t, &["plugins", "enabled"], "rgt")
    })?;

    Ok(match (written_plugin, config) {
        (true, _) | (_, WriteOutcome::Configured(_)) => WriteOutcome::Configured(plugin_file),
        _ => WriteOutcome::Skipped,
    })
}

// ---------------------------------------------------------------------------
// New integrations (US1): rules-tier agents
// ---------------------------------------------------------------------------

/// Cline / Roo Code: append RGT section to `.clinerules`.
fn write_cline(force: bool) -> Result<WriteOutcome, ConfigEditError> {
    let rules_file = PathBuf::from(".clinerules");
    ensure_text_block(
        &rules_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(glue::CLINE_RULES),
        force,
    )
}

/// Antigravity: rules file under `.agents/rules/`.
fn write_antigravity(force: bool) -> Result<WriteOutcome, ConfigEditError> {
    let rules_file = PathBuf::from(".agents")
        .join("rules")
        .join("antigravity-rgt-rules.md");
    ensure_text_block(
        &rules_file,
        "# RGT Integration",
        glue::RGT_END_MARKER,
        &glue::with_instruction(glue::ANTIGRAVITY_RULES),
        force,
    )
}

/// Kilo: rules file under `.kilocode/rules/`.
fn write_kilocode(force: bool) -> Result<WriteOutcome, ConfigEditError> {
    let rules_file = PathBuf::from(".kilocode")
        .join("rules")
        .join("rgt-rules.md");
    ensure_text_block(
        &rules_file,
        "# RGT Integration",
        glue::RGT_END_MARKER,
        &glue::with_instruction(glue::KILO_RULES),
        force,
    )
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

/// Returns the RGT hook artifact paths that currently exist for the given
/// home/config bases (FR-002 — `rgt doctor` uses this to detect installed
/// hooks). Enumerates the known per-agent artifact locations in both the
/// project (`global == false`) and home (`global == true`) variants; only
/// paths that exist on disk are returned.
pub fn list_hook_artifacts(home: &Path, config_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut artifacts = Vec::new();
    let mut push = |agent: &str, path: PathBuf| {
        if path.exists() {
            artifacts.push((agent.to_string(), path));
        }
    };

    let project = |name: &str| PathBuf::from(name);
    let home_dir = |name: &str| home.join(name);

    // Claude Code (project `.claude` + global `~/.claude`).
    push("claude-code", project(".claude").join("settings.json"));
    push("claude-code", home_dir(".claude").join("settings.json"));
    // Cursor.
    push("cursor", project(".cursor").join("hooks.json"));
    push("cursor", home_dir(".cursor").join("hooks.json"));
    // Copilot Chat (VS Code user settings) + CLI rules.
    push(
        "copilot",
        crate::hooks::paths::copilot_user_settings(config_dir),
    );
    push(
        "copilot",
        crate::hooks::paths::copilot_cli_config_dir(home, config_dir).join("AGENTS.md"),
    );
    // Gemini / Vibe.
    push("gemini", home_dir(".gemini").join("hooks.toml"));
    push("vibe", home_dir(".vibe").join("hooks.toml"));
    // OpenCode / Pi / Hermes plugins.
    push(
        "opencode",
        project(".opencode").join("plugin").join("rgt.ts"),
    );
    push(
        "opencode",
        home_dir(".config")
            .join("opencode")
            .join("plugins")
            .join("rgt.ts"),
    );
    push("pi", project(".pi").join("extensions").join("rgt.ts"));
    push(
        "pi",
        home_dir(".pi")
            .join("agent")
            .join("extensions")
            .join("rgt.ts"),
    );
    push(
        "hermes",
        project(".hermes")
            .join("plugins")
            .join("rgt")
            .join("plugin.py"),
    );
    push(
        "hermes",
        home_dir(".hermes")
            .join("plugins")
            .join("rgt")
            .join("plugin.py"),
    );
    // Rules-file agents.
    push("codex", project("AGENTS.md"));
    push("windsurf", project(".windsurfrules"));
    push("cline", project(".clinerules"));
    push(
        "antigravity",
        project(".agents")
            .join("rules")
            .join("antigravity-rgt-rules.md"),
    );
    push(
        "kilocode",
        project(".kilocode").join("rules").join("rgt-rules.md"),
    );

    artifacts
}
