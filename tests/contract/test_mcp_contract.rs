#[cfg(test)]
mod tests {
    use rgt::mcp::handlers::{
        handle_query_provenance, handle_record_derivation, handle_record_value,
    };
    use serde_json::json;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
        let guard = CWD_MUTEX.lock().unwrap();
        std::env::set_current_dir(dir).unwrap();
        guard
    }

    #[test]
    fn test_mcp_record_value_and_derivation() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

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

    #[test]
    fn test_query_provenance_response_schema() {
        let dir = tempdir().unwrap();
        let _guard = set_cwd(dir.path());

        let file_path = dir.path().join("data.txt");
        std::fs::write(&file_path, "100").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 100.0,
            "line_number": 5
        }))
        .unwrap();
        let id = n.get("node_id").unwrap().as_str().unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id })).unwrap();

        assert!(res.get("node_id").is_some(), "missing node_id");
        assert!(res.get("is_stale").is_some(), "missing is_stale");
        assert!(
            res.get("total_ancestors").is_some(),
            "missing total_ancestors"
        );
        assert!(res.get("lineage_steps").is_some(), "missing lineage_steps");

        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert!(!steps.is_empty());
        let step = &steps[0];
        assert!(
            step.get("node_type").is_some(),
            "missing node_type in lineage step"
        );
        assert!(
            step.get("line_number").is_some(),
            "missing line_number in lineage step"
        );
        assert_eq!(step.get("node_type").unwrap().as_str().unwrap(), "Root");
    }
}
