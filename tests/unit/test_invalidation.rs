#[cfg(test)]
mod tests {
    use rgt::graph::engine::GraphEngine;

    #[test]
    fn test_reverse_edge_downstream_dependents() {
        let mut engine = GraphEngine::new();
        engine.get_or_add_node("root1");
        engine.get_or_add_node("derived1");
        engine.get_or_add_node("derived2");

        let _ = engine.add_derivation_edge("root1", "derived1", "ADD");
        let _ = engine.add_derivation_edge("derived1", "derived2", "MULTIPLY");

        let dependents = engine.get_downstream_dependents("root1");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"derived1".to_string()));
        assert!(dependents.contains(&"derived2".to_string()));
    }

    #[test]
    fn test_cycle_prevention() {
        let mut engine = GraphEngine::new();
        engine.get_or_add_node("nodeA");
        engine.get_or_add_node("nodeB");

        assert!(engine.add_derivation_edge("nodeA", "nodeB", "OP1").is_ok());
        assert!(engine.add_derivation_edge("nodeB", "nodeA", "OP2").is_err());
    }
}

// ---- 029 / US3 (T011): shared downstream node counted once ----

#[test]
fn shared_downstream_node_is_counted_once() {
    use chrono::Utc;
    use rgt::graph::invalidation::InvalidationCascade;
    use rgt::store::queries::{
        insert_derivation_edge, insert_tracked_node, upsert_source_document,
    };
    use rgt::store::DbStore;
    use rgt::types::{NodeType, TrackedNode, ValueData};

    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    let doc = upsert_source_document(conn, "f.csv", 1, 1, "h", None, None).unwrap();

    let now = Utc::now();
    let mk = |id: &str, doc_id: i64| TrackedNode {
        id: id.into(),
        node_type: NodeType::Root,
        value_kind: ValueData::Number(1.0).kind(),
        value: ValueData::Number(1.0),
        source_doc_id: Some(doc_id),
        line_number: Some(1),
        is_stale: false,
        stale_reason: None,
        created_at: now,
        updated_at: now,
    };
    insert_tracked_node(conn, &mk("r1", doc.id)).unwrap();
    insert_tracked_node(conn, &mk("r2", doc.id)).unwrap();
    insert_tracked_node(conn, &mk("shared", doc.id)).unwrap();
    insert_derivation_edge(conn, "r1", "shared", "EXPRESSION", None).unwrap();
    insert_derivation_edge(conn, "r2", "shared", "EXPRESSION", None).unwrap();

    let mut cascade = InvalidationCascade::build_from_db(&db).unwrap();
    let count = cascade
        .invalidate_file(&db, "f.csv", "SOURCE_FILE_MODIFIED")
        .unwrap();
    // r1, r2, shared = 3 distinct nodes, even though `shared` is reachable
    // from both roots (would have been counted twice without dedup).
    assert_eq!(count, 3);
}

#[test]
fn rerecord_invalidates_modified_root_nodes_and_their_derivations() {
    use rgt::store::DbStore;
    use rgt::types::{NodeType, ValueData};

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("data.csv");
    std::fs::write(&file_path, "a,100\nb,500\n").unwrap();

    // Initialize repo so DbStore::open_in_project works in dir
    let db = DbStore::open_in_project(dir.path()).unwrap();

    // We need current directory to be dir.path() for record and derive to find the DB
    let _guard = {
        struct CwdGuard(std::path::PathBuf);
        impl Drop for CwdGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.0);
            }
        }
        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        CwdGuard(old)
    };

    // Record the initial file
    let path_str = file_path.to_string_lossy().to_string();
    rgt::cli::record::execute_record(&path_str, false, None).unwrap();

    let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
    assert_eq!(nodes.len(), 2);
    let n100 = nodes
        .iter()
        .find(|n| n.value == ValueData::Number(100.0))
        .unwrap()
        .clone();
    let n500 = nodes
        .iter()
        .find(|n| n.value == ValueData::Number(500.0))
        .unwrap()
        .clone();

    let drv_id =
        rgt::cli::derive::execute_derive_mcp(&n100.id, "EXPRESSION", Some("a * 2"), 200.0).unwrap();

    // Edit the file: change 100 to 200, but leave 500 unchanged
    std::fs::write(&file_path, "a,200\nb,500\n").unwrap();

    // Re-record the file
    rgt::cli::record::execute_record(&path_str, false, None).unwrap();

    // Check node states
    let old_n100 = rgt::store::queries::get_tracked_node(db.conn(), &n100.id)
        .unwrap()
        .unwrap();
    assert!(
        old_n100.is_stale,
        "modified root node 100 must be marked stale"
    );
    assert_eq!(
        old_n100.stale_reason.as_deref(),
        Some("SOURCE_FILE_MODIFIED")
    );

    let drv_node = rgt::store::queries::get_tracked_node(db.conn(), &drv_id)
        .unwrap()
        .unwrap();
    assert!(
        drv_node.is_stale,
        "derived child of stale root must be marked stale"
    );
    assert_eq!(drv_node.stale_reason.as_deref(), Some("PARENT_NODE_STALE"));

    let still_500 = rgt::store::queries::get_tracked_node(db.conn(), &n500.id)
        .unwrap()
        .unwrap();
    assert!(
        !still_500.is_stale,
        "unchanged root node 500 must remain fresh (not stale)"
    );

    // The newly extracted 200 node must exist and be fresh
    let all_now = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
    let n200 = all_now
        .iter()
        .find(|n| n.value == ValueData::Number(200.0) && n.node_type == NodeType::Root)
        .unwrap();
    assert!(!n200.is_stale, "new 200 root node must be fresh");
}
