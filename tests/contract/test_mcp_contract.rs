#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Child, ChildStdin, ChildStdout, Stdio};
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn rgt() -> &'static str {
        env!("CARGO_BIN_EXE_rgt")
    }

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    /// A minimal JSON-RPC client for the `rgt mcp` stdio server.
    struct McpClient {
        child: Child,
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    }

    impl McpClient {
        fn spawn(dir: &std::path::Path) -> Self {
            let mut child = std::process::Command::new(rgt())
                .arg("mcp")
                .current_dir(dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn rgt mcp");
            let stdin = child.stdin.take().unwrap();
            let stdout = BufReader::new(child.stdout.take().unwrap());
            McpClient {
                child,
                stdin,
                stdout,
            }
        }

        fn send(&mut self, id: u64, method: &str, params: serde_json::Value) {
            let msg = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            });
            writeln!(self.stdin, "{}", msg).unwrap();
            self.stdin.flush().unwrap();
        }

        fn recv(&mut self) -> serde_json::Value {
            let mut line = String::new();
            self.stdout
                .read_line(&mut line)
                .expect("read MCP response line");
            serde_json::from_str(&line).expect("MCP response must be JSON")
        }

        fn initialize(&mut self) {
            self.send(
                1,
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "1.0"},
                }),
            );
            let resp = self.recv();
            assert_eq!(resp["id"], 1);
            assert!(
                resp["result"]["capabilities"]["tools"].is_object(),
                "{resp}"
            );
        }

        fn call_tool(
            &mut self,
            id: u64,
            name: &str,
            arguments: serde_json::Value,
        ) -> serde_json::Value {
            self.send(
                id,
                "tools/call",
                serde_json::json!({
                    "name": name,
                    "arguments": arguments,
                }),
            );
            self.recv()
        }

        fn text_content(&self, resp: &serde_json::Value) -> String {
            resp["result"]["content"][0]["text"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_default()
        }

        fn list_tools(&mut self) -> serde_json::Value {
            self.send(2, "tools/list", serde_json::json!({}));
            self.recv()
        }
    }

    impl Drop for McpClient {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn seed_store(dir: &std::path::Path) -> String {
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir)
            .output()
            .expect("spawn rgt init")
            .status
            .success());
        let file = dir.join("data.csv");
        std::fs::write(&file, "revenue,120000\ncosts,60000\n").unwrap();
        assert!(std::process::Command::new(rgt())
            .arg("record")
            .arg(&file)
            .current_dir(dir)
            .output()
            .expect("spawn rgt record")
            .status
            .success());
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let all = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        all[0].id.clone()
    }

    fn store_has_value(v: &str) -> bool {
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        rgt::store::queries::list_all_nodes(conn)
            .expect("list nodes")
            .iter()
            .any(|n| n.value.to_string_repr() == v)
    }

    fn node_id_for_value(v: &str) -> String {
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        rgt::store::queries::list_all_nodes(conn)
            .expect("list nodes")
            .into_iter()
            .find(|n| n.value.to_string_repr() == v)
            .expect("node with value")
            .id
    }

    fn seed_root(dir: &std::path::Path, value: f64) -> String {
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir)
            .output()
            .expect("spawn rgt init")
            .status
            .success());
        let file = dir.join("n.csv");
        std::fs::write(&file, format!("x,{}\n", value)).unwrap();
        assert!(std::process::Command::new(rgt())
            .arg("record")
            .arg(&file)
            .current_dir(dir)
            .output()
            .expect("spawn rgt record")
            .status
            .success());
        node_id_for_value(&format!("{}", value))
    }

    // -----------------------------------------------------------------------
    // 041 / US1 (SC-001): initialize + tools/list + query_provenance
    // -----------------------------------------------------------------------

    #[test]
    fn mcp_server_initializes_and_lists_tools() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let tools = client.list_tools();
        assert_eq!(tools["id"], 2);
        let names: Vec<String> = tools["result"]["tools"]
            .as_array()
            .expect("tools must be an array")
            .iter()
            .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
            .collect();
        assert!(
            names.contains(&"query_provenance".to_string()),
            "query_provenance must be listed: {names:?}"
        );
    }

    #[test]
    fn query_provenance_returns_lineage_via_mcp() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let node_id = seed_store(dir.path());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        client.send(
            3,
            "tools/call",
            serde_json::json!({
                "name": "query_provenance",
                "arguments": { "node_id": node_id },
            }),
        );
        let resp = client.recv();
        assert_eq!(resp["id"], 3);
        let content = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("tool result text");
        let lineage: serde_json::Value =
            serde_json::from_str(content).expect("tool result must embed lineage JSON");
        assert_eq!(lineage["node_id"].as_str(), Some(node_id.as_str()));
        assert!(lineage["lineage_steps"].is_array());
    }

    #[test]
    fn query_provenance_unknown_node_returns_error_and_server_survives() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        client.send(
            3,
            "tools/call",
            serde_json::json!({
                "name": "query_provenance",
                "arguments": { "node_id": "nonexistent" },
            }),
        );
        let resp = client.recv();
        assert_eq!(resp["id"], 3);
        assert!(
            resp["result"]["isError"] == true,
            "unknown node must return an MCP tool error result: {resp}"
        );
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .contains("Node not found"),
            "error text must carry the message: {resp}"
        );

        // The server must survive the error: a follow-up tools/list still works.
        let tools = client.list_tools();
        assert_eq!(tools["id"], 2);
        assert!(tools["result"]["tools"].is_array(), "{tools}");
    }

    // -----------------------------------------------------------------------
    // 041 / US2 (SC-001): record_values, verify_only, derive_and_verify,
    // get_status, and server resilience on invalid input
    // -----------------------------------------------------------------------

    #[test]
    fn mcp_lists_all_five_tools() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let tools = client.list_tools();
        let names: Vec<String> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
            .collect();
        for expected in [
            "query_provenance",
            "record_values",
            "verify_only",
            "derive_and_verify",
            "get_status",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "missing {expected}: {names:?}"
            );
        }
    }

    #[test]
    fn record_values_via_mcp_stores_values_and_returns_outcome() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());
        let file = dir.path().join("m.csv");
        std::fs::write(&file, "amount,777\n").unwrap();

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        let resp = client.call_tool(
            3,
            "record_values",
            serde_json::json!({
                "file": file.to_string_lossy(),
                "content": serde_json::Value::Null,
                "number_format": serde_json::Value::Null,
            }),
        );
        let text = client.text_content(&resp);
        let outcome: serde_json::Value = serde_json::from_str(&text).expect("outcome JSON");
        assert_eq!(outcome["count"], 1, "{text}");
        assert!(store_has_value("777"), "value must be recorded via MCP");

        // Server survives: another call works.
        let resp2 = client.call_tool(4, "get_status", serde_json::json!({}));
        assert_eq!(resp2["id"], 4);
        let status: serde_json::Value =
            serde_json::from_str(&client.text_content(&resp2)).expect("status JSON");
        assert_eq!(status["total_nodes"], 1);
    }

    #[test]
    fn verify_only_checks_without_writing() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let parent = seed_root(dir.path(), 10.0);

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        // Correct derivation: succeeds.
        let ok = client.call_tool(
            3,
            "verify_only",
            serde_json::json!({
                "parents": parent,
                "operation": "EXPRESSION",
                "expression": "a * 2",
                "result": 20.0,
            }),
        );
        assert_eq!(
            ok["result"]["isError"],
            serde_json::Value::Bool(false),
            "{ok}"
        );

        // Wrong result: fails, records nothing.
        let bad = client.call_tool(
            4,
            "verify_only",
            serde_json::json!({
                "parents": parent,
                "operation": "EXPRESSION",
                "expression": "a * 2",
                "result": 99.0,
            }),
        );
        assert_eq!(
            bad["result"]["isError"],
            serde_json::Value::Bool(true),
            "{bad}"
        );
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let nodes = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        let derived = nodes
            .iter()
            .filter(|n| n.node_type == rgt::types::NodeType::Derived)
            .count();
        assert_eq!(derived, 0, "verify_only must never write to the graph");
    }

    #[test]
    fn derive_and_verify_records_on_success_and_errors_on_failure() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let parent = seed_root(dir.path(), 10.0);

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        // Wrong result first: errors, records nothing.
        let bad = client.call_tool(
            3,
            "derive_and_verify",
            serde_json::json!({
                "parents": parent,
                "operation": "EXPRESSION",
                "expression": "a * 2",
                "result": 99.0,
            }),
        );
        assert_eq!(
            bad["result"]["isError"],
            serde_json::Value::Bool(true),
            "{bad}"
        );
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let nodes = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.node_type == rgt::types::NodeType::Derived)
                .count(),
            0,
            "failed derive must not record"
        );

        // Correct result: records a derived node.
        let ok = client.call_tool(
            4,
            "derive_and_verify",
            serde_json::json!({
                "parents": parent,
                "operation": "EXPRESSION",
                "expression": "a * 2",
                "result": 20.0,
            }),
        );
        assert_eq!(
            ok["result"]["isError"],
            serde_json::Value::Bool(false),
            "{ok}"
        );
        let text = client.text_content(&ok);
        let resp: serde_json::Value = serde_json::from_str(&text).expect("derive JSON");
        assert!(
            resp["node_id"]
                .as_str()
                .unwrap_or("")
                .starts_with("node_drv_"),
            "{resp}"
        );
    }

    #[test]
    fn invalid_input_to_derive_and_verify_errors_and_server_survives() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        // Unknown operation: must return a tool error, NOT terminate the server.
        let resp = client.call_tool(
            3,
            "derive_and_verify",
            serde_json::json!({
                "parents": "x",
                "operation": "bogus",
                "result": 1.0,
            }),
        );
        assert_eq!(
            resp["result"]["isError"],
            serde_json::Value::Bool(true),
            "{resp}"
        );
        assert!(
            client.text_content(&resp).contains("unknown operation"),
            "error text must carry the message: {resp}"
        );

        // Server survives: a subsequent tools/list still works.
        let tools = client.list_tools();
        assert_eq!(tools["id"], 2);
        assert!(tools["result"]["tools"].is_array(), "{tools}");
    }

    #[test]
    fn get_status_returns_graph_summary() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        seed_root(dir.path(), 42.0);

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        let resp = client.call_tool(3, "get_status", serde_json::json!({}));
        let status: serde_json::Value =
            serde_json::from_str(&client.text_content(&resp)).expect("status JSON");
        assert_eq!(status["total_nodes"], 1);
        assert_eq!(status["active_nodes"], 1);
        assert_eq!(status["stale_nodes"], 0);
    }

    #[test]
    fn record_values_with_inline_content_and_get_status_staleness_sync() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let file = dir.path().join("inline.csv");
        std::fs::write(&file, "revenue,250\n").unwrap();

        let mut client = McpClient::spawn(dir.path());
        client.initialize();
        let _ = client.list_tools();

        let rec = client.call_tool(
            3,
            "record_values",
            serde_json::json!({
                "file": file.to_string_lossy(),
                "content": "revenue,250\n",
            }),
        );
        assert_eq!(
            rec["result"]["isError"],
            serde_json::Value::Bool(false),
            "{rec}"
        );

        std::fs::remove_file(&file).unwrap();
        let status_resp = client.call_tool(4, "get_status", serde_json::json!({}));
        let status: serde_json::Value =
            serde_json::from_str(&client.text_content(&status_resp)).expect("status JSON");
        assert_eq!(status["total_nodes"], 1);
        assert_eq!(status["stale_nodes"], 1);
    }

    #[test]
    fn derive_date_diff_without_result_and_export_graph_via_mcp() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let file = dir.path().join("dates.csv");
        std::fs::write(&file, "start,2026-01-01\nend,2026-01-11\n").unwrap();

        let mut client = McpClient::spawn(dir.path());
        client.initialize();

        let rec = client.call_tool(
            3,
            "record_values",
            serde_json::json!({
                "file": file.to_string_lossy(),
                "content": "start,2026-01-01\nend,2026-01-11\n",
            }),
        );
        assert_eq!(rec["result"]["isError"], serde_json::Value::Bool(false));

        let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(nodes.len(), 2);
        let id1 = &nodes[0].id;
        let id2 = &nodes[1].id;

        // Derive DATE_DIFF without result, specifying unit: "days"
        let derive_resp = client.call_tool(
            4,
            "derive_and_verify",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "unit": "days",
            }),
        );
        assert_eq!(
            derive_resp["result"]["isError"],
            serde_json::Value::Bool(false),
            "{derive_resp}"
        );
        let derive_data: serde_json::Value =
            serde_json::from_str(&client.text_content(&derive_resp)).expect("derive JSON");
        assert!(derive_data["node_id"].is_string());
        assert_eq!(derive_data["display"], "10 days");

        // Export graph via MCP
        let graph_resp = client.call_tool(
            5,
            "export_graph",
            serde_json::json!({
                "format": "mermaid",
            }),
        );
        assert_eq!(
            graph_resp["result"]["isError"],
            serde_json::Value::Bool(false),
            "{graph_resp}"
        );
        let graph_data: serde_json::Value =
            serde_json::from_str(&client.text_content(&graph_resp)).expect("graph JSON");
        assert_eq!(graph_data["format"], "mermaid");
        assert!(graph_data["graph"].as_str().unwrap().contains("graph TD"));
    }

    #[test]
    fn get_status_in_uninitialized_directory_does_not_create_store() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let mut client = McpClient::spawn(dir.path());
        client.initialize();

        let resp = client.call_tool(3, "get_status", serde_json::json!({}));
        let status: serde_json::Value =
            serde_json::from_str(&client.text_content(&resp)).expect("status JSON");
        assert_eq!(status["total_nodes"], 0);
        assert_eq!(status["active_nodes"], 0);
        assert_eq!(status["stale_nodes"], 0);

        assert!(
            !dir.path().join(".rgt").exists(),
            ".rgt directory must NOT be created by read-only MCP get_status"
        );
    }

    #[test]
    fn verify_only_supports_optional_result_for_temporal_and_rejects_invalid_result() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        let file = dir.path().join("dates.csv");
        std::fs::write(&file, "start,2026-01-01\nend,2026-01-11\n").unwrap();

        let mut client = McpClient::spawn(dir.path());
        client.initialize();

        let rec = client.call_tool(
            3,
            "record_values",
            serde_json::json!({
                "file": file.to_string_lossy(),
                "content": "start,2026-01-01\nend,2026-01-11\n",
            }),
        );
        assert_eq!(rec["result"]["isError"], serde_json::Value::Bool(false));

        let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(nodes.len(), 2);
        let id1 = &nodes[0].id;
        let id2 = &nodes[1].id;

        // 1. verify_only on DATE_DIFF without result and with unit: "days" succeeds and returns display
        let resp = client.call_tool(
            4,
            "verify_only",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "unit": "days",
            }),
        );
        assert_eq!(
            resp["result"]["isError"],
            serde_json::Value::Bool(false),
            "{resp}"
        );
        let body: serde_json::Value =
            serde_json::from_str(&client.text_content(&resp)).expect("verify JSON");
        assert_eq!(body["verified"], true);
        assert_eq!(body["display"], "10 days");
        assert_eq!(body["display_details"]["decimal"], "10");

        // 2b. verify_only and derive_and_verify with result in default seconds ("864000") and display unit: "days" (no result_unit)
        let ok_default_sec_with_display = client.call_tool(
            50,
            "verify_only",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "result": "864000",
                "unit": "days",
            }),
        );
        assert_eq!(
            ok_default_sec_with_display["result"]["isError"],
            serde_json::Value::Bool(false),
            "result without result_unit must default to seconds even when display unit is days: {ok_default_sec_with_display}"
        );
        let sec_body: serde_json::Value =
            serde_json::from_str(&client.text_content(&ok_default_sec_with_display)).unwrap();
        assert_eq!(sec_body["display"], "10 days");

        // 2c. verify_only with result_unit but no result must fail with --result-unit requires --result
        let orphan_result_unit = client.call_tool(
            51,
            "verify_only",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "result_unit": "days",
            }),
        );
        assert_eq!(
            orphan_result_unit["result"]["isError"],
            serde_json::Value::Bool(true),
            "result_unit without result must be rejected: {orphan_result_unit}"
        );
        let ok_claim = client.call_tool(
            5,
            "verify_only",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "result": "10",
                "result_unit": "days",
            }),
        );
        assert_eq!(
            ok_claim["result"]["isError"],
            serde_json::Value::Bool(false)
        );

        // 3. verify_only on DATE_DIFF with incorrect result claim fails
        let bad_claim = client.call_tool(
            6,
            "verify_only",
            serde_json::json!({
                "parents": format!("{id1},{id2}"),
                "operation": "DATE_DIFF",
                "result": "5",
                "result_unit": "days",
            }),
        );
        assert_eq!(
            bad_claim["result"]["isError"],
            serde_json::Value::Bool(true)
        );

        // 4. verify_only on EXPRESSION without result fails with "result is required"
        let no_res_expr = client.call_tool(
            7,
            "verify_only",
            serde_json::json!({
                "parents": id1,
                "operation": "EXPRESSION",
                "expression": "a * 2",
            }),
        );
        assert_eq!(
            no_res_expr["result"]["isError"],
            serde_json::Value::Bool(true)
        );
        assert!(
            client
                .text_content(&no_res_expr)
                .contains("result is required"),
            "expected 'result is required' in: {no_res_expr}"
        );

        // 5. verify_only with null or boolean or object result fails as invalid input
        for invalid_res in [
            serde_json::Value::Null,
            serde_json::json!(false),
            serde_json::json!({"unexpected": "object"}),
        ] {
            let bad_type = client.call_tool(
                8,
                "verify_only",
                serde_json::json!({
                    "parents": format!("{id1},{id2}"),
                    "operation": "DATE_DIFF",
                    "result": invalid_res,
                }),
            );
            assert_eq!(bad_type["result"]["isError"], serde_json::Value::Bool(true));
            assert!(
                client.text_content(&bad_type).contains("invalid result"),
                "expected 'invalid result' in: {bad_type}"
            );
        }

        // Verify that verify_only never writes to the graph
        let nodes_after = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(
            nodes_after.len(),
            2,
            "verify_only must never record derived nodes"
        );
    }
}
