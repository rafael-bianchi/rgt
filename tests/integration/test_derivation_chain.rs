#[cfg(test)]
mod tests {
    use rgt::mcp::handlers::{handle_query_provenance, handle_record_derivation, handle_record_value};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_multi_level_derivation_chain_query() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("source.txt");
        std::fs::write(&file_path, "50").unwrap();

        let n1 = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 50.0
        })).unwrap();
        let id1 = n1.get("node_id").unwrap().as_str().unwrap();

        let n2 = handle_record_derivation(&json!({
            "parent_node_ids": [id1],
            "value_kind": "NUMBER",
            "number_value": 100.0,
            "operation_type": "MULTIPLY"
        })).unwrap();
        let id2 = n2.get("node_id").unwrap().as_str().unwrap();

        let n3 = handle_record_derivation(&json!({
            "parent_node_ids": [id2],
            "value_kind": "NUMBER",
            "number_value": 105.0,
            "operation_type": "ADD"
        })).unwrap();
        let id3 = n3.get("node_id").unwrap().as_str().unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id3 })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert!(steps.len() >= 2);
    }
}
