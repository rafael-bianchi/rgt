use oxttl::TurtleParser;
use rgt::store::{queries::list_capture_associations, DbStore};
use rgt::types::ValueData;
use std::io::Write;
use std::sync::Mutex;
use tempfile::tempdir;

static CWD_MUTEX: Mutex<()> = Mutex::new(());

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_rgt")
}

fn hook(project: &std::path::Path, agent: &str, payload: &str) -> std::process::Output {
    let mut child = std::process::Command::new(bin())
        .args(["hook", "post", "--agent", agent])
        .current_dir(project)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn rgt hook");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn hook_default_agent(project: &std::path::Path, payload: &str) -> std::process::Output {
    let mut child = std::process::Command::new(bin())
        .args(["hook", "post"])
        .current_dir(project)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn default-agent hook");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn eight_event_agents_persist_deduplicated_capture_lineage_across_processes() {
    let _cwd_guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let project = tempdir().unwrap();
    std::env::set_current_dir(project.path()).unwrap();
    assert!(std::process::Command::new(bin())
        .args(["init", "--agent", "codex"])
        .current_dir(project.path())
        .output()
        .unwrap()
        .status
        .success());
    std::fs::write(project.path().join("data.csv"), "captured,42.5\n").unwrap();

    let agents = [
        "claude-code",
        "cursor",
        "copilot",
        "gemini",
        "vibe",
        "opencode",
        "pi",
        "hermes",
    ];
    for agent in agents {
        let payload = if matches!(agent, "claude-code" | "cursor" | "copilot") {
            serde_json::json!({
                "tool_name": "Read",
                "tool_input": {"path": "data.csv"},
                "tool_response": {"content": "captured,42.5\n"}
            })
        } else {
            serde_json::json!({
                "tool_name": "Bash",
                "tool_input": {"command": "cat data.csv"},
                "tool_response": {"stdout": "captured,42.5\n"}
            })
        };
        let agent_arg = if agent == "claude-code" {
            "claude"
        } else {
            agent
        };
        let output = hook(project.path(), agent_arg, &payload.to_string());
        assert!(
            output.status.success(),
            "{agent}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let cursor_repeat = serde_json::json!({
        "tool_name": "Read",
        "tool_input": {"file_path": "data.csv"},
        "tool_response": {"content": "captured,42.5\n"}
    });
    assert!(hook(project.path(), "cursor", &cursor_repeat.to_string())
        .status
        .success());
    let omitted_agent = serde_json::json!({
        "tool_name": "Read",
        "tool_input": {"path": "data.csv"},
        "tool_response": {"content": "captured,42.5\n"}
    });
    assert!(
        hook_default_agent(project.path(), &omitted_agent.to_string())
            .status
            .success()
    );

    std::fs::write(project.path().join("manual.csv"), "manual,77\n").unwrap();
    assert!(std::process::Command::new(bin())
        .args(["record", "manual.csv"])
        .current_dir(project.path())
        .output()
        .unwrap()
        .status
        .success());

    let rules_event = serde_json::json!({
        "tool_input": {"path": "data.csv"},
        "tool_response": {"content": "rules-only,88\n"}
    });
    assert!(hook(project.path(), "windsurf", &rules_event.to_string())
        .status
        .success());
    assert!(
        hook(project.path(), "unknown-agent", &rules_event.to_string())
            .status
            .success()
    );
    assert!(hook(project.path(), "roo-code", &rules_event.to_string())
        .status
        .success());
    assert!(hook(project.path(), "gemini", "{").status.success());

    let db = DbStore::open_in_project(project.path()).unwrap();
    let captures = list_capture_associations(db.conn()).unwrap();
    assert_eq!(captures.len(), agents.len());
    let actual_agents = captures
        .iter()
        .map(|(_, agent)| agent.as_str())
        .collect::<Vec<_>>();
    let mut expected_agents = agents.to_vec();
    expected_agents.sort_unstable();
    assert_eq!(actual_agents, expected_agents);
    let captured_node_id = captures[0].0.clone();
    assert!(captures
        .iter()
        .all(|(node_id, _)| node_id == &captured_node_id));
    assert!(!captures
        .iter()
        .any(|(_, agent)| agent == "claude" || agent == "roo-code"));
    let manual_id = rgt::types::TrackedNode::generate_root_id(
        "manual.csv",
        Some(1),
        0,
        &ValueData::Number(77.0),
    );
    assert!(!captures.iter().any(|(node_id, _)| node_id == &manual_id));

    // A value inserted before/without any event attribution stays unattributed.
    db.conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('historical-unattributed','ROOT','NUMBER',5.0,0,'2020-01-01T00:00:00Z','2020-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    drop(db);

    let output = std::process::Command::new(bin())
        .args(["graph", "--format", "ttl"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let triples = TurtleParser::new()
        .for_slice(&output.stdout)
        .collect::<Result<Vec<_>, _>>()
        .expect("fresh-process graph export must parse");
    assert_eq!(
        triples
            .iter()
            .filter(
                |triple| triple.predicate.as_str() == "http://www.w3.org/ns/prov#wasAssociatedWith"
            )
            .count(),
        agents.len()
    );
    assert_eq!(
        triples
            .iter()
            .filter(|triple| triple.predicate.as_str()
                == "https://github.com/rafael-bianchi/rgt/vocab#localNodeId")
            .count(),
        3,
        "event, manual, and historical values are preserved"
    );
}

#[test]
fn failed_capture_association_rolls_back_the_text_value_and_fails_open() {
    let _cwd_guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let project = tempdir().unwrap();
    std::env::set_current_dir(project.path()).unwrap();
    assert!(std::process::Command::new(bin())
        .args(["init", "--agent", "codex"])
        .current_dir(project.path())
        .output()
        .unwrap()
        .status
        .success());
    std::fs::write(project.path().join("data.csv"), "captured,42.5\n").unwrap();
    let db = DbStore::open_in_project(project.path()).unwrap();
    db.conn()
        .execute_batch(
            "CREATE TRIGGER fail_capture_association
             BEFORE INSERT ON capture_associations
             BEGIN SELECT RAISE(ABORT, 'injected capture write failure'); END;",
        )
        .unwrap();
    drop(db);

    let event = serde_json::json!({
        "tool_name": "Read",
        "tool_input": {"path": "data.csv"},
        "tool_response": {"content": "captured,42.5\n"}
    });
    let output = hook(project.path(), "cursor", &event.to_string());
    assert!(output.status.success(), "hook must fail open");
    let db = DbStore::open_in_project(project.path()).unwrap();
    let nodes: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM tracked_nodes", [], |row| row.get(0))
        .unwrap();
    let captures: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM capture_associations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        nodes, 0,
        "node insert must roll back with its failed association"
    );
    assert_eq!(captures, 0);
}
