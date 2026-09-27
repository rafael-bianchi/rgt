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
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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
        tier: Tier::Hook,
    },
    AgentDef {
        canonical: "windsurf",
        aliases: &[],
        tier: Tier::Hook,
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
    AGENTS.iter().any(|agent| {
        agent.canonical == name
            && matches!(agent.tier, Tier::Hook | Tier::Plugin)
            && !client_surfaces_for_agent(name).is_empty()
    })
}

/// Resolves a canonical installer identity through the shared surface table.
/// Split hosts remain separate (Copilot Chat/CLI and Cline/Roo).
pub fn client_surfaces_for_agent(
    name: &str,
) -> Vec<&'static crate::hooks::registration::ClientSurface> {
    if !AGENTS.iter().any(|agent| agent.canonical == name) {
        return Vec::new();
    }
    crate::hooks::registration::resolve_client_surfaces(name)
}

/// Whether `name` is an accepted `--agent` spelling.
/// All accepted `--agent` spellings (13 canonical + 3 aliases) for error
/// messages and documentation parity (SC-005).
pub fn valid_agent_names() -> Vec<&'static str> {
    crate::hooks::paths::ALL_AGENT_NAMES.to_vec()
}

/// Result of configuring a single agent's artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteKind {
    Configured,
    Migrated,
    AlreadyCurrent,
}

#[derive(Debug)]
struct WriteOutcome {
    kind: WriteKind,
    artifact: PathBuf,
    backups: Vec<PathBuf>,
}

impl WriteOutcome {
    fn configured(artifact: &Path, backup: Option<PathBuf>) -> Self {
        Self {
            kind: WriteKind::Configured,
            artifact: artifact.to_path_buf(),
            backups: backup.into_iter().collect(),
        }
    }

    fn migrated(artifact: &Path, backup: Option<PathBuf>) -> Self {
        Self {
            kind: WriteKind::Migrated,
            artifact: artifact.to_path_buf(),
            backups: backup.into_iter().collect(),
        }
    }

    fn already_current(artifact: &Path) -> Self {
        Self {
            kind: WriteKind::AlreadyCurrent,
            artifact: artifact.to_path_buf(),
            backups: Vec::new(),
        }
    }

    fn combine(primary: &Path, outcomes: impl IntoIterator<Item = Self>) -> Self {
        let outcomes: Vec<Self> = outcomes.into_iter().collect();
        let kind = if outcomes
            .iter()
            .any(|outcome| outcome.kind == WriteKind::Migrated)
        {
            WriteKind::Migrated
        } else if outcomes
            .iter()
            .any(|outcome| outcome.kind == WriteKind::Configured)
        {
            WriteKind::Configured
        } else {
            WriteKind::AlreadyCurrent
        };
        let backups = outcomes
            .into_iter()
            .flat_map(|outcome| outcome.backups)
            .collect();
        Self {
            kind,
            artifact: primary.to_path_buf(),
            backups,
        }
    }
}

/// Per-agent outcome of an `rgt init` run (FR-008).
#[derive(Debug)]
pub enum AgentOutcome {
    Configured {
        agent: String,
        artifact: PathBuf,
        backups: Vec<PathBuf>,
    },
    Migrated {
        agent: String,
        artifact: PathBuf,
        backups: Vec<PathBuf>,
    },
    AlreadyCurrent {
        agent: String,
        artifact: PathBuf,
    },
    Conflict {
        agent: String,
        artifact: Option<PathBuf>,
        backup: Option<PathBuf>,
        reason: String,
    },
    Failed {
        agent: String,
        artifact: Option<PathBuf>,
        backup: Option<PathBuf>,
        reason: String,
    },
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

/// Quotes a path for the POSIX command shell, including spaces, quotes, and
/// expansion metacharacters. NUL is not representable in a filesystem path.
fn posix_shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\''"))
}

/// Builds a fail-open direct hook command. The small host-shell fallback keeps
/// a missing or moved RGT binary from changing the agent's completed action.
fn direct_hook_command(rgt_path: &Path, event: &str, agent: &str) -> String {
    #[cfg(windows)]
    {
        direct_hook_command_for_platform(rgt_path, event, agent, true)
    }
    #[cfg(not(windows))]
    {
        direct_hook_command_for_platform(rgt_path, event, agent, false)
    }
}

