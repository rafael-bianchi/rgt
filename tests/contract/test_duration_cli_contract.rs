use chrono::Utc;
use rgt::store::queries::{get_tracked_node, insert_tracked_node, list_all_nodes};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let db = DbStore::open_in_project(directory.path()).unwrap();
    for (id, date) in [
        ("date-a", "2026-01-01T00:00:00Z"),
        ("date-b", "2026-02-15T00:00:00Z"),
    ] {
        let value = ValueData::Date(
            chrono::DateTime::parse_from_rfc3339(date)
                .unwrap()
                .with_timezone(&Utc),
        );
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

fn run(directory: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(args)
        .current_dir(directory.path())
        .output()
        .expect("run rgt")
}

fn node_count(directory: &TempDir) -> usize {
    list_all_nodes(DbStore::open_in_project(directory.path()).unwrap().conn())
        .unwrap()
        .len()
}

#[test]
fn derive_calculates_gap_and_reports_the_selected_unit() {
    let directory = fixture();
    let output = run(
        &directory,
        &[
            "derive",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--unit",
            "days",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("45 days"), "{stdout}");
    assert!(stdout.contains("3888000 seconds"), "{stdout}");
    assert_eq!(node_count(&directory), 3);
}

#[test]
fn equivalent_day_claim_is_checked_before_recording() {
    let directory = fixture();
    let output = run(
        &directory,
        &[
            "derive",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "45",
            "--result-unit",
            "days",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(node_count(&directory), 3);
}

#[test]
fn wrong_day_claim_fails_without_writing_nodes() {
    let directory = fixture();
    let output = run(
        &directory,
        &[
            "derive",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "44",
            "--result-unit",
            "days",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("expected 3888000 seconds"), "{stderr}");
    assert_eq!(node_count(&directory), 2);
}

#[test]
fn no_unit_date_diff_keeps_single_line_node_id_output() {
    let directory = fixture();
    let output = run(
        &directory,
        &[
            "derive",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "3888000",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("Derived node: node_drv_"), "{stdout}");
    assert_eq!(stdout.lines().count(), 1, "{stdout}");

    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert!(
        get_tracked_node(db.conn(), stdout.trim().split(':').nth(1).unwrap().trim())
            .unwrap()
            .is_some()
    );
}

#[test]
fn equivalent_claim_and_display_units_reuse_one_exact_duration_node() {
    let directory = fixture();
    let db = DbStore::open_in_project(directory.path()).unwrap();
    let value = ValueData::Date(
        chrono::DateTime::parse_from_rfc3339("2026-01-08T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
    );
    let now = Utc::now();
    insert_tracked_node(
        db.conn(),
        &TrackedNode {
            id: "date-week".into(),
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
    drop(db);
    let baseline = node_count(&directory);
    let cases = [
        ("seconds", "604800", "604800 seconds"),
        ("minutes", "10080", "10080 minutes"),
        ("hours", "168", "168 hours"),
        ("days", "7", "7 days"),
        ("weeks", "1", "1 week"),
    ];
    let mut expected_id = None;

    for (unit, claim, display) in cases {
        let output = run(
            &directory,
            &[
                "derive",
                "--parents",
                "date-a,date-week",
                "--operation",
                "DATE_DIFF",
                "--result",
                claim,
                "--result-unit",
                unit,
                "--unit",
                unit,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let node_id = stdout
            .lines()
            .next()
            .unwrap()
            .strip_prefix("Derived node: ")
            .unwrap()
            .to_string();
        assert!(stdout.contains(display), "{stdout}");
        assert!(stdout.contains("604800 seconds"), "{stdout}");
        if let Some(expected_id) = &expected_id {
            assert_eq!(
                &node_id, expected_id,
                "unit {unit} created another identity"
            );
        } else {
            expected_id = Some(node_id);
        }
    }

    assert_eq!(node_count(&directory), baseline + 1);
}
