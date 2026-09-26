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
        (
            "duration-one-second",
            ValueData::Duration(Duration::seconds(1)),
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

fn json_output(output: std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn query_without_unit_preserves_default_and_json_wrappers() {
    let directory = fixture();
    let wrapped = json_output(run(&directory, &["query", "duration-days"]));
    let direct = json_output(run(&directory, &["query", "duration-days", "--json"]));

    assert!(wrapped.get("lineage").is_some());
    assert!(direct.get("lineage").is_none());
    for response in [&wrapped["lineage"], &direct] {
        let step = &response["lineage_steps"][0];
        assert_eq!(step["value"], "45d");
        assert!(step.get("duration_seconds").is_none());
        assert!(step.get("selected_unit_display").is_none());
    }
}

#[test]
fn query_unit_adds_exact_display_only_to_requested_duration() {
    let directory = fixture();
    let derive = run(
        &directory,
        &[
            "derive",
            "--parents",
            "duration-days,duration-minutes",
            "--operation",
            "DURATION_SUM",
        ],
    );
    assert!(
        derive.status.success(),
        "{}",
        String::from_utf8_lossy(&derive.stderr)
    );
    let node_id = String::from_utf8(derive.stdout)
        .unwrap()
        .strip_prefix("Derived node: ")
        .unwrap()
        .trim()
        .to_string();
    let before = node_count(&directory);

    let response = json_output(run(
        &directory,
        &["query", &node_id, "--unit", "minutes", "--json"],
    ));
    let steps = response["lineage_steps"].as_array().unwrap();
    let requested = &steps[0];
    assert_eq!(requested["duration_seconds"], "3890700");
    assert_eq!(requested["selected_unit_display"]["unit"], "minutes");
    assert_eq!(requested["selected_unit_display"]["decimal"], "64845");
    assert_eq!(requested["selected_unit_display"]["approximate"], false);
    assert_eq!(requested["selected_unit_display"]["text"], "64845 minutes");
    assert_eq!(requested["value"], "45d 0h 45m");
    assert!(steps[1..].iter().all(|step| {
        step.get("duration_seconds").is_none() && step.get("selected_unit_display").is_none()
    }));
    assert_eq!(node_count(&directory), before);
}

#[test]
fn query_unit_marks_repeating_conversion_approximate() {
    let directory = fixture();
    for (unit, approximate) in [
        ("seconds", false),
        ("minutes", true),
        ("hours", true),
        ("days", true),
        ("weeks", true),
    ] {
        let response = json_output(run(
            &directory,
            &["query", "duration-one-second", "--unit", unit, "--json"],
        ));
        let step = &response["lineage_steps"][0];
        let display = &step["selected_unit_display"];
        assert_eq!(step["duration_seconds"], "1");
        assert_eq!(display["unit"], unit);
        assert_eq!(display["approximate"], approximate);
        if unit == "minutes" {
            assert_eq!(display["decimal"], "0.016666666667");
            assert_eq!(display["text"], "approximately 0.016666666667 minutes");
        }
    }
}

#[test]
fn query_all_fixed_units_preserves_duration_identity_value_and_lineage() {
    let directory = fixture();
    let derive = run(
        &directory,
        &[
            "derive",
            "--parents",
            "duration-days,duration-minutes",
            "--operation",
            "DURATION_SUM",
        ],
    );
    assert!(
        derive.status.success(),
        "{}",
        String::from_utf8_lossy(&derive.stderr)
    );
    let node_id = String::from_utf8(derive.stdout)
        .unwrap()
        .strip_prefix("Derived node: ")
        .unwrap()
        .trim()
        .to_string();
    let baseline = json_output(run(&directory, &["query", &node_id, "--json"]));
    let before = node_count(&directory);
    let cases = [
        ("seconds", "3890700", false),
        ("minutes", "64845", false),
        ("hours", "1080.75", false),
        ("days", "45.03125", false),
        ("weeks", "6.433035714286", true),
    ];

    for (unit, decimal, approximate) in cases {
        let response = json_output(run(
            &directory,
            &["query", &node_id, "--unit", unit, "--json"],
        ));
        let requested = &response["lineage_steps"][0];
        let original = &baseline["lineage_steps"][0];
        assert_eq!(requested["node_id"], original["node_id"]);
        assert_eq!(requested["value"], original["value"]);
        assert_eq!(requested["parent_ids"], original["parent_ids"]);
        assert_eq!(requested["value"], "45d 0h 45m");
        assert_eq!(requested["duration_seconds"], "3890700");
        assert_eq!(requested["selected_unit_display"]["unit"], unit);
        assert_eq!(requested["selected_unit_display"]["decimal"], decimal);
        assert_eq!(
            requested["selected_unit_display"]["approximate"],
            approximate
        );
        assert!(response["lineage_steps"].as_array().unwrap()[1..]
            .iter()
            .all(|step| {
                step.get("duration_seconds").is_none()
                    && step.get("selected_unit_display").is_none()
            }));
    }
    assert_eq!(node_count(&directory), before);
}

#[test]
fn query_unit_rejects_non_duration_without_graph_writes() {
    let directory = fixture();
    let before = node_count(&directory);
    for node_id in ["number", "date"] {
        let output = run(&directory, &["query", node_id, "--unit", "days"]);
        assert_eq!(output.status.code(), Some(2), "target: {node_id}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Duration"),
            "target: {node_id}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(node_count(&directory), before);
}