/// Platform-specific command wrapper used by native command-hook hosts. Public
/// for deterministic cross-platform contract checks that do not require a
/// second operating system runner.
pub fn direct_hook_command_for_platform(
    rgt_path: &Path,
    event: &str,
    agent: &str,
    windows: bool,
) -> String {
    if windows {
        windows_powershell_hook_command(rgt_path, event, agent)
    } else {
        posix_direct_hook_command(rgt_path, event, agent)
    }
}

/// Runs a direct hook under a shell watchdog and buffers its response so a
/// timeout cannot leak a partial host response. The RGT CLI also enforces its
/// own event deadline; this outer bound covers a stalled child process.
fn posix_direct_hook_command(rgt_path: &Path, event: &str, agent: &str) -> String {
    format!(
        "( _rgt_tmp=$(mktemp -d \"${{TMPDIR:-/tmp}}/rgt-hook.XXXXXX\") || exit 0; trap 'rm -rf \"$_rgt_tmp\"' 0; {} hook {} --agent {} --rgt-managed >\"$_rgt_tmp/stdout\" 2>/dev/null <&0 & _rgt_pid=$!; ( trap 'kill \"$_rgt_timer\" 2>/dev/null || true' 0; sleep 0.8 & _rgt_timer=$!; wait \"$_rgt_timer\"; if kill -0 \"$_rgt_pid\" 2>/dev/null; then : >\"$_rgt_tmp/timed-out\"; kill -KILL \"$_rgt_pid\" 2>/dev/null || true; fi ) </dev/null >/dev/null 2>&1 & _rgt_watchdog=$!; wait \"$_rgt_pid\" 2>/dev/null || true; kill \"$_rgt_watchdog\" 2>/dev/null || true; wait \"$_rgt_watchdog\" 2>/dev/null || true; if [ ! -f \"$_rgt_tmp/timed-out\" ]; then cat \"$_rgt_tmp/stdout\"; fi ) 2>/dev/null || true",
        posix_shell_quote(rgt_path),
        event,
        agent
    )
}

/// Uses PowerShell's encoded-command mode and ProcessStartInfo so Windows
/// paths never pass through cmd.exe expansion or quote parsing. Both stdin and
/// child output are handled asynchronously; only a completed response is
/// forwarded, and the child is killed at the remaining 800 ms deadline.
fn windows_powershell_hook_command(rgt_path: &Path, event: &str, agent: &str) -> String {
    let executable = powershell_single_quote(&rgt_path.to_string_lossy());
    let arguments = powershell_single_quote(&format!("hook {event} --agent {agent} --rgt-managed"));
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; $clock=[System.Diagnostics.Stopwatch]::StartNew(); $p=$null; $cts=$null; $started=$false; try {{ $p=New-Object System.Diagnostics.Process; $p.StartInfo=New-Object System.Diagnostics.ProcessStartInfo; $p.StartInfo.FileName={executable}; $p.StartInfo.Arguments={arguments}; $p.StartInfo.UseShellExecute=$false; $p.StartInfo.CreateNoWindow=$true; $p.StartInfo.RedirectStandardInput=$true; $p.StartInfo.RedirectStandardOutput=$true; $p.StartInfo.RedirectStandardError=$true; $started=$p.Start(); if ($started) {{ $cts=New-Object System.Threading.CancellationTokenSource; $stdoutTask=$p.StandardOutput.ReadToEndAsync(); $stderrTask=$p.StandardError.ReadToEndAsync(); $stdinTask=[Console]::OpenStandardInput().CopyToAsync($p.StandardInput.BaseStream,81920,$cts.Token); while (-not $p.HasExited -and -not $stdinTask.IsCompleted -and $clock.ElapsedMilliseconds -lt 800) {{ $remaining=[Math]::Max(0,800-[int]$clock.ElapsedMilliseconds); if ($remaining -gt 0) {{ $p.WaitForExit([int][Math]::Min(10,$remaining)) | Out-Null }} }}; if ($stdinTask.IsCompleted) {{ $p.StandardInput.Close() }}; $remaining=[Math]::Max(0,800-[int]$clock.ElapsedMilliseconds); if (-not $p.HasExited -and $remaining -gt 0) {{ $p.WaitForExit([int]$remaining) | Out-Null }}; if (-not $p.HasExited) {{ try {{ $p.Kill() }} catch {{ }}; $p.WaitForExit(50) | Out-Null }}; $remaining=[Math]::Max(0,800-[int]$clock.ElapsedMilliseconds); if ($p.HasExited -and $remaining -gt 0 -and $stdoutTask.Wait([int]$remaining)) {{ $remaining=[Math]::Max(0,800-[int]$clock.ElapsedMilliseconds); if ($remaining -gt 0 -and $stderrTask.Wait([int]$remaining) -and $clock.ElapsedMilliseconds -lt 800) {{ [Console]::Out.Write($stdoutTask.Result) }} }}; $cts.Cancel(); $p.StandardInput.Close() }} }} catch {{ }} finally {{ if ($cts) {{ $cts.Cancel() }}; if ($started -and $p -and -not $p.HasExited) {{ try {{ $p.Kill() }} catch {{ }}; $p.WaitForExit(50) | Out-Null }} }}; exit 0"
    );
    let encoded = script
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    format!(
        "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand {}",
        BASE64.encode(encoded)
    )
}

fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
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
                    Ok(WriteOutcome {
                        kind: WriteKind::Configured,
                        artifact,
                        backups,
                    }) => outcomes.push(AgentOutcome::Configured {
                        agent: $agent.to_string(),
                        artifact,
                        backups,
                    }),
                    Ok(WriteOutcome {
                        kind: WriteKind::Migrated,
                        artifact,
                        backups,
                    }) => outcomes.push(AgentOutcome::Migrated {
                        agent: $agent.to_string(),
                        artifact,
                        backups,
                    }),
                    Ok(WriteOutcome {
                        kind: WriteKind::AlreadyCurrent,
                        artifact,
                        ..
                    }) => outcomes.push(AgentOutcome::AlreadyCurrent {
                        agent: $agent.to_string(),
                        artifact,
                    }),
                    Err(e) if is_ownership_conflict(&e) => outcomes.push(AgentOutcome::Conflict {
                        agent: $agent.to_string(),
                        artifact: e.path.as_deref().map(PathBuf::from),
                        backup: e.backup_path.as_deref().map(PathBuf::from),
                        reason: e.to_string(),
                    }),
                    Err(e) => outcomes.push(AgentOutcome::Failed {
                        agent: $agent.to_string(),
                        artifact: e.path.as_deref().map(PathBuf::from),
                        backup: e.backup_path.as_deref().map(PathBuf::from),
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
    configure_agent!("cursor", write_cursor(home, global, force, rgt_path));
    configure_agent!("codex", write_codex(home, global, force, rgt_path));
    configure_agent!("windsurf", write_windsurf(home, global, force, rgt_path));
    configure_agent!("copilot", write_copilot(home, config_dir, force, rgt_path));
    configure_agent!("gemini", write_gemini(home, global, force, rgt_path));
    configure_agent!("vibe", write_vibe(home, global, force, rgt_path));
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
        return Ok(WriteOutcome::configured(path, None));
    }
    let has_rgt_entry = editor::jsonc_hook_commands(&text)
        .map_err(|error| with_path(path, error))?
        .iter()
        .any(|command| editor::is_rgt_command(command));
    let (edited, changed) =
        editor::jsonc_merge_hook_entries(&text, desired).map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::already_current(path));
    }
    let backup = existing_backup_path(path);
    editor::write_config_with_backup(path, &edited)?;
    Ok(if has_rgt_entry {
        WriteOutcome::migrated(path, backup)
    } else {
        WriteOutcome::configured(path, backup)
    })
}

