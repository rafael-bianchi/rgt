#[cfg(test)]
mod tests {
    use rgt::graph::InvalidationCascade;
    use rgt::mcp::handlers::{handle_record_derivation, handle_record_value};
    use rgt::store::queries::{get_tracked_node, list_stale_nodes};
    use rgt::store::DbStore;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_file_edit_cascades_staleness_to_derived_nodes() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let db = DbStore::open_in_project(dir.path()).unwrap();

        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "150").unwrap();

        let root_res = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 150.0
        }))
        .unwrap();
        let root_id = root_res.get("node_id").unwrap().as_str().unwrap();

        let drv_res = handle_record_derivation(&json!({
            "parent_node_ids": [root_id],
            "value_kind": "NUMBER",
            "number_value": 300.0,
            "operation_type": "MULTIPLY"
        }))
        .unwrap();
        let drv_id = drv_res.get("node_id").unwrap().as_str().unwrap();

        // Modify file
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&file_path, "200").unwrap();

        let mut cascade = InvalidationCascade::build_from_db(&db).unwrap();
        let count = cascade.evaluate_and_invalidate_all(&db, dir.path()).unwrap();
        assert!(count >= 2);

        let stale_nodes = list_stale_nodes(db.conn()).unwrap();
        assert_eq!(stale_nodes.len(), 2);

        let drv_node = get_tracked_node(db.conn(), drv_id).unwrap().unwrap();
        assert!(drv_node.is_stale);
    }
}
