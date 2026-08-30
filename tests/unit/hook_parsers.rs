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

// ---------------------------------------------------------------------------
// 026: capture Bash-tool reads (rtk wrapper, grep, stdout, dispatch fallback)
// ---------------------------------------------------------------------------

fn bash_payload(command: &str, stdout: Option<&str>) -> String {
    let stdout_json = match stdout {
        Some(s) => format!("\"stdout\": {s:?}"),
        None => "\"stdout\": \"\"".to_string(),
    };
    format!(
        r#"{{"event":"PostToolUse","tool_name":"Bash","tool_input":{{"command":{command:?}}},"tool_response":{{{stdout_json}}}}}"#
    )
}

// ---- T002: rtk wrapper, grep, stdout (foundational) ----

#[test]
fn rtk_read_wrapper_extracts_path() {
    assert_eq!(
        extract_path_from_command("rtk read /path/report.csv"),
        Some("/path/report.csv".to_string())
    );
}

#[test]
fn rtk_cat_is_not_a_read_subcommand() {
    assert_eq!(extract_path_from_command("rtk cat /path/report.csv"), None);
}

#[test]
fn grep_is_a_read_verb() {
    assert_eq!(
        extract_path_from_command("grep -r amount /path/report.csv"),
        Some("/path/report.csv".to_string())
    );
}

#[test]
fn plain_cat_still_works() {
    assert_eq!(
        extract_path_from_command("cat /path/report.csv"),
        Some("/path/report.csv".to_string())
    );
}

#[test]
fn command_dialect_captures_stdout_as_content() {
    let cap = normalize_agent_event(
        Some("opencode"),
        &bash_payload("cat /path/report.csv", Some("amount,120000\n")),
    )
    .unwrap();
    assert_eq!(cap.path.as_deref(), Some("/path/report.csv"));
    assert_eq!(cap.content.as_deref(), Some("amount,120000\n"));
}

// ---- T005 (US1): dispatch fallback for claude-code/cursor ----

#[test]
fn bash_payload_is_captured_for_claude_and_cursor() {
    let payload = bash_payload("cat /path/report.csv", Some("amount,120000\n"));
    for agent in [None, Some("claude-code"), Some("cursor")] {
        let cap = normalize_agent_event(agent, &payload).unwrap();
        assert_eq!(cap.path.as_deref(), Some("/path/report.csv"), "{agent:?}");
        assert_eq!(cap.content.as_deref(), Some("amount,120000\n"), "{agent:?}");
    }
}

#[test]
fn native_read_event_is_unchanged() {
    let payload = r#"{"event":"PostToolUse","tool_name":"ReadLocalFile","tool_input":{"path":"docs/budget.md"},"tool_response":{"content":"Budget: 15000"}}"#;
    for agent in [None, Some("claude-code"), Some("cursor")] {
        let cap = normalize_agent_event(agent, payload).unwrap();
        assert_eq!(cap.path.as_deref(), Some("docs/budget.md"));
        assert_eq!(cap.content.as_deref(), Some("Budget: 15000"));
    }
}

// ---- T007 (US2): rtk end-to-end ----

#[test]
fn rtk_read_payload_is_captured_end_to_end() {
    let payload = bash_payload("rtk read /path/report.csv", Some("amount,42\n"));
    let cap = normalize_agent_event(None, &payload).unwrap();
    assert_eq!(cap.path.as_deref(), Some("/path/report.csv"));
    assert_eq!(cap.content.as_deref(), Some("amount,42\n"));
}

#[test]
fn rtk_non_read_payload_is_a_noop() {
    let payload = bash_payload("rtk git status", None);
    let cap = normalize_agent_event(None, &payload).unwrap();
    assert_eq!(cap.path, None);
}

// ---- T009 (US3): fallback / passive ----

#[test]
fn empty_stdout_is_path_only() {
    let payload = bash_payload("cat /path/report.csv", None);
    let cap = normalize_agent_event(None, &payload).unwrap();
    assert_eq!(cap.path.as_deref(), Some("/path/report.csv"));
    assert_eq!(
        cap.content, None,
        "empty stdout -> content None (disk fallback)"
    );
}

#[test]
fn non_read_command_is_a_noop() {
    for cmd in ["ls", "git status", "echo hi"] {
        let cap = normalize_agent_event(None, &bash_payload(cmd, Some("x"))).unwrap();
        assert_eq!(cap.path, None, "{}", cmd);
    }
}

