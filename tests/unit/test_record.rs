#[cfg(test)]
mod tests {
    use rgt::hooks::parser::{extract_values_from_content, NumberFormat};
    use rgt::types::{TrackedNode, ValueKind};
    use std::sync::Mutex;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    /// Extracts values under the default `auto` format for existing assertions.
    fn extract(content: &str) -> Vec<rgt::hooks::parser::ExtractedValue> {
        extract_values_from_content(content, NumberFormat::Auto).values
    }

    #[test]
    fn test_extract_numbers_from_content() {
        let content = "category,amount\nrevenue,100000\ncosts,60000\nprofit,40000\n";
        let extracted = extract(content);
        assert_eq!(extracted.len(), 3); // 100000, 60000, 40000
    }

    #[test]
    fn test_extract_dates_from_content() {
        let content = "start: 2026-01-15\nreview: 2026-03-01\nlaunch: 2026-06-30\n";
        let extracted = extract(content);
        // FR-005: exact total count — previously only date_count was asserted,
        // masking the 9 spurious number nodes from the dates' digits.
        assert_eq!(extracted.len(), 3);
        for v in &extracted {
            assert_eq!(v.value.kind(), ValueKind::Date);
        }
    }

    #[test]
    fn test_single_date_emits_one_node_no_spurious_numbers() {
        let extracted = extract("start: 2026-01-15\n");
        assert_eq!(
            extracted.len(),
            1,
            "a date must not leak its digits as numbers"
        );
        assert_eq!(extracted[0].value.kind(), ValueKind::Date);
        assert_eq!(extracted[0].occurrence, 0);
    }

