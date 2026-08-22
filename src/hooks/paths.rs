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
