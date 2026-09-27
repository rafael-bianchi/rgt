//! Platform-aware agent config path resolution (constitution cross-platform
//! parity). Paths are resolved from an explicit `home`/`config_dir` base so
//! unit tests can exercise every platform branch deterministically.

use std::path::{Path, PathBuf};

/// VS Code user settings file for the given platform's user config directory.
///
/// `config_dir` is the platform's per-user config directory:
/// - Windows: `%APPDATA%` (e.g. `C:\Users\<u>\AppData\Roaming`)
/// - Linux: `~/.config`
/// - macOS: `~/Library/Application Support`
pub fn vscode_user_settings(config_dir: &Path) -> PathBuf {
    config_dir.join("Code").join("User").join("settings.json")
}

/// Copilot Chat hook config target: VS Code user settings.
pub fn copilot_user_settings(config_dir: &Path) -> PathBuf {
    vscode_user_settings(config_dir)
}

/// Copilot CLI user config directory for a given platform.
///
/// The GitHub Copilot CLI stores its config at `~/.config/github-copilot/` on
/// both macOS and Linux, and at `%APPDATA%\github-copilot\` on Windows. Note
/// that on macOS this is NOT the `dirs::config_dir()` value
/// (`~/Library/Application Support`), which is why the base is resolved
/// explicitly per platform. Parameterized by `os` so every branch is
/// unit-testable on any host.
pub fn copilot_cli_config_dir_for(home: &Path, config_dir: &Path, os: &str) -> PathBuf {
    match os {
        "macos" => home.join(".config").join("github-copilot"),
        "windows" => config_dir.join("github-copilot"),
        // Linux: the standard config dir (respects `$XDG_CONFIG_HOME`).
        _ => config_dir.join("github-copilot"),
    }
}

/// Copilot CLI user config directory for the current host platform.
pub fn copilot_cli_config_dir(home: &Path, config_dir: &Path) -> PathBuf {
    copilot_cli_config_dir_for(home, config_dir, std::env::consts::OS)
}

/// Codex CLI hook registration for the selected scope.
pub fn codex_hooks_json(home: &Path, global: bool) -> PathBuf {
    if global {
        home.join(".codex").join("hooks.json")
    } else {
        PathBuf::from(".codex").join("hooks.json")
    }
}

/// Pi appends project instructions to its trusted system prompt. User
/// instructions use the override file when it already shadows AGENTS.md.
pub fn pi_guidance_path(home: &Path, global: bool) -> PathBuf {
    if !global {
        return PathBuf::from(".pi/APPEND_SYSTEM.md");
    }
    let user_dir = home.join(".pi/agent");
    let override_file = user_dir.join("AGENTS.override.md");
    if override_file.exists() {
        override_file
    } else {
        user_dir.join("AGENTS.md")
    }
}

/// Hermes loads one project context file. Select its first existing file in
/// documented precedence order so guidance is not hidden by a higher-priority
/// context file. AGENTS.md is the default when none exists.
pub fn hermes_guidance_path() -> PathBuf {
    [
        ".hermes.md",
        "HERMES.md",
        "AGENTS.override.md",
        "AGENTS.md",
        "CLAUDE.md",
        ".cursorrules",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.exists())
    .unwrap_or_else(|| PathBuf::from("AGENTS.md"))
}

/// Vibe reads project and user AGENTS.md files as context. Files under
/// ~/.vibe/prompts are selectable replacement system prompts, not auto-loaded
/// instructions.
pub fn vibe_guidance_path(home: &Path, global: bool) -> PathBuf {
    if global {
        home.join(".vibe/AGENTS.md")
    } else {
        PathBuf::from("AGENTS.md")
    }
}

/// Windsurf Cascade user-level hook file documented for the IDE.
pub fn windsurf_user_hooks_json(home: &Path) -> PathBuf {
    home.join(".codeium").join("windsurf").join("hooks.json")
}

/// Preferred workspace hook file. `.windsurf/hooks.json` is a legacy fallback
/// and is selected only when the preferred file does not already define hooks.
pub fn windsurf_workspace_hooks_json() -> PathBuf {
    PathBuf::from(".devin").join("hooks.json")
}

pub fn windsurf_legacy_workspace_hooks_json() -> PathBuf {
    PathBuf::from(".windsurf").join("hooks.json")
}

/// All accepted `--agent` spellings (13 canonical + 3 aliases).
pub const ALL_AGENT_NAMES: &[&str] = &[
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
];
