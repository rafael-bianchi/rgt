use std::fs;
use std::io;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct DetectedTool {
    pub name: String,
    pub config_path: PathBuf,
    pub is_configured: bool,
}

pub fn detect_and_configure_hooks(global: bool, force: bool) -> io::Result<Vec<String>> {
    let mut configured = Vec::new();
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;

    // 1. Claude Code (~/.claude/hooks.json or .claude/hooks.json)
    let claude_dir = if global {
        home.join(".claude")
    } else {
        PathBuf::from(".claude")
    };
    let claude_hooks_file = claude_dir.join("hooks.json");
    if force || !claude_hooks_file.exists() {
        if let Some(parent) = claude_hooks_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = serde_json::json!({
            "PostToolUse": {
                "command": "rgt hook post"
            },
            "PreToolUse": {
                "command": "rgt hook pre"
            }
        });
        fs::write(&claude_hooks_file, serde_json::to_string_pretty(&config)?)?;
        configured.push(format!("Claude Code -> {}", claude_hooks_file.display()));
    }

    // 2. Cursor (~/.cursor/hooks.json)
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
            "hooks": [
                { "event": "PostToolUse", "command": "rgt hook post" },
                { "event": "PreToolUse", "command": "rgt hook pre" }
            ]
        });
        fs::write(&cursor_hooks_file, serde_json::to_string_pretty(&config)?)?;
        configured.push(format!("Cursor -> {}", cursor_hooks_file.display()));
    }

    // 3. Codex CLI (~/.codex/config.json)
    let codex_dir = if global {
        home.join(".codex")
    } else {
        PathBuf::from(".codex")
    };
    let codex_hooks_file = codex_dir.join("hooks.json");
    if force || !codex_hooks_file.exists() {
        if let Some(parent) = codex_hooks_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = serde_json::json!({
            "rgt_hook": "rgt hook post"
        });
        fs::write(&codex_hooks_file, serde_json::to_string_pretty(&config)?)?;
        configured.push(format!("Codex CLI -> {}", codex_hooks_file.display()));
    }

    // 4. Windsurf (~/.codeium/windsurf/hooks.json)
    let windsurf_dir = if global {
        home.join(".codeium").join("windsurf")
    } else {
        PathBuf::from(".windsurf")
    };
    let windsurf_hooks_file = windsurf_dir.join("hooks.json");
    if force || !windsurf_hooks_file.exists() {
        if let Some(parent) = windsurf_hooks_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = serde_json::json!({
            "hooks": {
                "post_execution": "rgt hook post"
            }
        });
        fs::write(&windsurf_hooks_file, serde_json::to_string_pretty(&config)?)?;
        configured.push(format!("Windsurf -> {}", windsurf_hooks_file.display()));
    }

    Ok(configured)
}
