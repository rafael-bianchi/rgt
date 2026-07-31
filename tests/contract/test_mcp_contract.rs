#[cfg(test)]
mod tests {
    use rgt::mcp::handlers::{handle_record_derivation, handle_record_value};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_mcp_record_value_and_derivation() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("spec.md");
        std::fs::write(&file_path, "Target: 100").unwrap();

        let val_params = json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 100.0
        });

        let res_val = handle_record_value(&val_params).unwrap();
        let node_id = res_val.get("node_id").unwrap().as_str().unwrap();

        let drv_params = json!({
            "parent_node_ids": [node_id],
            "value_kind": "NUMBER",
            "number_value": 200.0,
            "operation_type": "MULTIPLY"
        });

        let res_drv = handle_record_derivation(&drv_params).unwrap();
        let drv_id = res_drv.get("node_id").unwrap().as_str().unwrap();

        assert!(drv_id.starts_with("node_drv_"));
    }
}