// ---- T007 (US1): signs & accounting parentheses ----

fn nums(content: &str, format: rgt::hooks::parser::NumberFormat) -> Vec<f64> {
    use rgt::hooks::parser::extract_values_from_content;
    extract_values_from_content(content, format)
        .values
        .into_iter()
        .map(|v| match v.value {
            rgt::types::ValueData::Number(n) => n,
            _ => panic!("expected Number"),
        })
        .collect()
}

fn skip_count(content: &str, format: rgt::hooks::parser::NumberFormat) -> usize {
    use rgt::hooks::parser::extract_values_from_content;
    extract_values_from_content(content, format).skipped_malformed
}

#[test]
fn sign_minus_is_part_of_the_number() {
    assert_eq!(
        nums("-42\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![-42.0]
    );
}

#[test]
fn unary_plus_is_tolerated() {
    assert_eq!(
        nums("+42\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![42.0]
    );
}

#[test]
fn accounting_parentheses_are_negative() {
    assert_eq!(
        nums("(500)\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![-500.0]
    );
}

#[test]
fn minus_before_parentheses_is_positive() {
    assert_eq!(
        nums("-(500)\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![500.0]
    );
}

#[test]
fn sign_only_attaches_when_not_preceded_by_word_char() {
    // `abc-123` is an identifier hyphen, not a negative; sign is dropped.
    assert_eq!(
        nums("abc-123\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![123.0]
    );
}

// ---- T009 (US2): separators per format ----

#[test]
fn us_thousands_and_decimal_are_one_node() {
    let n = nums("1,234.56\n", rgt::hooks::parser::NumberFormat::Us);
    assert_eq!(n, vec![1234.56]);
}

#[test]
fn eu_thousands_and_decimal_are_one_node() {
    let n = nums("1.234,56\n", rgt::hooks::parser::NumberFormat::Eu);
    assert_eq!(n, vec![1234.56]);
}

#[test]
fn us_bare_thousands_integer() {
    assert_eq!(
        nums("1,234\n", rgt::hooks::parser::NumberFormat::Us),
        vec![1234.0]
    );
}

#[test]
fn eu_bare_comma_is_decimal() {
    assert_eq!(
        nums("1,234\n", rgt::hooks::parser::NumberFormat::Eu),
        vec![1.234]
    );
}

#[test]
fn currency_and_percent_symbols_are_dropped() {
    let n = nums("$1,234.56\n", rgt::hooks::parser::NumberFormat::Us);
    assert_eq!(n, vec![1234.56]);
    let n = nums("12.5%\n", rgt::hooks::parser::NumberFormat::Us);
    assert_eq!(n, vec![12.5]);
}

#[test]
fn malformed_number_is_skipped_and_counted_never_split() {
    let n = nums("1.234.567,89\n", rgt::hooks::parser::NumberFormat::Us);
    assert!(n.is_empty(), "malformed value must not emit any node");
    assert_eq!(
        skip_count("1.234.567,89\n", rgt::hooks::parser::NumberFormat::Us),
        1,
        "malformed value must be counted once"
    );
}

// ---- T015 (US4): auto inference, once per file ----

#[test]
fn auto_us_input_infers_us() {
    assert_eq!(
        nums("1,234.56\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![1234.56]
    );
}

#[test]
fn auto_eu_input_infers_eu() {
    assert_eq!(
        nums("1.234,56\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![1234.56]
    );
}

#[test]
fn auto_single_separator_falls_back_to_us() {
    assert_eq!(
        nums("1,234\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![1234.0]
    );
    assert_eq!(
        nums("1.234\n", rgt::hooks::parser::NumberFormat::Auto),
        vec![1.234]
    );
}

#[test]
fn auto_infers_once_per_file_and_applies_to_all_lines() {
    // First candidate with both separators (line 1) picks `.` as decimal -> us,
    // so line 2's eu-style number is malformed and skipped (never split).
    let content = "1,234.56\n1.234,56\n";
    let n = nums(content, rgt::hooks::parser::NumberFormat::Auto);
    assert_eq!(n, vec![1234.56]);
    assert_eq!(
        skip_count(content, rgt::hooks::parser::NumberFormat::Auto),
        1
    );
}

#[test]
fn auto_consistent_us_file_extracts_all_lines() {
    let content = "1,234.56\n2,345.67\n";
    assert_eq!(
        nums(content, rgt::hooks::parser::NumberFormat::Auto),
        vec![1234.56, 2345.67]
    );
}