    #[test]
    fn test_iso_datetime_emits_one_node() {
        let extracted = extract("stamp: 2026-01-15T12:30:00Z\n");
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0].value.kind(), ValueKind::Date);
    }

    #[test]
    fn test_same_line_identical_values_have_distinct_occurrences_and_ids() {
        let extracted = extract("revenue,120000,discount,120000\n");
        assert_eq!(extracted.len(), 2);
        for v in &extracted {
            assert_eq!(v.value.kind(), ValueKind::Number);
        }
        assert_eq!(extracted[0].occurrence, 0);
        assert_eq!(extracted[1].occurrence, 1);
        // SC-003: distinct occurrences must yield distinct root node IDs.
        let id0 = TrackedNode::generate_root_id(
            "f.csv",
            extracted[0].line_number,
            extracted[0].occurrence,
            &extracted[0].value,
        );
        let id1 = TrackedNode::generate_root_id(
            "f.csv",
            extracted[1].line_number,
            extracted[1].occurrence,
            &extracted[1].value,
        );
        assert_ne!(
            id0, id1,
            "same-line identical values must get distinct node IDs"
        );
    }

    #[test]
    fn test_control_different_values_same_line() {
        let extracted = extract("a,100,b,200\n");
        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted[0].occurrence, 0);
        assert_eq!(extracted[1].occurrence, 0);
    }

    #[test]
    fn test_extraction_is_deterministic() {
        let content = "start: 2026-01-15\nrevenue,120000,discount,120000\nqty,42\n";
        let a = extract(content);
        let b = extract(content);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.line_number, y.line_number);
            assert_eq!(x.occurrence, y.occurrence);
            assert_eq!(x.value.to_string_repr(), y.value.to_string_repr());
        }
    }

    #[test]
    fn test_empty_content() {
        let extracted = extract("");
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_extract_no_numbers_or_dates() {
        let content = "hello world\njust text\nnothing here\n";
        let extracted = extract(content);
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_value_limit_10k() {
        let mut content = String::new();
        for i in 0..11000 {
            content.push_str(&format!("{}\n", i));
        }
        let extracted = extract(&content);

        let num_count = extracted
            .iter()
            .filter(|v| v.value.kind() == ValueKind::Number)
            .count();
        assert!(
            num_count > 10000,
            "Should extract all numbers; limit enforcement is in record CLI, not parser"
        );
    }

    #[test]
    fn test_value_limit_functionality() {
        let numbers: Vec<f64> = (0..11000).map(|i| i as f64).collect();
        let limited: Vec<f64> = numbers.iter().take(10000).copied().collect();
        assert_eq!(limited.len(), 10000);
        assert_eq!(numbers.len(), 11000);
        assert_eq!(limited.last(), Some(&9999.0));
    }

    #[test]
    fn test_record_10k_performance() {
        use rgt::cli::{execute_init, execute_record};
        use std::fs;
        use std::io::Write;
        use std::time::Instant;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let _guard = {
            let guard = CWD_MUTEX.lock().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            guard
        };

        execute_init(false, true, Some("codex"), None).unwrap();

        let file_path = dir.path().join("perf.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        for i in 0..10000 {
            writeln!(f, "{}", i).unwrap();
        }

        let path_str = file_path.to_string_lossy().to_string();
        let start = Instant::now();
        execute_record(&path_str, false, None).unwrap();
        let elapsed = start.elapsed();

        // Threshold rationale: guards against reintroducing per-node auto-commit inserts
        // (measured ~16s on CI before the transaction fix). The transaction fix measures
        // ~0.25s locally and ~2.8s on Windows CI. 8s catches the regression with ample
        // headroom for shared-runner variance.
        assert!(
            elapsed.as_secs_f64() < 8.0,
            "SC-001: Record 10k values took {:.3}s, expected <8s (per-node inserts take ~16s; CI runners are slower than local)",
            elapsed.as_secs_f64()
        );
    }

    #[test]
    fn test_record_reconciles_pure_line_shifts_without_staleness_or_id_churn() {
        use rgt::cli::{execute_derive_core, execute_init, execute_record};
        use rgt::store::DbStore;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let _guard = {
            let guard = CWD_MUTEX.lock().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            guard
        };

        execute_init(false, true, Some("codex"), None).unwrap();

        let file_path = dir.path().join("data.txt");
        fs::write(&file_path, "100.0\n200.0\n").unwrap();
        execute_record("data.txt", false, None).unwrap();

        let db = DbStore::open_in_project(".").unwrap();
        let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        let node_100 = nodes
            .iter()
            .find(|n| n.value == rgt::types::ValueData::Number(100.0))
            .unwrap();
        let id_100 = node_100.id.clone();
        assert_eq!(node_100.line_number, Some(1));
        assert!(!node_100.is_stale);

        let node_200 = nodes
            .iter()
            .find(|n| n.value == rgt::types::ValueData::Number(200.0))
            .unwrap();
        let id_200 = node_200.id.clone();
        assert_eq!(node_200.line_number, Some(2));
        assert!(!node_200.is_stale);

        // Derive child
        let parents_arg = format!("{},{}", id_100, id_200);
        execute_derive_core(
            &parents_arg,
            "EXPRESSION",
            Some("a + b"),
            Some("300.0"),
            None,
            None,
        )
        .unwrap();

        // Duplicate parent ID in EXPRESSION must be rejected
        let dup_parents = format!("{},{}", id_100, id_100);
        let dup_err = execute_derive_core(
            &dup_parents,
            "EXPRESSION",
            Some("a + b"),
            Some("200.0"),
            None,
            None,
        )
        .unwrap_err();
        assert!(
            dup_err.to_string().contains("duplicate parent ID"),
            "unexpected err: {dup_err}"
        );

        // Now shift lines by inserting header lines
        fs::write(
            &file_path,
            "# Header comment\n# Another line\n100.0\n200.0\n",
        )
        .unwrap();
        execute_record("data.txt", false, None).unwrap();

        // Re-check nodes
        let nodes_after = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        let updated_100 = nodes_after
            .iter()
            .find(|n| n.id == id_100)
            .expect("root 100 id must be preserved");
        assert_eq!(
            updated_100.line_number,
            Some(3),
            "line number should update to 3"
        );
        assert!(
            !updated_100.is_stale,
            "root 100 must NOT be marked stale on line shift"
        );

        let updated_200 = nodes_after
            .iter()
            .find(|n| n.id == id_200)
            .expect("root 200 id must be preserved");
        assert_eq!(
            updated_200.line_number,
            Some(4),
            "line number should update to 4"
        );
        assert!(
            !updated_200.is_stale,
            "root 200 must NOT be marked stale on line shift"
        );

        // Derived child must remain fresh
        let derived_nodes: Vec<_> = nodes_after
            .iter()
            .filter(|n| n.node_type == rgt::types::NodeType::Derived)
            .collect();
        assert_eq!(derived_nodes.len(), 1);
        assert!(
            !derived_nodes[0].is_stale,
            "derived child must NOT be marked stale"
        );
    }

    #[test]
    fn test_extract_european_decimal_in_prose() {
        let content = "Price: 12,34 EUR";
        let auto_res = extract_values_from_content(content, NumberFormat::Auto);
        assert_eq!(auto_res.values.len(), 1);
        assert_eq!(
            auto_res.values[0].value,
            rgt::types::ValueData::Number(12.34)
        );

        let us_res = extract_values_from_content(content, NumberFormat::Us);
        assert_eq!(
            us_res.values.len(),
            0,
            "Us mode must reject 12,34 in prose rather than splitting it"
        );
        assert_eq!(us_res.skipped_malformed, 1);
    }

    #[test]
    fn test_extract_rejects_ipv4_and_versions() {
        let content = "192.168.100.100\nv1.100.200";
        let eu_res = extract_values_from_content(content, NumberFormat::Eu);
        assert_eq!(
            eu_res.values.len(),
            0,
            "IPv4 and versions must not be extracted as numbers in Eu mode"
        );

        let us_res = extract_values_from_content(content, NumberFormat::Us);
        assert_eq!(
            us_res.values.len(),
            0,
            "IPv4 and versions must not be extracted as numbers in Us mode"
        );
    }

    #[test]
    fn test_extract_masks_urls_times_uuids_and_compact_iso_offset() {
        let time_res = extract_values_from_content("Current time: 14:30:00", NumberFormat::Auto);
        assert_eq!(
            time_res.values.len(),
            0,
            "Standalone time must not shatter into numbers"
        );

        let iso_res = extract_values_from_content("2026-01-15T14:30:00+0200", NumberFormat::Auto);
        assert_eq!(iso_res.values.len(), 1);
        assert_eq!(iso_res.values[0].value.kind(), ValueKind::Date);
        if let rgt::types::ValueData::Date(dt) = iso_res.values[0].value {
            assert_eq!(dt.to_rfc3339(), "2026-01-15T12:30:00+00:00");
        } else {
            panic!("expected Date");
        }

        let url_res =
            extract_values_from_content("https://example.com:8080/v2/item_1", NumberFormat::Auto);
        assert_eq!(url_res.values.len(), 0, "URL must not shatter into numbers");

        let uuid_res = extract_values_from_content("81e6-4faf-1234-5678b", NumberFormat::Auto);
        assert_eq!(
            uuid_res.values.len(),
            0,
            "UUID must not shatter into numbers"
        );

        let sub_res = extract_values_from_content("(a)-200\n[x]-200", NumberFormat::Auto);
        assert_eq!(sub_res.values.len(), 2);
        assert_eq!(
            sub_res.values[0].value,
            rgt::types::ValueData::Number(200.0)
        );
        assert_eq!(
            sub_res.values[1].value,
            rgt::types::ValueData::Number(200.0)
        );
    }
}