/// Runs a TOML edit closure against the config at `path`, writing with a backup
/// when anything changed (FR-004).
fn ensure_toml<F>(path: &Path, edit: F) -> Result<WriteOutcome, ConfigEditError>
where
    F: FnOnce(&str) -> Result<(String, bool), ConfigEditError>,
{
    let text = editor::read_config(path)?;
    let had_rgt_command = text.contains("rgt hook");
    let (edited, changed) = edit(&text).map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::already_current(path));
    }
    let backup = existing_backup_path(path);
    editor::write_config_with_backup(path, &edited)?;
    Ok(if had_rgt_command {
        WriteOutcome::migrated(path, backup)
    } else {
        WriteOutcome::configured(path, backup)
    })
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
    _force: bool,
) -> Result<WriteOutcome, ConfigEditError> {
    let text = editor::read_config(path)?;
    let had_rgt_block = text
        .lines()
        .any(|line| line.trim_start().starts_with(open_marker));
    let (edited, changed) = editor::text_replace_block(&text, open_marker, end_marker, block)
        .map_err(|e| with_path(path, e))?;
    if !changed {
        return Ok(WriteOutcome::already_current(path));
    }
    let backup = existing_backup_path(path);
    editor::write_config_with_backup(path, &edited)?;
    Ok(if had_rgt_block {
        WriteOutcome::migrated(path, backup)
    } else {
        WriteOutcome::configured(path, backup)
    })
}

fn existing_backup_path(path: &Path) -> Option<PathBuf> {
    path.exists()
        .then(|| PathBuf::from(format!("{}.rgt.bak", path.display())))
}

