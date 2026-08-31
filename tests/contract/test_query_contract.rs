#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    fn rgt() -> &'static str {
        env!("CARGO_BIN_EXE_rgt")
    }

    /// Seeds a small provenance graph: a root value recorded from a file and a
    /// derived child. Returns the derived node's id.
    fn seed_lineage(dir: &std::path::Path) -> String {
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir)
            .output()
            .expect("spawn rgt init")
            .status
            .success());

        let file_path = dir.join("data.csv");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "revenue,120000\ncosts,60000\n").unwrap();
        assert!(std::process::Command::new(rgt())
            .arg("record")
            .arg(&file_path)
            .current_dir(dir)
            .output()
            .expect("spawn rgt record")
            .status
            .success());

        // Resolve the two recorded root nodes directly from the store.
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let all = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        let roots: Vec<String> = all
            .iter()
            .filter(|n| n.node_type == rgt::types::NodeType::Root)
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(roots.len(), 2, "expected 2 root nodes: {all:?}");

        let a = &roots[0];
        let b = &roots[1];
        let derive_out = std::process::Command::new(rgt())
            .args([
                "derive",
                "--parents",
                &format!("{a},{b}"),
                "--operation",
                "EXPRESSION",
                "--expression",
                "a + b",
                "--result",
                "180000",
            ])
            .current_dir(dir)
            .output()
            .expect("spawn rgt derive");
        assert!(
            derive_out.status.success(),
            "derive failed: {}",
            String::from_utf8_lossy(&derive_out.stderr)
        );

        // The derived node is the one with parents — ask for it via the store.
        let db = rgt::store::DbStore::open_in_project(".").expect("open store");
        let conn = db.conn();
        let all = rgt::store::queries::list_all_nodes(conn).expect("list nodes");
        let derived = all
            .iter()
            .find(|n| n.node_type == rgt::types::NodeType::Derived)
            .expect("derived node must exist");
        derived.id.clone()
    }

    #[test]
    fn default_output_is_plain_text_not_json() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let id = seed_lineage(dir.path());

        let out = std::process::Command::new(rgt())
            .args(["query", &id])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt query");

        assert!(
            out.status.success(),
            "query failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.trim_start().starts_with('{'),
            "default output must not be JSON: {text}"
        );
        assert!(
            !text.contains("\"lineage\""),
            "default output must not contain the JSON lineage wrapper: {text}"
        );
        assert!(
            text.contains(&id) && text.contains("value") && text.contains("kind"),
            "default output must be a readable lineage trace: {text}"
        );
        assert!(
            text.contains("total ancestors"),
            "must show ancestor count: {text}"
        );
    }

    #[test]
    fn json_flag_output_parses_and_has_no_wrapper() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let id = seed_lineage(dir.path());

        let out = std::process::Command::new(rgt())
            .args(["query", &id, "--json"])
            .current_dir(dir.path())
            .output()
            .expect("spawn rgt query --json");

        assert!(
            out.status.success(),
            "query --json failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let value: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("--json output must parse as JSON");
        assert_eq!(value["node_id"].as_str(), Some(id.as_str()));
        assert!(
            value.get("lineage").is_none(),
            "--json output must not wrap in a lineage key: {value}"
        );
        assert!(value["lineage_steps"].is_array());
        assert_eq!(
            value["lineage_steps"].as_array().unwrap().len(),
            3,
            "root, root, derived"
        );
        assert_eq!(value["total_ancestors"].as_u64(), Some(2));
    }

    #[test]
    fn both_modes_carry_same_lineage_steps() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        let id = seed_lineage(dir.path());

        let text = String::from_utf8_lossy(
            &std::process::Command::new(rgt())
                .args(["query", &id])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .stdout,
        )
        .to_string();

        let json: serde_json::Value = serde_json::from_slice(
            &std::process::Command::new(rgt())
                .args(["query", &id, "--json"])
                .current_dir(dir.path())
                .output()
                .unwrap()
                .stdout,
        )
        .expect("json must parse");

        for step in json["lineage_steps"].as_array().unwrap() {
            let nid = step["node_id"].as_str().unwrap();
            assert!(text.contains(nid), "text output must mention node {nid}");
        }
    }

    #[test]
    fn unknown_node_errors_in_both_modes() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());
        assert!(std::process::Command::new(rgt())
            .args(["init", "--agent", "codex"])
            .current_dir(dir.path())
            .output()
            .unwrap()
            .status
            .success());

        for extra in [&[][..], &["--json"][..]] {
            let mut args = vec!["query", "nonexistent_node"];
            args.extend_from_slice(extra);
            let out = std::process::Command::new(rgt())
                .args(&args)
                .current_dir(dir.path())
                .output()
                .expect("spawn rgt query");
            assert!(!out.status.success());
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(stderr.contains("Node not found"), "{stderr}");
        }
    }
}
