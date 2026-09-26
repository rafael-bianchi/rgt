use chrono::{Duration, Utc};
use rgt::store::queries::{
    get_parent_edges, get_tracked_node, insert_tracked_node, list_all_nodes,
};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let db = DbStore::open_in_project(directory.path()).unwrap();
    for (id, seconds) in [
        ("days", 3_888_000),
        ("minutes", 2_700),
        ("negative", -900),
        ("positive", 1_200),
    ] {
        let value = ValueData::Duration(Duration::seconds(seconds));
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

fn derive(directory: &TempDir, operation: &str, parents: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["derive", "--parents", parents, "--operation", operation])
        .current_dir(directory.path())
        .output()
        .unwrap()
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
fn sum_and_average_are_typed_and_link_every_parent() {
    let directory = fixture();
    for (operation, expected) in [("DURATION_SUM", 3_890_700), ("DURATION_AVG", 1_945_350)] {
        let output = derive(&directory, operation, "days,minutes");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let id = output_id(&output);
        let db = DbStore::open_in_project(directory.path()).unwrap();
        assert_eq!(
            get_tracked_node(db.conn(), &id).unwrap().unwrap().value,
            ValueData::Duration(Duration::seconds(expected))
        );
        let edges = get_parent_edges(db.conn(), &id).unwrap();
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|edge| edge.operation_type == operation));
    }
}

#[test]
fn aggregate_parent_permutations_reuse_one_node_and_edge_set() {
    let directory = fixture();
    for operation in ["DURATION_SUM", "DURATION_AVG"] {
        let first = derive(&directory, operation, "days,minutes");
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        let second = derive(&directory, operation, "minutes,days");
        assert!(
            second.status.success(),
            "{}",
            String::from_utf8_lossy(&second.stderr)
        );
        assert_eq!(output_id(&first), output_id(&second));
        let db = DbStore::open_in_project(directory.path()).unwrap();
        assert_eq!(
            get_parent_edges(db.conn(), &output_id(&first))
                .unwrap()
                .len(),
            2
        );
    }
    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(list_all_nodes(db.conn()).unwrap().len(), 6);
}

#[test]
fn signed_duration_aggregate_preserves_exact_value_and_parent_links() {
    let directory = fixture();
    let output = derive(&directory, "DURATION_AVG", "negative,positive");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let id = output_id(&output);
    let db = DbStore::open_in_project(directory.path()).unwrap();
    assert_eq!(
        get_tracked_node(db.conn(), &id).unwrap().unwrap().value,
        ValueData::Duration(Duration::seconds(150))
    );
    assert_eq!(get_parent_edges(db.conn(), &id).unwrap().len(), 2);
}

#[test]
fn repeated_parent_rejection_writes_nothing_and_turtle_exports_activities() {
    let directory = fixture();
    let before = list_all_nodes(DbStore::open_in_project(directory.path()).unwrap().conn())
        .unwrap()
        .len();
    let duplicate = derive(&directory, "DURATION_SUM", "days,days");
    assert_eq!(duplicate.status.code(), Some(2));
    assert_eq!(
        list_all_nodes(DbStore::open_in_project(directory.path()).unwrap().conn())
            .unwrap()
            .len(),
        before
    );

    let output = derive(&directory, "DURATION_SUM", "days,minutes");
    assert!(output.status.success());
    let turtle = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(turtle.status.success());
    let turtle = String::from_utf8_lossy(&turtle.stdout);
    assert!(turtle.contains("DURATION_SUM"));
    assert!(turtle.contains("prov:used"));
}
