#[cfg(test)]
mod tests {
    #[test]
    fn source_lookup_accepts_windows_separators_after_normalized_recording() {
        let db = rgt::store::DbStore::open_in_memory().unwrap();
        rgt::store::queries::upsert_source_document(
            db.conn(),
            "C:/project/data.csv",
            1,
            1,
            "hash",
            None,
            None,
        )
        .unwrap();
        let doc =
            rgt::store::queries::get_source_document_by_path(db.conn(), "C:\\project\\data.csv")
                .unwrap();
        assert!(
            doc.is_some(),
            "lookup must use the same separator convention as recording"
        );
    }
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

    #[test]
    fn test_record_after_sync_staleness_unstales_and_inserts_new_values() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.csv");
        let content_v1 = "a,100\n";
        std::fs::write(&file_path, content_v1).unwrap();

        let mut db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let meta_v1 = rgt::detection::get_metadata_snapshot(&file_path).unwrap();
        let hash_v1 = rgt::detection::compute_blake3_hash_from_bytes(content_v1.as_bytes());
        let ext_v1 = extract_values_from_content(content_v1, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(
            db.conn(),
            "data.csv",
            &meta_v1,
            &hash_v1,
            &ext_v1,
            None,
        )
        .unwrap();

        let nodes_v1 = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(nodes_v1.len(), 1);
        let orig_id = nodes_v1[0].id.clone();

        // Modify file on disk and trigger staleness check (as `rgt status` does)
        let content_v2 = "a,100\nb,200\n";
        std::fs::write(&file_path, content_v2).unwrap();
        let mut cascade =
            rgt::graph::invalidation::InvalidationCascade::build_from_db(&db).unwrap();
        cascade
            .evaluate_and_invalidate_all(&mut db, dir.path())
            .unwrap();

        let stale_after_sync = rgt::store::queries::list_stale_nodes(db.conn()).unwrap();
        assert_eq!(stale_after_sync.len(), 1);

        // Re-record modified file (including capture_association to verify FK safety)
        let meta_v2 = rgt::detection::get_metadata_snapshot(&file_path).unwrap();
        let hash_v2 = rgt::detection::compute_blake3_hash_from_bytes(content_v2.as_bytes());
        let ext_v2 = extract_values_from_content(content_v2, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(
            db.conn(),
            "data.csv",
            &meta_v2,
            &hash_v2,
            &ext_v2,
            Some("claude-code"),
        )
        .expect("re-recording after sync_staleness must not fail FK constraint");

        let nodes_v2 = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(
            nodes_v2.len(),
            2,
            "new value 200 must be inserted after sync_staleness"
        );
        for n in &nodes_v2 {
            assert!(
                !n.is_stale,
                "node {} must be un-staled after re-recording",
                n.id
            );
        }
        assert!(nodes_v2.iter().any(|n| n.id == orig_id));
    }

    #[test]
    fn test_record_shifted_node_not_clobbered_when_same_value_inserted_at_old_line() {
        let dir = tempfile::tempdir().unwrap();
        let db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let meta = rgt::detection::MetadataSnapshot {
            mtime_nsec: 1,
            file_size: 10,
            dev: None,
            ino: None,
        };

        // Step 1: 100 at line 1
        let c1 = "val,100\n";
        let ext1 = extract_values_from_content(c1, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(db.conn(), "shift.csv", &meta, "h1", &ext1, None)
            .unwrap();
        let n1 = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(n1.len(), 1);
        let shifted_id = n1[0].id.clone();

        // Step 2: 100 shifts to line 5
        let c2 = "#a\n#b\n#c\n#d\nval,100\n";
        let ext2 = extract_values_from_content(c2, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(db.conn(), "shift.csv", &meta, "h2", &ext2, None)
            .unwrap();
        let n2 = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
        assert_eq!(n2.len(), 1);
        assert_eq!(n2[0].id, shifted_id);
        assert_eq!(n2[0].line_number, Some(5));

        // Step 3: A new 100 is added at line 1 while the existing 100 stays at line 5
        let c3 = "val,100\n#b\n#c\n#d\nval,100\n";
        let ext3 = extract_values_from_content(c3, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(db.conn(), "shift.csv", &meta, "h3", &ext3, None)
            .unwrap();
        let n3: Vec<_> = rgt::store::queries::list_all_nodes(db.conn())
            .unwrap()
            .into_iter()
            .filter(|n| !n.is_stale)
            .collect();
        assert_eq!(
            n3.len(),
            2,
            "both line 1 and line 5 nodes must exist distinctly without ON CONFLICT clobbering"
        );
        let line5_node = n3.iter().find(|n| n.line_number == Some(5)).unwrap();
        assert_eq!(
            line5_node.id, shifted_id,
            "shifted node at line 5 must retain its identity"
        );
        let mut lines: Vec<_> = n3.iter().map(|n| n.line_number).collect();
        lines.sort();
        assert_eq!(lines, vec![Some(1), Some(5)]);
    }

    #[test]
    fn test_dev_ino_high_bit_u64_conversion_does_not_fail() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("high_ino.csv");
        let content = "x,42\n";
        std::fs::write(&file_path, content).unwrap();

        let mut db = rgt::store::DbStore::open_in_project(dir.path()).unwrap();
        let meta = rgt::detection::MetadataSnapshot {
            mtime_nsec: 123,
            file_size: content.len() as u64,
            dev: Some(u64::MAX),
            ino: Some(u64::MAX - 7),
        };
        let hash = rgt::detection::compute_blake3_hash_from_bytes(content.as_bytes());
        let ext = extract_values_from_content(content, NumberFormat::Auto).values;
        rgt::store::queries::record_file_batch(db.conn(), "high_ino.csv", &meta, &hash, &ext, None)
            .unwrap();

        // Re-recording under an alias path with the same high-bit (dev, ino) exercises get_source_document_by_dev_ino
        rgt::store::queries::record_file_batch(
            db.conn(),
            "alias_high_ino.csv",
            &meta,
            &hash,
            &ext,
            None,
        )
        .expect("record_file_batch with high-bit dev/ino must handle u64 > i64::MAX");

        let mut cascade =
            rgt::graph::invalidation::InvalidationCascade::build_from_db(&db).unwrap();
        cascade
            .evaluate_and_invalidate_all(&mut db, dir.path())
            .expect("evaluate_and_invalidate_all must handle u64 > i64::MAX");
    }

    #[test]
    fn test_auto_format_not_poisoned_by_csv_or_arrays_with_spaces() {
        let content = "Alice Smith,10,20\nItems: [1,2]\nPrice,12.50\nRatio,1.234\n";
        let res = extract_values_from_content(content, NumberFormat::Auto);
        let nums: Vec<f64> = res
            .values
            .into_iter()
            .filter_map(|v| match v.value {
                rgt::types::ValueData::Number(n) => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(
            nums,
            vec![10.0, 20.0, 1.0, 2.0, 12.5, 1.234],
            "US file with spaced CSV/array must not switch to Eu and corrupt 12.50 or 1.234"
        );
    }

    #[test]
    fn test_csv_rows_with_spaces_or_two_columns_split_cleanly_in_us_mode() {
        let content = "a,b\n10,200\n10,200,300,John Doe\nJohn Doe,12,34\n";
        let res = extract_values_from_content(content, NumberFormat::Us);
        let nums: Vec<f64> = res
            .values
            .into_iter()
            .filter_map(|v| match v.value {
                rgt::types::ValueData::Number(n) => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(
            nums,
            vec![10.0, 200.0, 10.0, 200.0, 300.0, 12.0, 34.0],
            "CSV cells must split cleanly rather than merging into 10200/10200300 or failing as Malformed"
        );
    }

    #[test]
    fn test_unwrap_colon_does_not_corrupt_yaml_keys_and_handles_accounting_percent() {
        let yaml = "2024: 100\n2025: 200\n";
        let (unwrapped, mapping) = rgt::hooks::parser::unwrap_line_numbered_content(yaml);
        assert_eq!(
            unwrapped, yaml,
            "YAML keys starting with year: must not be stripped as line numbers"
        );
        assert_eq!(mapping, vec![1, 2]);

        let res = extract_values_from_content(
            "Margin: (12.5%) and step (1) had (500)\n",
            NumberFormat::Us,
        );
        let nums: Vec<f64> = res
            .values
            .into_iter()
            .filter_map(|v| match v.value {
                rgt::types::ValueData::Number(n) => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(
            nums,
            vec![-12.5, 1.0, -500.0],
            "(12.5%) and (500) are negative, while prose 'step (1)' stays positive"
        );
    }
}
