#[cfg(test)]
mod tests {
    use rgt::store::DbStore;
    use std::sync::Mutex;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    // ---- US1 (T006): opening a newer store fails with a clear error ----

    #[test]
    fn opening_a_newer_store_fails_with_version_error() {
        use rgt::store::schema::SCHEMA_VERSION;
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        // Initialize a store, then bump its version to simulate a future build.
        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        db.conn()
            .execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION + 1))
            .unwrap();
        drop(db);

        // A fresh connection must fail fast with the version message (not a SQL error).
        let Err(err) = DbStore::open_in_project(".") else {
            panic!("must fail");
        };
        let msg = format!("{}", err);
        assert!(msg.contains("newer than this build supports"), "{}", msg);
    }

    #[test]
    fn fresh_store_records_schema_version() {
        use rgt::store::schema::schema_version;
        use rgt::store::schema::SCHEMA_VERSION;
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        assert_eq!(schema_version(db.conn()).unwrap(), SCHEMA_VERSION);
    }

    // ---- US2 (T007): rgt gc removes only obsolete nodes ----

    fn insert_root(conn: &rusqlite::Connection, id: &str, stale: bool) {
        use chrono::Utc;
        use rgt::types::{NodeType, TrackedNode, ValueData};
        let now = Utc::now();
        let node = TrackedNode {
            id: id.to_string(),
            node_type: NodeType::Root,
            value_kind: ValueData::Number(1.0).kind(),
            value: ValueData::Number(1.0),
            source_doc_id: None,
            line_number: None,
            is_stale: stale,
            stale_reason: if stale {
                Some("FILE_DELETED".into())
            } else {
                None
            },
            created_at: now,
            updated_at: now,
        };
        rgt::store::queries::insert_tracked_node(conn, &node).unwrap();
    }

    #[test]
    fn gc_removes_only_stale_leaves_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        let conn = db.conn();

        insert_root(conn, "obsolete", true);
        insert_root(conn, "stale_with_dependent", true);
        insert_root(conn, "fresh", false);
        insert_root(conn, "child", false);
        rgt::store::queries::insert_derivation_edge(
            conn,
            "stale_with_dependent",
            "child",
            "EXPRESSION",
            None,
        )
        .unwrap();
        drop(db);

        rgt::cli::execute_gc(false).unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        let mut remaining: Vec<String> = rgt::store::queries::list_all_nodes(db.conn())
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            vec![
                "child".to_string(),
                "fresh".to_string(),
                "stale_with_dependent".to_string()
            ]
        );
    }

    #[test]
    fn gc_reports_zero_when_nothing_obsolete() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        insert_root(db.conn(), "fresh", false);
        drop(db);

        rgt::cli::execute_gc(false).unwrap();
        // Second run: nothing to collect.
        rgt::cli::execute_gc(true).unwrap(); // --vacuum path must also be harmless
    }

    #[test]
    fn gc_deletes_entire_stale_chain_in_single_invocation() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        let conn = db.conn();

        insert_root(conn, "root", true);
        insert_root(conn, "d1", true);
        insert_root(conn, "d2", true);
        insert_root(conn, "d3", true);
        insert_root(conn, "fresh_isolated", false);

        rgt::store::queries::insert_derivation_edge(conn, "root", "d1", "EXPRESSION", None)
            .unwrap();
        rgt::store::queries::insert_derivation_edge(conn, "d1", "d2", "EXPRESSION", None).unwrap();
        rgt::store::queries::insert_derivation_edge(conn, "d2", "d3", "EXPRESSION", None).unwrap();
        drop(db);

        rgt::cli::execute_gc(false).unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        let remaining: Vec<String> = rgt::store::queries::list_all_nodes(db.conn())
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(remaining, vec!["fresh_isolated".to_string()]);
    }

    #[test]
    fn derive_from_stale_parent_propagates_staleness() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        rgt::cli::execute_init(false, true, Some("codex"), None).unwrap();
        let db = DbStore::open_in_project(".").unwrap();
        insert_root(db.conn(), "stale_p", true);
        insert_root(db.conn(), "fresh_p", false);
        drop(db);

        let (child_id, _) = rgt::cli::execute_derive_core(
            "stale_p,fresh_p",
            "EXPRESSION",
            Some("a + b"),
            Some("2"),
            None,
            None,
        )
        .unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        let child = rgt::store::queries::get_tracked_node(db.conn(), &child_id)
            .unwrap()
            .expect("derived child");
        assert!(
            child.is_stale,
            "derived child of a stale parent must be stale"
        );
        assert_eq!(child.stale_reason.as_deref(), Some("PARENT_NODE_STALE"));
    }
}
