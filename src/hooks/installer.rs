use std::fs;
use std::io;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct DetectedTool {
    pub name: String,
    pub config_path: PathBuf,
    pub is_configured: bool,
}

pub fn detect_and_configure_hooks(
    global: bool,
    force: bool,
    agent: Option<&str>,
) -> io::Result<Vec<String>> {
    let mut configured = Vec::new();
    let home = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;

    let all_agents = agent.is_none();

    // 1. Claude Code (~/.claude/settings.json)
    if all_agents || agent == Some("claude-code") {
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
    }

    // 2. Cursor (~/.cursor/hooks.json)
    if all_agents || agent == Some("cursor") {
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
    }

    // 3. Codex CLI (AGENTS.md integration)
    if all_agents || agent == Some("codex") {
        let agents_file = PathBuf::from("AGENTS.md");
        let codex_section = "\n## RGT Integration\n\n\
            For shell commands, use the `rgt` prefix to get optimized output:\n\
            - `rgt git status`, `rgt cargo test`, `rgt ls`, `rgt grep`\n\
            - `rgt npm install`, `rgt pip install`, `rgt docker ps`\n\
            - Run `rgt --help` for supported commands\n";

        let write_file = force
            || !agents_file.exists()
            || !fs::read_to_string(&agents_file)
                .unwrap_or_default()
                .contains("## RGT Integration");

        if write_file {
            if force || !agents_file.exists() {
                fs::write(
                    &agents_file,
                    format!("# RGT Agents Instructions\n{}", codex_section),
                )?;
            } else {
                use std::io::Write;
                let mut file = fs::OpenOptions::new().append(true).open(&agents_file)?;
                file.write_all(codex_section.as_bytes())?;
            }
        }

        configured.push(format!(
            "Codex CLI (rules-file) -> {}",
            agents_file.display()
        ));
    }

    // 4. Windsurf (.windsurfrules)
    if all_agents || agent == Some("windsurf") {
        let rules_file = PathBuf::from(".windsurfrules");
        let rules_content = "# RGT Integration\n\
            When executing shell commands such as git, cargo, npm, ls, grep,\n\
            use the `rgt` prefix for optimized, token-efficient output.\n\
            Example: `rgt git status`, `rgt cargo test`, `rgt ls`.\n\
            Run `rgt --help` for available commands.\n\
            Learn more: https://github.com/rafael-bianchi/rgt\n";

        if force || !rules_file.exists() {
            fs::write(&rules_file, rules_content)?;
            configured.push(format!("Windsurf (rules-file) -> {}", rules_file.display()));
        }
    }

    Ok(configured)
}