fn is_ownership_conflict(error: &ConfigEditError) -> bool {
    let reason = error.reason.to_ascii_lowercase();
    [
        "ownership is ambiguous",
        "mixed rgt",
        "interleaved with other",
        "conflicting user value",
    ]
    .iter()
    .any(|marker| reason.contains(marker))
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
    let desired = json!({
        "hooks": {
            "PostToolUse": [ { "matcher": "Read|Edit|Write|Bash", "hooks": [{ "type": "command", "command": direct_hook_command(rgt_path, "post", "claude-code") }] } ],
            "PreToolUse": [ { "matcher": "Read|Edit|Write|Bash", "hooks": [{ "type": "command", "command": direct_hook_command(rgt_path, "pre", "claude-code") }] } ],
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

    if outcome.kind != WriteKind::AlreadyCurrent {
        // The legacy hooks.json artifact is superseded by settings.json hooks.
        let old_hooks_file = claude_dir.join("hooks.json");
        if old_hooks_file.exists() {
            let _ = fs::remove_file(&old_hooks_file);
        }
    }
    Ok(WriteOutcome::combine(
        &settings_file,
        [outcome, instructions],
    ))
}

fn write_cursor(
    home: &Path,
    global: bool,
    _force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let cursor_dir = if global {
        home.join(".cursor")
    } else {
        PathBuf::from(".cursor")
    };
    let cursor_hooks_file = cursor_dir.join("hooks.json");

    let desired = json!({
        "version": 1,
        "hooks": {
            "preToolUse": [ { "command": direct_hook_command(rgt_path, "pre", "cursor"), "matcher": "Shell" } ],
            "postToolUse": [ { "command": direct_hook_command(rgt_path, "post", "cursor"), "matcher": "Shell" } ],
        }
    });
    // Project rules are the documented loadable Cursor guidance location.
    // A dedicated filename avoids editing other Cursor rules.
    let rules_file = PathBuf::from(".cursor/rules/rgt.mdc");
    let existing = editor::read_config(&rules_file)?;
    if !existing.is_empty()
        && (!existing.contains(glue::RGT_MARKER) || !existing.contains(glue::RGT_END_MARKER))
    {
        return Err(ConfigEditError::new(
            Some(rules_file.display().to_string()),
            "rule ownership is ambiguous; move or rename the existing file before initializing RGT",
        ));
    }
    // Validate and repair the dedicated RGT rule before writing the hook. A
    // malformed frontmatter is an ownership conflict, not a usable rule.
    let repaired = if existing.is_empty() {
        None
    } else {
        Some(repair_cursor_frontmatter(&existing, &rules_file)?)
    };
    let hooks = ensure_json_hooks(&cursor_hooks_file, &desired)?;
    let guidance = if existing.is_empty() {
        editor::write_config(
            &rules_file,
            &format!(
                "---\ndescription: Record numeric and date provenance with RGT\nalwaysApply: true\n---\n{}",
                glue::with_instruction(&codex_section())
            ),
        )?;
        WriteOutcome::configured(&rules_file, None)
    } else {
        let (edited, body_changed) = editor::text_replace_block(
            repaired.as_deref().unwrap_or(&existing),
            glue::RGT_MARKER,
            glue::RGT_END_MARKER,
            &glue::with_instruction(&codex_section()),
        )
        .map_err(|error| with_path(&rules_file, error))?;
        if edited == existing && !body_changed {
            WriteOutcome::already_current(&rules_file)
        } else {
            let backup = existing_backup_path(&rules_file);
            editor::write_config_with_backup(&rules_file, &edited)?;
            WriteOutcome::migrated(&rules_file, backup)
        }
    };
    Ok(WriteOutcome::combine(&cursor_hooks_file, [hooks, guidance]))
}

fn repair_cursor_frontmatter(text: &str, path: &Path) -> Result<String, ConfigEditError> {
    let conflict = || {
        ConfigEditError::new(
            Some(path.display().to_string()),
            "rule ownership is ambiguous: RGT rule frontmatter is malformed; restore valid --- delimiters and alwaysApply: true",
        )
    };
    let newline = if text.starts_with("---\r\n") {
        "\r\n"
    } else if text.starts_with("---\n") {
        "\n"
    } else {
        return Err(conflict());
    };
    let opener = format!("---{newline}");
    let rest = text.strip_prefix(opener.as_str()).ok_or_else(conflict)?;
    let delimiter = format!("{newline}---");
    let (header, body) = rest.split_once(delimiter.as_str()).ok_or_else(conflict)?;
    if !body.is_empty() && !body.starts_with(newline) {
        return Err(conflict());
    }
    let mut header_lines: Vec<String> = header.split(newline).map(str::to_string).collect();
    let settings: Vec<_> = header_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            line.split_once(':')
                .filter(|(key, _)| key.trim() == "alwaysApply")
                .map(|(_, value)| (index, value.trim().to_string()))
        })
        .collect();
    if settings.len() > 1 {
        return Err(conflict());
    }
    let updated_header = match settings.first() {
        Some((_, value)) if value == "true" => header.to_string(),
        Some((index, value)) if value == "false" => {
            header_lines[*index] = header_lines[*index].replacen("false", "true", 1);
            header_lines.join(newline)
        }
        None => format!("{header}{newline}alwaysApply: true"),
        _ => return Err(conflict()),
    };
    Ok(format!("---{newline}{updated_header}{newline}---{body}"))
}

fn codex_section() -> String {
    "\n## RGT Integration\n\nRGT tracks numeric and date provenance for AI coding agents.\n\n### Recording Values\nAfter reading a data file containing numbers or dates, record its values:\n- `rgt record <file>`: extracts and tracks numeric/date values from file content\n- Run `rgt status` to check what's tracked\n\n### Recording Derivations\nAfter computing a derived value from tracked root nodes:\n- `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression \"a - b\" --result <val>`\n- The derivation is verified before recording; wrong results are rejected (exit 1)\n\n### Inspecting the Graph\n- `rgt status`: see all tracked nodes and staleness state\n- `rgt query <node_id>`: trace provenance lineage for a value\n- `rgt graph`: export the dependency graph as text or mermaid\n\nRun `rgt --help` for all available commands.\n"
        .to_string()
}

fn write_codex(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = crate::hooks::paths::codex_hooks_json(home, global);
    let desired_hooks = json!({
        "hooks": {
            "PostToolUse": [{
                "matcher": ".*",
                "hooks": [{
                    "type": "command",
                    "command": direct_hook_command(rgt_path, "post", "codex")
                }]
            }]
        }
    });
    let hook_outcome = ensure_json_hooks(&hooks_file, &desired_hooks)?;

    let agents_file = PathBuf::from("AGENTS.md");
    let block = glue::with_instruction(&codex_section());

    let text = editor::read_config(&agents_file)?;
    let instruction_outcome = if text.trim().is_empty() {
        let content = format!("# RGT Agents Instructions{}", block);
        editor::write_config(&agents_file, &content)?;
        WriteOutcome::configured(&agents_file, None)
    } else {
        ensure_text_block(
            &agents_file,
            glue::RGT_MARKER,
            glue::RGT_END_MARKER,
            &block,
            force,
        )?
    };
    Ok(WriteOutcome::combine(
        &hooks_file,
        [hook_outcome, instruction_outcome],
    ))
}

fn write_windsurf(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = if global {
        crate::hooks::paths::windsurf_user_hooks_json(home)
    } else {
        let preferred = crate::hooks::paths::windsurf_workspace_hooks_json();
        let legacy = crate::hooks::paths::windsurf_legacy_workspace_hooks_json();
        if preferred.exists() {
            let preferred_text = editor::read_config(&preferred)?;
            let preferred_has_hooks = editor::jsonc_has_active_hooks(&preferred_text)
                .map_err(|e| with_path(&preferred, e))?;
            if preferred_has_hooks || !legacy.exists() {
                preferred
            } else {
                legacy
            }
        } else if legacy.exists() {
            legacy
        } else {
            preferred
        }
    };
    let desired_hooks = json!({
        "hooks": {
            "post_read_code": [{
                "command": direct_hook_command(rgt_path, "post", "windsurf"),
                "show_output": false
            }]
        }
    });
    let hook_outcome = ensure_json_hooks(&hooks_file, &desired_hooks)?;

    let rules_file = PathBuf::from(".windsurfrules");
    let base = "# RGT Integration\nRGT tracks numeric and date provenance for AI coding agents.\nAfter reading data files, record extracted values with `rgt record <file>`.\nAfter computing derived values, record them with `rgt derive --parents <ids> --operation EXPRESSION --expression \"a - b\" --result <val>`.\nDerivations are verified before recording; wrong results are rejected.\nUse `rgt status` to check provenance graph state.\nRun `rgt --help` for all available commands.\n";
    let rules_content = glue::with_instruction(base);
    let rules_outcome = ensure_text_block(
        &rules_file,
        "# RGT Integration",
        glue::RGT_END_MARKER,
        &rules_content,
        force,
    )?;
    Ok(WriteOutcome::combine(
        &hooks_file,
        [hook_outcome, rules_outcome],
    ))
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
    let desired = json!({
        "github.copilot.chat.hooks": {
            "PostToolUse": [ { "matcher": ".*", "hooks": [{ "type": "command", "command": direct_hook_command(rgt_path, "post", "copilot") }] } ],
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

    let chat_rules_file = PathBuf::from(".github/copilot-instructions.md");
    let chat_rules = ensure_text_block(
        &chat_rules_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(&codex_section()),
        force,
    )?;

    Ok(WriteOutcome::combine(
        &settings_file,
        [chat, cli, chat_rules],
    ))
}

/// Gemini CLI: `~/.gemini/hooks.toml` PostToolUse entry.
fn write_gemini(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = home.join(".gemini").join("hooks.toml");
    let hooks = ensure_toml(&hooks_file, |t| {
        editor::toml_ensure_table_entry(
            t,
            &["PostToolUse"],
            "command",
            &direct_hook_command(rgt_path, "post", "gemini"),
            "gemini",
        )
    })?;
    let guidance_file = if global {
        home.join(".gemini/GEMINI.md")
    } else {
        PathBuf::from("GEMINI.md")
    };
    let guidance = ensure_text_block(
        &guidance_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(&codex_section()),
        force,
    )?;
    Ok(WriteOutcome::combine(&hooks_file, [hooks, guidance]))
}

/// Mistral Vibe: a pre-tool entry plus loaded AGENTS.md guidance.
fn write_vibe(
    home: &Path,
    global: bool,
    force: bool,
    rgt_path: &Path,
) -> Result<WriteOutcome, ConfigEditError> {
    let hooks_file = home.join(".vibe").join("hooks.toml");
    let hooks = ensure_toml(&hooks_file, |t| {
        editor::toml_ensure_vibe_pre_tool(t, &direct_hook_command(rgt_path, "pre", "vibe"))
    })?;

    let guidance_file = crate::hooks::paths::vibe_guidance_path(home, global);
    let guidance = ensure_text_block(
        &guidance_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(&codex_section()),
        force,
    )?;

    Ok(WriteOutcome::combine(&hooks_file, [hooks, guidance]))
}

// ---------------------------------------------------------------------------
// New integrations (US1): plugin-tier agents
// ---------------------------------------------------------------------------

/// Writes a standalone plugin file. Idempotent: skipped when present unless
/// `--force`.
fn write_plugin_file(
    path: &Path,
    content: &str,
    agent: &str,
    _force: bool,
) -> Result<WriteOutcome, ConfigEditError> {
    if path.exists() {
        let existing = editor::read_config(path)?;
        if existing == content {
            return Ok(WriteOutcome::already_current(path));
        }
        if !is_rgt_owned_plugin(&existing, agent) {
            return Err(ConfigEditError::new(
                Some(path.display().to_string()),
                "plugin ownership is ambiguous; move or rename the existing file before initializing RGT",
            ));
        }
        let backup = existing_backup_path(path);
        editor::write_config_with_backup(path, content)?;
        return Ok(WriteOutcome::migrated(path, backup));
    }
    editor::write_config(path, content)?;
    Ok(WriteOutcome::configured(path, None))
}

fn is_rgt_owned_plugin(text: &str, agent: &str) -> bool {
    if text.contains("RGT-managed integration. rgt init may safely replace this file.") {
        return true;
    }
    match agent {
        "opencode" => {
            text.contains("plugin(\"rgt\"")
                && text.contains("\"--agent\", \"opencode\"")
                && text.contains("\"hook\", \"post\"")
        }
        "pi" => {
            text.contains("name: \"rgt\"")
                && text.contains("\"--agent\", \"pi\"")
                && text.contains("\"hook\", \"post\"")
        }
        "hermes" => {
            text.contains("def post_tool_call(args):")
                && text.contains("\"hook\", \"post\", \"--agent\", \"hermes\"")
        }
        _ => false,
    }
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
        PathBuf::from(".opencode").join("plugins")
    };
    let plugin_file = plugin_dir.join("rgt.ts");
    let content = glue::opencode_plugin(&rgt_path.display().to_string());
    let legacy_file = PathBuf::from(".opencode/plugin/rgt.ts");
    let migrate_legacy = !global
        && legacy_file.exists()
        && is_rgt_owned_plugin(&editor::read_config(&legacy_file)?, "opencode");
    let outcome = write_plugin_file(&plugin_file, &content, "opencode", force)?;
    if !migrate_legacy {
        return Ok(outcome);
    }
    let backup = editor::backup_before_write(&legacy_file).map_err(|e| {
        ConfigEditError::new(
            Some(legacy_file.display().to_string()),
            format!("failed to back up obsolete OpenCode plugin: {e}"),
        )
    })?;
    fs::remove_file(&legacy_file).map_err(|e| {
        ConfigEditError::new(
            Some(legacy_file.display().to_string()),
            format!("failed to remove obsolete OpenCode plugin: {e}"),
        )
    })?;
    Ok(WriteOutcome::combine(
        &plugin_file,
        [outcome, WriteOutcome::migrated(&plugin_file, backup)],
    ))
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
    let plugin = write_plugin_file(&ext_file, &content, "pi", force)?;
    let guidance_file = crate::hooks::paths::pi_guidance_path(home, global);
    let guidance = ensure_text_block(
        &guidance_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(&codex_section()),
        force,
    )?;
    Ok(WriteOutcome::combine(&ext_file, [plugin, guidance]))
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
    let written_plugin = write_plugin_file(&plugin_file, &content, "hermes", force)?;

    let config_file = if global {
        home.join(".hermes").join("config.toml")
    } else {
        PathBuf::from(".hermes").join("config.toml")
    };
    let config = ensure_toml(&config_file, |t| {
        editor::toml_ensure_array_value(t, &["plugins", "enabled"], "rgt")
    })?;

    let guidance_file = crate::hooks::paths::hermes_guidance_path();
    let guidance = ensure_text_block(
        &guidance_file,
        glue::RGT_MARKER,
        glue::RGT_END_MARKER,
        &glue::with_instruction(&codex_section()),
        force,
    )?;

    Ok(WriteOutcome::combine(
        &plugin_file,
        [written_plugin, config, guidance],
    ))
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
