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
