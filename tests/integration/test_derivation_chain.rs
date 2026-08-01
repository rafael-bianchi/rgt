#[cfg(test)]
mod tests {
    use rgt::mcp::handlers::{handle_query_provenance, handle_record_derivation, handle_record_value};
    use rgt::store::{DbStore, insert_derivation_edge};
    use serde_json::json;
    use std::time::Instant;
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
        assert!(steps.len() >= 3, "expected at least 3 lineage steps (n3, n2, n1/root), got {}: {:#?}", steps.len(), steps);
        let total_ancestors = res.get("total_ancestors").unwrap().as_u64().unwrap();
        assert_eq!(total_ancestors, 2, "expected 2 ancestors (n2 + n1), got {}", total_ancestors);
    }

    #[test]
    fn test_root_node_query() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("source.txt");
        std::fs::write(&file_path, "42").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 42.0
        })).unwrap();
        let id = n.get("node_id").unwrap().as_str().unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(res.get("total_ancestors").unwrap().as_u64().unwrap(), 0);
        let step = &steps[0];
        assert!(step.get("parent_ids").unwrap().as_array().unwrap().is_empty());
        assert_eq!(step.get("node_type").unwrap().as_str().unwrap(), "Root");
    }

    #[test]
    fn test_branching_chain_query() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let src_a = dir.path().join("a.txt");
        let src_b = dir.path().join("b.txt");
        std::fs::write(&src_a, "10").unwrap();
        std::fs::write(&src_b, "20").unwrap();

        let n_a = handle_record_value(&json!({
            "file_path": src_a.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 10.0
        })).unwrap();
        let id_a = n_a.get("node_id").unwrap().as_str().unwrap();

        let n_b = handle_record_value(&json!({
            "file_path": src_b.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 20.0
        })).unwrap();
        let id_b = n_b.get("node_id").unwrap().as_str().unwrap();

        let n_sum = handle_record_derivation(&json!({
            "parent_node_ids": [id_a, id_b],
            "value_kind": "NUMBER",
            "number_value": 30.0,
            "operation_type": "ADD"
        })).unwrap();
        let id_sum = n_sum.get("node_id").unwrap().as_str().unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id_sum })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 3, "expected 3 steps (sum + 2 roots), got {}: {:#?}", steps.len(), steps);
        let total_ancestors = res.get("total_ancestors").unwrap().as_u64().unwrap();
        assert_eq!(total_ancestors, 2);
    }

    #[test]
    fn test_determinism_repeatable_query() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("src.txt");
        std::fs::write(&file_path, "7").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 7.0
        })).unwrap();
        let id = n.get("node_id").unwrap().as_str().unwrap();

        let res1 = handle_query_provenance(&json!({ "node_id": id })).unwrap();
        let res2 = handle_query_provenance(&json!({ "node_id": id })).unwrap();

        assert_eq!(res1.get("total_ancestors"), res2.get("total_ancestors"));
        let steps1 = res1.get("lineage_steps").unwrap().as_array().unwrap();
        let steps2 = res2.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps1.len(), steps2.len());
        for i in 0..steps1.len() {
            assert_eq!(steps1[i].get("node_id"), steps2[i].get("node_id"));
            assert_eq!(steps1[i].get("node_type"), steps2[i].get("node_type"));
        }
    }

    #[test]
    fn test_self_loop_cycle() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("src.txt");
        std::fs::write(&file_path, "7").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 7.0
        })).unwrap();
        let id = n.get("node_id").unwrap().as_str().unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        insert_derivation_edge(db.conn(), id, id, "self_ref", None).unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 1, "self-loop should return only the node itself");
        assert_eq!(steps[0].get("node_id").unwrap().as_str().unwrap(), id);
    }

    #[test]
    fn test_cross_node_cycle() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let src_a = dir.path().join("a.txt");
        let src_b = dir.path().join("b.txt");
        std::fs::write(&src_a, "10").unwrap();
        std::fs::write(&src_b, "20").unwrap();

        let n_a = handle_record_value(&json!({
            "file_path": src_a.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 10.0
        })).unwrap();
        let id_a = n_a.get("node_id").unwrap().as_str().unwrap();

        let n_b = handle_record_value(&json!({
            "file_path": src_b.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 20.0
        })).unwrap();
        let id_b = n_b.get("node_id").unwrap().as_str().unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        insert_derivation_edge(db.conn(), id_a, id_b, "cycle_ab", None).unwrap();
        insert_derivation_edge(db.conn(), id_b, id_a, "cycle_ba", None).unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id_b })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 2, "cycle A→B→A should return both nodes without looping");
        let total_ancestors = res.get("total_ancestors").unwrap().as_u64().unwrap();
        assert_eq!(total_ancestors, 1);
    }

    #[test]
    fn test_diamond_dependency() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let src_a = dir.path().join("a.txt");
        std::fs::write(&src_a, "5").unwrap();

        let n_a = handle_record_value(&json!({
            "file_path": src_a.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 5.0
        })).unwrap();
        let id_a = n_a.get("node_id").unwrap().as_str().unwrap();

        let n_b = handle_record_derivation(&json!({
            "parent_node_ids": [id_a],
            "value_kind": "NUMBER",
            "number_value": 10.0,
            "operation_type": "MULTIPLY"
        })).unwrap();
        let id_b = n_b.get("node_id").unwrap().as_str().unwrap();

        let n_c = handle_record_derivation(&json!({
            "parent_node_ids": [id_a],
            "value_kind": "NUMBER",
            "number_value": 15.0,
            "operation_type": "MULTIPLY"
        })).unwrap();
        let id_c = n_c.get("node_id").unwrap().as_str().unwrap();

        let n_d = handle_record_derivation(&json!({
            "parent_node_ids": [id_b, id_c],
            "value_kind": "NUMBER",
            "number_value": 25.0,
            "operation_type": "ADD"
        })).unwrap();
        let id_d = n_d.get("node_id").unwrap().as_str().unwrap();

        let res = handle_query_provenance(&json!({ "node_id": id_d })).unwrap();
        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 4, "diamond D(B,C,A) should return 4 nodes, got {}: {:#?}", steps.len(), steps);
        let total_ancestors = res.get("total_ancestors").unwrap().as_u64().unwrap();
        assert_eq!(total_ancestors, 3);
        let mut node_ids: Vec<String> = steps.iter().map(|s| s.get("node_id").unwrap().as_str().unwrap().to_string()).collect();
        node_ids.sort();
        node_ids.dedup();
        assert_eq!(node_ids.len(), 4, "all 4 nodes should be unique");
    }

    #[test]
    fn test_deep_linear_chain_performance() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("deep.txt");
        std::fs::write(&file_path, "1").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 1.0
        })).unwrap();
        let mut prev_id = n.get("node_id").unwrap().as_str().unwrap().to_string();

        let chain_len = 100;
        for i in 0..chain_len {
            let d = handle_record_derivation(&json!({
                "parent_node_ids": [prev_id],
                "value_kind": "NUMBER",
                "number_value": (i + 2) as f64,
                "operation_type": "ADD"
            })).unwrap();
            prev_id = d.get("node_id").unwrap().as_str().unwrap().to_string();
        }

        let start = Instant::now();
        let res = handle_query_provenance(&json!({ "node_id": prev_id })).unwrap();
        let elapsed = start.elapsed();

        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), chain_len + 1, "{} nodes in chain, got {} steps", chain_len + 1, steps.len());
        let total_ancestors = res.get("total_ancestors").unwrap().as_u64().unwrap();
        assert_eq!(total_ancestors as usize, chain_len);

        let ms = elapsed.as_millis();
        assert!(ms < 200, "chain traversal took {}ms, expected <200ms", ms);
    }

    #[test]
    fn test_shallow_branching_performance() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let file_path = dir.path().join("shallow.txt");
        std::fs::write(&file_path, "1").unwrap();

        let n = handle_record_value(&json!({
            "file_path": file_path.to_str().unwrap(),
            "value_kind": "NUMBER",
            "number_value": 1.0
        })).unwrap();
        let root_id = n.get("node_id").unwrap().as_str().unwrap().to_string();

        // create 5 root nodes
        let mut root_ids = vec![root_id];
        for i in 0..4 {
            let fp = dir.path().join(format!("s{}.txt", i));
            std::fs::write(&fp, (i + 2).to_string()).unwrap();
            let r = handle_record_value(&json!({
                "file_path": fp.to_str().unwrap(),
                "value_kind": "NUMBER",
                "number_value": (i + 2) as f64
            })).unwrap();
            root_ids.push(r.get("node_id").unwrap().as_str().unwrap().to_string());
        }

        // create additional shallow nodes to build graph size (8,000 total)
        for _ in 0..8000 {
            let _ = handle_record_value(&json!({
                "file_path": file_path.to_str().unwrap(),
                "value_kind": "NUMBER",
                "number_value": 42.0
            })).unwrap();
        }

        // create a derived node with 5 root parents
        let d = handle_record_derivation(&json!({
            "parent_node_ids": root_ids,
            "value_kind": "NUMBER",
            "number_value": 99.0,
            "operation_type": "SUM"
        })).unwrap();
        let derived_id = d.get("node_id").unwrap().as_str().unwrap().to_string();

        let start = Instant::now();
        let res = handle_query_provenance(&json!({ "node_id": derived_id })).unwrap();
        let elapsed = start.elapsed();

        let steps = res.get("lineage_steps").unwrap().as_array().unwrap();
        assert_eq!(steps.len(), 6, "expected 6 steps (derived + 5 roots), got {}", steps.len());
        let ms = elapsed.as_millis();
        assert!(ms < 200, "shallow branching query took {}ms, expected <200ms", ms);
    }
}
