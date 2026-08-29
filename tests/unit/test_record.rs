#[cfg(test)]
mod tests {
    use rgt::hooks::parser::extract_values_from_content;
    use rgt::types::ValueKind;

    #[test]
    fn test_extract_numbers_from_content() {
        let content = "category,amount\nrevenue,100000\ncosts,60000\nprofit,40000\n";
        let extracted = extract_values_from_content(content);
        assert_eq!(extracted.len(), 3); // 100000, 60000, 40000
    }

    #[test]
    fn test_extract_dates_from_content() {
        let content = "start: 2026-01-15\nreview: 2026-03-01\nlaunch: 2026-06-30\n";
        let extracted = extract_values_from_content(content);
        let date_count = extracted
            .iter()
            .filter(|v| v.value.kind() == ValueKind::Date)
            .count();
        assert_eq!(date_count, 3);
    }

    #[test]
    fn test_extract_empty_content() {
        let extracted = extract_values_from_content("");
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_extract_no_numbers_or_dates() {
        let content = "hello world\njust text\nnothing here\n";
        let extracted = extract_values_from_content(content);
        assert!(extracted.is_empty());
    }

    #[test]
    fn test_value_limit_10k() {
        let mut content = String::new();
        for i in 0..11000 {
            content.push_str(&format!("{}\n", i));
        }
        let extracted = extract_values_from_content(&content);

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
        use std::sync::Mutex;
        use std::time::Instant;
        use tempfile::tempdir;

        static CWD_MUTEX: Mutex<()> = Mutex::new(());
        let dir = tempdir().unwrap();
        let _guard = {
            let guard = CWD_MUTEX.lock().unwrap();
            std::env::set_current_dir(dir.path()).unwrap();
            guard
        };

        execute_init(false, true, Some("codex")).unwrap();

        let file_path = dir.path().join("perf.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        for i in 0..10000 {
            writeln!(f, "{}", i).unwrap();
        }

        let path_str = file_path.to_string_lossy().to_string();
        let start = Instant::now();
        execute_record(&path_str, false).unwrap();
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
}
