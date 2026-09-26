use chrono::{Duration, Utc};
use rgt::store::queries::{
    get_parent_edges, get_tracked_node, insert_derivation_edge, insert_tracked_node, list_all_nodes,
};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::process::Command;
use tempfile::TempDir;

const SECONDS: i64 = 45 * 86_400;

fn setup(legacy: Option<&str>) -> TempDir {
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
        insert_node(db.conn(), id, NodeType::Root, value);
    }
    if let Some(id) = legacy {
        insert_node(
            db.conn(),
            id,
            NodeType::Derived,
            ValueData::Duration(Duration::seconds(SECONDS)),
        );
        insert_derivation_edge(db.conn(), "date-a", id, "DATE_DIFF", None).unwrap();
        insert_derivation_edge(db.conn(), "date-b", id, "DATE_DIFF", None).unwrap();
    }
    drop(db);
    directory
}

fn setup_same_parent(legacy: Option<&str>) -> TempDir {
    let directory = setup(None);
    if let Some(id) = legacy {
        let db = DbStore::open_in_project(directory.path()).unwrap();
        insert_node(
            db.conn(),
            id,
            NodeType::Derived,
            ValueData::Duration(Duration::zero()),
        );
        // The graph stores one edge per distinct parent, while DATE_DIFF's
        // identity still records both ordered operand positions.
        insert_derivation_edge(db.conn(), "date-a", id, "DATE_DIFF", None).unwrap();
    }
    directory
}

fn insert_node(conn: &rusqlite::Connection, id: &str, node_type: NodeType, value: ValueData) {
    let now = Utc::now();
    insert_tracked_node(
        conn,
        &TrackedNode {
            id: id.into(),
            node_type,
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

fn legacy_id(seconds: i64) -> String {
    TrackedNode::generate_derived_id(
        &["date-a".into(), "date-b".into()],
        "DATE_DIFF",
        &ValueData::Duration(Duration::seconds(seconds)),
    )
}

fn same_parent_legacy_id(seconds: i64) -> String {
    TrackedNode::generate_derived_id(
        &["date-a".into(), "date-a".into()],
        "DATE_DIFF",
        &ValueData::Duration(Duration::seconds(seconds)),
    )
}

fn derive(directory: &TempDir) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args([
            "derive",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--unit",
            "days",
        ])
        .current_dir(directory.path())
        .output()
        .expect("run rgt derive")
}

fn derive_same_parent(directory: &TempDir) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args([
            "derive",
            "--parents",
            "date-a,date-a",
            "--operation",
            "DATE_DIFF",
            "--unit",
            "seconds",
        ])
        .current_dir(directory.path())
        .output()
        .expect("run rgt derive with repeated Date parent")
}

fn output_id(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap()
        .strip_prefix("Derived node: ")
        .unwrap()
        .to_string()
}

#[test]
fn computed_duration_has_two_parent_edges_and_exports_a_prov_activity() {
    let directory = setup(None);
    let output = derive(&directory);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let id = output_id(&output);
    let db = DbStore::open_in_project(directory.path()).unwrap();
    let node = get_tracked_node(db.conn(), &id).unwrap().unwrap();
    assert_eq!(node.value, ValueData::Duration(Duration::seconds(SECONDS)));
    let mut parents = get_parent_edges(db.conn(), &id)
        .unwrap()
        .into_iter()
        .map(|edge| edge.parent_node_id)
        .collect::<Vec<_>>();
    parents.sort();
    assert_eq!(parents, ["date-a", "date-b"]);

    let turtle = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(turtle.status.success());
    let turtle = String::from_utf8(turtle.stdout).unwrap();
    assert!(turtle.contains("^^xsd:duration"));
    assert!(turtle.contains("prov:used"));
    assert!(turtle.contains("DATE_DIFF"));
}

#[test]
fn matching_historical_12_character_id_is_reused() {
    let id = legacy_id(SECONDS)[.."node_drv_".len() + 12].to_string();
    let directory = setup(Some(&id));
    let output = derive(&directory);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output_id(&output), id);
    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 3);
}

#[test]
fn matching_historical_32_character_id_is_reused() {
    let id = legacy_id(SECONDS);
    let directory = setup(Some(&id));
    let output = derive(&directory);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output_id(&output), id);
    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 3);
}

#[test]
fn conflicting_legacy_candidate_is_left_untouched() {
    let id = legacy_id(SECONDS);
    let directory = setup(None);
    {
        let db = DbStore::open_in_project(directory.path()).unwrap();
        insert_node(
            db.conn(),
            &id,
            NodeType::Derived,
            ValueData::Duration(Duration::seconds(SECONDS + 1)),
        );
    }
    let output = derive(&directory);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(output_id(&output), id);
    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(
        get_tracked_node(db.conn(), &id).unwrap().unwrap().value,
        ValueData::Duration(Duration::seconds(SECONDS + 1))
    );
    assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 4);
}

#[test]
fn repeated_date_parent_is_reusable_and_exports_both_ordered_usages() {
    let directory = setup_same_parent(None);
    let first = derive_same_parent(&directory);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let id = output_id(&first);
    let expected_id = TrackedNode::generate_duration_derived_id(
        &["date-a".into(), "date-a".into()],
        "DATE_DIFF",
        0,
    )
    .unwrap();
    assert_eq!(id, expected_id);

    let db = DbStore::open_in_project(directory.path()).unwrap();
    let node = get_tracked_node(db.conn(), &id).unwrap().unwrap();
    assert_eq!(node.value, ValueData::Duration(Duration::zero()));
    let edges = get_parent_edges(db.conn(), &id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].parent_node_id, "date-a");
    drop(db);

    let repeated = derive_same_parent(&directory);
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert_eq!(output_id(&repeated), id);

    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 3);
    assert_eq!(get_parent_edges(db.conn(), &id).unwrap().len(), 1);
    drop(db);

    let turtle = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(turtle.status.success());
    let turtle = String::from_utf8(turtle.stdout).unwrap();
    assert!(turtle.contains("prov:Activity"));
    assert!(turtle.contains("rgt:operandIndex \"0\"^^xsd:integer"));
    assert!(turtle.contains("rgt:operandIndex \"1\"^^xsd:integer"));
    assert!(turtle.contains("\"PT0S\"^^xsd:duration"));
}

#[test]
fn repeated_date_parent_reuses_full_and_short_historical_ids() {
    let historical = same_parent_legacy_id(0);
    for id in [
        historical.clone(),
        historical[.."node_drv_".len() + 12].to_string(),
    ] {
        let directory = setup_same_parent(Some(&id));
        let output = derive_same_parent(&directory);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output_id(&output), id);
        let db = DbStore::open_in_project(directory.path()).unwrap();
        assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 3);
        let edges = get_parent_edges(db.conn(), &id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].parent_node_id, "date-a");
        drop(db);

        let turtle = Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args(["graph", "--format", "ttl"])
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(turtle.status.success());
        let turtle = String::from_utf8(turtle.stdout).unwrap();
        assert!(turtle.contains("prov:wasGeneratedBy"));
        assert!(turtle.contains("rgt:operandIndex \"0\"^^xsd:integer"));
        assert!(turtle.contains("rgt:operandIndex \"1\"^^xsd:integer"));
    }
}
