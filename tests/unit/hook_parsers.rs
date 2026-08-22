use rgt::hooks::parser::{extract_path_from_command, normalize_agent_event};

#[test]
fn default_dialect_extracts_path_and_content() {
    let cap = normalize_agent_event(
        None,
        r#"{
            "event": "PostToolUse",
            "tool_name": "ReadLocalFile",
            "tool_input": { "path": "docs/budget.md", "file_path": "docs/budget.md" },
            "tool_response": { "content": "Budget: 15000" }
        }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("docs/budget.md"));
    assert_eq!(cap.content.as_deref(), Some("Budget: 15000"));
}

#[test]
fn default_dialect_missing_response_content_yields_path_only() {
    let cap = normalize_agent_event(
        Some("claude-code"),
        r#"{ "tool_input": { "path": "/tmp/a.json" } }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/tmp/a.json"));
    assert!(cap.content.is_none());
}

#[test]
fn copilot_vscode_snake_case() {
    let cap = normalize_agent_event(
        Some("copilot"),
        r#"{
            "tool_name": "Read",
            "tool_input": { "path": "/abs/path.json", "file_path": "/abs/path.json" },
            "tool_response": { "content": "{\"count\": 42}" }
        }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/abs/path.json"));
    assert_eq!(cap.content.as_deref(), Some(r#"{"count": 42}"#));
}

#[test]
fn copilot_cli_camelcase_with_stringified_tool_args() {
    let cap = normalize_agent_event(
        Some("copilot"),
        r#"{ "toolName": "read_file", "toolArgs": "{\"file_path\": \"/abs/path.json\"}" }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/abs/path.json"));
    assert!(cap.content.is_none());
}

#[test]
fn copilot_cli_command_in_tool_args() {
    let cap = normalize_agent_event(
        Some("copilot"),
        r#"{ "toolName": "shell", "toolArgs": "{\"command\": \"cat /abs/data.csv\"}" }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/abs/data.csv"));
}

#[test]
fn gemini_command_dialect_extracts_path() {
    let cap = normalize_agent_event(
        Some("gemini"),
        r#"{ "tool_name": "run_shell_command", "tool_input": { "command": "cat /abs/path.json" } }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/abs/path.json"));
    assert!(cap.content.is_none());
}

#[test]
fn vibe_command_dialect_extracts_path() {
    let cap = normalize_agent_event(
        Some("vibe"),
        r#"{
            "tool_name": "bash",
            "tool_input": { "command": "head -n 5 /tmp/data.yaml" },
            "hook_event_name": "pre_tool",
            "session_id": "abc"
        }"#,
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/tmp/data.yaml"));
}

#[test]
fn vibe_non_read_command_yields_no_path() {
    let cap = normalize_agent_event(
        Some("vibe"),
        r#"{ "tool_input": { "command": "git status" } }"#,
    )
    .unwrap();
    assert!(cap.path.is_none());
    assert!(cap.content.is_none());
}

#[test]
fn plugin_agents_use_command_fallback() {
    for agent in ["opencode", "pi", "hermes"] {
        let cap = normalize_agent_event(
            Some(agent),
            r#"{ "args": { "command": "cat /tmp/plugin.json" } }"#,
        )
        .unwrap();
        assert_eq!(
            cap.path.as_deref(),
            Some("/tmp/plugin.json"),
            "agent {}",
            agent
        );
    }
}

#[test]
fn malformed_json_yields_noop() {
    assert!(normalize_agent_event(None, "not json").is_none());
    assert!(normalize_agent_event(Some("copilot"), "not json").is_none());
    assert!(normalize_agent_event(Some("gemini"), "not json").is_none());
}

#[test]
fn empty_payload_yields_noop_path() {
    let cap = normalize_agent_event(None, "{}").unwrap();
    assert!(cap.path.is_none());
    assert!(cap.content.is_none());
}

#[test]
fn unknown_agent_yields_noop() {
    assert!(normalize_agent_event(Some("nonsense"), r#"{ "a": 1 }"#).is_none());
}

#[test]
fn no_normalizer_returns_error() {
    // Fail-open contract: malformed input never propagates an error.
    for input in ["", "{", "[]", "42", "null", "not json at all"] {
        for agent in [None, Some("claude-code"), Some("copilot"), Some("gemini")] {
            let result = normalize_agent_event(agent, input);
            assert!(
                result.is_none() || result.unwrap().path.is_none(),
                "input {:?}",
                input
            );
        }
    }
}

#[test]
fn extract_path_from_command_read_styles() {
    for (cmd, expected) in [
        ("cat /a/b.json", Some("/a/b.json")),
        ("read /a/b.json", Some("/a/b.json")),
        ("less /a/b.json", Some("/a/b.json")),
        ("head -n 3 /a/b.json", Some("/a/b.json")),
        ("tail -f /a/b.log", Some("/a/b.log")),
        ("more /a/b.txt", Some("/a/b.txt")),
        ("python /a/script.py", Some("/a/script.py")),
        ("python3 /a/script.py", Some("/a/script.py")),
    ] {
        assert_eq!(
            extract_path_from_command(cmd).as_deref(),
            expected,
            "cmd: {}",
            cmd
        );
    }
}

#[test]
fn extract_path_from_command_non_read_returns_none() {
    assert!(extract_path_from_command("git status").is_none());
    assert!(extract_path_from_command("ls -la").is_none());
    assert!(extract_path_from_command("echo hello").is_none());
    assert!(extract_path_from_command("").is_none());
    assert!(extract_path_from_command("cat").is_none());
}
