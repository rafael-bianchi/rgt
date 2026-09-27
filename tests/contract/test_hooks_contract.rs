#[cfg(test)]
mod tests {
    use rgt::hooks::parser::{extract_values_from_content, parse_hook_payload, NumberFormat};
    use rgt::types::ValueKind;
    use tempfile::tempdir;

    #[test]
    fn test_parse_post_tool_use_payload() {
        let payload_json = r#"{
            "event": "PostToolUse",
            "tool_name": "ReadLocalFile",
            "tool_input": { "path": "docs/budget.md" },
            "tool_response": { "content": "Budget: 15000. Start Date: 2026-01-01" }
        }"#;

        let parsed = parse_hook_payload(payload_json).unwrap();
        assert_eq!(parsed.event.as_deref(), Some("PostToolUse"));
        assert_eq!(
            parsed.tool_input.as_ref().unwrap().path.as_deref(),
            Some("docs/budget.md")
        );

        let content = parsed
            .tool_response
            .as_ref()
            .unwrap()
            .content
            .as_deref()
            .unwrap();
        let extracted = extract_values_from_content(content, NumberFormat::Auto).values;
        assert!(!extracted.is_empty());
        assert!(extracted
            .iter()
            .any(|v| v.value.kind() == ValueKind::Number));
        assert!(extracted.iter().any(|v| v.value.kind() == ValueKind::Date));
    }

    #[test]
    fn only_completed_exact_snapshot_records_values_lines_fingerprint_and_agent() {
        use rgt::detection::compute_blake3_hash_from_bytes;
        use rgt::store::queries::{
            get_source_document_by_path, list_all_nodes, list_capture_associations,
        };
        use std::io::Write;
        use std::process::{Command, Stdio};

        fn run_hook(dir: &std::path::Path, event: &str) -> std::process::Output {
            let mut child = Command::new(env!("CARGO_BIN_EXE_rgt"))
                .args(["hook", "post", "--agent", "claude-code"])
                .current_dir(dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(event.as_bytes())
                .unwrap();
            child.wait_with_output().unwrap()
        }

        let dir = tempdir().unwrap();
        let file = dir.path().join("observed.csv");
        let file_path = file.to_string_lossy().into_owned();
        let content = "amount 42\ncreated 2026-01-01\n";
        std::fs::write(&file, content).unwrap();
        assert!(Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let success = serde_json::json!({
            "event": "PostToolUse",
            "tool_name": "Read",
            "tool_input": {"file_path": &file_path},
            "tool_response": {"content": content}
        })
        .to_string();
        assert!(run_hook(dir.path(), &success).status.success());

        let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let nodes = list_all_nodes(db.conn()).unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes
            .iter()
            .any(|node| { node.value.to_string_repr() == "42" && node.line_number == Some(1) }));
        assert!(nodes.iter().any(|node| {
            node.value.to_string_repr().starts_with("2026-01-01") && node.line_number == Some(2)
        }));
        let source = get_source_document_by_path(db.conn(), &file_path)
            .unwrap()
            .unwrap();
        assert_eq!(
            source.blake3_hash,
            compute_blake3_hash_from_bytes(content.as_bytes())
        );
        let associations = list_capture_associations(db.conn()).unwrap();
        assert_eq!(associations.len(), 2);
        assert!(associations.iter().all(|(_, agent)| agent == "claude-code"));

        // Duplicate success is idempotent. Path-only, partial, before, and
        // failed events, plus a changed snapshot, add no values or attribution.
        assert!(run_hook(dir.path(), &success).status.success());
        for event in [
            serde_json::json!({"event":"PostToolUse","tool_input":{"path":&file_path}}).to_string(),
            serde_json::json!({"event":"PostToolUse","tool_input":{"path":&file_path},"tool_response":{"content":content,"partial":true}}).to_string(),
            serde_json::json!({"event":"PreToolUse","tool_input":{"path":&file_path},"tool_response":{"content":content}}).to_string(),
            serde_json::json!({"event":"PostToolUse","success":false,"tool_input":{"path":&file_path},"tool_response":{"content":content}}).to_string(),
        ] {
            assert!(run_hook(dir.path(), &event).status.success());
        }
        std::fs::write(&file, "amount 99\n").unwrap();
        assert!(run_hook(dir.path(), &success).status.success());
        assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 2);
        assert_eq!(list_capture_associations(db.conn()).unwrap().len(), 2);
    }

    #[test]
    fn explicit_error_fields_never_create_values_or_capture_associations() {
        use rgt::store::queries::{list_all_nodes, list_capture_associations};
        use std::io::Write;
        use std::process::{Command, Stdio};

        for agent in ["claude-code", "codex"] {
            let dir = tempdir().unwrap();
            let file = dir.path().join("source.txt");
            let content = "amount 47\n";
            std::fs::write(&file, content).unwrap();
            let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
            let base = serde_json::json!({
                "event": "PostToolUse",
                "tool_name": "Read",
                "tool_input": {"file_path": file},
                "tool_response": {"content": content}
            });
            for (label, mut event) in [
                ("root-error", base.clone()),
                ("root-isError", base.clone()),
                ("result-isError", base.clone()),
            ] {
                match label {
                    "root-error" => event["error"] = serde_json::json!("read failed"),
                    "root-isError" => event["isError"] = serde_json::json!(true),
                    "result-isError" => event["tool_response"]["isError"] = serde_json::json!(true),
                    _ => unreachable!(),
                }
                let mut child = Command::new(env!("CARGO_BIN_EXE_rgt"))
                    .args(["hook", "post", "--agent", agent])
                    .current_dir(dir.path())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap();
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(event.to_string().as_bytes())
                    .unwrap();
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success(), "{agent}/{label}");
                assert!(output.stdout.is_empty(), "{agent}/{label}");
                assert!(
                    list_all_nodes(db.conn()).unwrap().is_empty(),
                    "{agent}/{label}"
                );
                assert!(
                    list_capture_associations(db.conn()).unwrap().is_empty(),
                    "{agent}/{label}"
                );
            }
        }
    }

    #[test]
    fn all_registered_dialects_return_a_neutral_noop_for_resultless_events() {
        use std::io::Write;
        use std::process::{Command, Stdio};

        for agent in [
            "claude-code",
            "cursor",
            "copilot",
            "gemini",
            "vibe",
            "opencode",
            "pi",
            "hermes",
            "codex",
            "windsurf",
            "cline",
            "antigravity",
            "kilocode",
        ] {
            let input = serde_json::json!({
                "event": "PostToolUse",
                "tool_name": "Read",
                "tool_input": {"file_path": "/no/such/source.txt"}
            })
            .to_string();
            let mut child = Command::new(env!("CARGO_BIN_EXE_rgt"))
                .args(["hook", "post", "--agent", agent])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            let output = child.wait_with_output().unwrap();
            assert!(output.status.success(), "{agent} must fail open");
            assert!(
                output.stdout.is_empty(),
                "{agent} neutral no-op must be empty"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn missing_direct_hook_executable_is_neutral_under_shell_fallback() {
        use std::process::Command;
        let output = Command::new("sh")
            .args([
                "-c",
                "'/definitely/moved/rgt' hook post --agent codex 2>/dev/null || true",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
