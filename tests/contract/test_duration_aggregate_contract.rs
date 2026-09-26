use chrono::{Duration, TimeZone, Utc};
use rgt::store::queries::{insert_tracked_node, list_all_nodes};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let db = DbStore::open_in_project(directory.path()).unwrap();
    for (id, value) in [
        (
            "duration-days",
            ValueData::Duration(Duration::seconds(3_888_000)),
        ),
        (
            "duration-minutes",
            ValueData::Duration(Duration::seconds(2_700)),
        ),
        ("number", ValueData::Number(5.0)),
        (
            "date",
            ValueData::Date(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ),
    ] {
        let now = Utc::now();
        insert_tracked_node(
            db.conn(),
            &TrackedNode {
                id: id.into(),
                node_type: NodeType::Root,
                value_kind: value.kind(),
                value,
                source_doc_id: None,
                line_number: None,
                is_stale: false,
                stale_reason: None,
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
    }
    drop(db);
    directory
}

fn run(
    directory: &TempDir,
    operation: &str,
    parents: &str,
    flags: &[&str],
) -> std::process::Output {
    let mut args = vec!["derive", "--parents", parents, "--operation", operation];
    args.extend_from_slice(flags);
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(args)
        .current_dir(directory.path())
        .output()
        .unwrap()
}

fn node_count(directory: &TempDir) -> usize {
    list_all_nodes(DbStore::open_in_project(directory.path()).unwrap().conn())
        .unwrap()
        .len()
}

#[test]
fn typed_aggregates_compute_and_display_mixed_scale_values() {
    let directory = fixture();
    let sum = run(
        &directory,
        "DURATION_SUM",
        "duration-days,duration-minutes",
        &["--unit", "minutes"],
    );
    assert!(
        sum.status.success(),
        "{}",
        String::from_utf8_lossy(&sum.stderr)
    );
    let sum_stdout = String::from_utf8(sum.stdout).unwrap();
    assert!(sum_stdout.contains("64845 minutes"), "{sum_stdout}");

    let average = run(
        &directory,
        "DURATION_AVG",
        "duration-days,duration-minutes",
        &["--unit", "days"],
    );
    assert!(
        average.status.success(),
        "{}",
        String::from_utf8_lossy(&average.stderr)
    );
    let average_stdout = String::from_utf8(average.stdout).unwrap();
    assert!(
        average_stdout.contains("22.515625 days"),
        "{average_stdout}"
    );
}

#[test]
fn wrong_parent_types_and_duplicate_ids_fail_without_writes() {
    let directory = fixture();
    let before = node_count(&directory);
    let wrong_type = run(&directory, "DURATION_SUM", "duration-days,number", &[]);
    assert_eq!(wrong_type.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&wrong_type.stderr).contains("Duration"));

    let duplicate = run(
        &directory,
        "DURATION_SUM",
        "duration-days,duration-days",
        &[],
    );
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("repeated"));
    assert_eq!(node_count(&directory), before);
}

#[test]
fn unsupported_units_and_bad_flag_combinations_are_invalid_input() {
    let directory = fixture();
    let before = node_count(&directory);
    let unsupported = run(
        &directory,
        "DURATION_SUM",
        "duration-days,duration-minutes",
        &["--unit", "months"],
    );
    assert_eq!(unsupported.status.code(), Some(2));
    let unit_without_result = run(
        &directory,
        "DURATION_SUM",
        "duration-days,duration-minutes",
        &["--result-unit", "days"],
    );
    assert_eq!(unit_without_result.status.code(), Some(2));
    assert_eq!(node_count(&directory), before);
}
