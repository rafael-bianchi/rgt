//! Store & graph integrity tests (feature 024):
//! cycle/self-loop rejection at the write path, non-zero busy_timeout, and
//! value-column updates on re-record.

use chrono::Utc;
use rgt::store::queries::{get_tracked_node, insert_derivation_edge, insert_tracked_node};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};

fn root_node(
    id: &str,
    value: ValueData,
    source_doc_id: Option<i64>,
    line_number: Option<u32>,
) -> TrackedNode {
    TrackedNode {
        id: id.to_string(),
        node_type: NodeType::Root,
        value_kind: value.kind(),
        value,
        source_doc_id,
        line_number,
        is_stale: false,
        stale_reason: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

// ---- T004 (US1): cycle/self-loop rejection at the write path ----

#[test]
fn self_loop_edge_is_rejected() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    insert_tracked_node(conn, &root_node("a", ValueData::Number(1.0), None, None)).unwrap();

    let err = insert_derivation_edge(conn, "a", "a", "EXPRESSION", None).unwrap_err();
    assert!(err.contains("self-loop"), "{}", err);
}

#[test]
fn two_node_cycle_edge_is_rejected() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    insert_tracked_node(conn, &root_node("a", ValueData::Number(1.0), None, None)).unwrap();
    insert_tracked_node(conn, &root_node("b", ValueData::Number(2.0), None, None)).unwrap();

    // A -> B is fine.
    insert_derivation_edge(conn, "a", "b", "EXPRESSION", None).unwrap();

    // B -> A would close the cycle.
    let err = insert_derivation_edge(conn, "b", "a", "EXPRESSION", None).unwrap_err();
    assert!(err.contains("cycle"), "{}", err);
}

#[test]
fn multi_node_cycle_edge_is_rejected() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    for id in ["a", "b", "c"] {
        insert_tracked_node(conn, &root_node(id, ValueData::Number(1.0), None, None)).unwrap();
    }
    insert_derivation_edge(conn, "a", "b", "EXPRESSION", None).unwrap();
    insert_derivation_edge(conn, "b", "c", "EXPRESSION", None).unwrap();

    // C -> A closes A->B->C->A.
    let err = insert_derivation_edge(conn, "c", "a", "EXPRESSION", None).unwrap_err();
    assert!(err.contains("cycle"), "{}", err);
}

#[test]
fn non_cyclic_edge_still_succeeds() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    for id in ["a", "b", "c"] {
        insert_tracked_node(conn, &root_node(id, ValueData::Number(1.0), None, None)).unwrap();
    }
    insert_derivation_edge(conn, "a", "b", "EXPRESSION", None).unwrap();
    insert_derivation_edge(conn, "a", "c", "EXPRESSION", None).unwrap();
    insert_derivation_edge(conn, "b", "c", "EXPRESSION", None).unwrap(); // still acyclic
}

// ---- T008 (US2): non-zero busy_timeout (a second connection waits on a held
// write lock instead of failing immediately with SQLITE_BUSY) ----

#[test]
fn connections_wait_on_locks_instead_of_immediate_busy() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("t.db");
    let db1 = DbStore::open(&db_path).unwrap();
    let db2 = DbStore::open(&db_path).unwrap();

    // conn1 holds the write lock (uncommitted write).
    let conn1 = db1.conn();
    conn1.execute_batch("BEGIN IMMEDIATE;").unwrap();
    conn1
        .execute(
            "INSERT INTO source_documents (file_path, mtime_nsec, file_size, blake3_hash, last_checked_at)
             VALUES ('x', 0, 0, 'h', 't')",
            [],
        )
        .unwrap();

    // conn2's write in a thread: with a non-zero busy_timeout it must wait for
    // conn1 to commit and then succeed (busy_timeout=0 would fail immediately).
    let handle = std::thread::spawn(move || {
        let conn2 = db2.conn();
        conn2
            .execute(
                "INSERT INTO source_documents (file_path, mtime_nsec, file_size, blake3_hash, last_checked_at)
                 VALUES ('y', 0, 0, 'h', 't')",
                [],
            )
            .is_ok()
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    conn1.execute_batch("COMMIT;").unwrap();
    assert!(
        handle.join().unwrap(),
        "conn2's write must wait for the lock (busy_timeout > 0)"
    );
}

// ---- T011 (US3): re-record updates value-bearing columns ----

#[test]
fn re_record_updates_value_columns() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();

    use rgt::store::queries::upsert_source_document;
    let doc1 = upsert_source_document(conn, "a.csv", 1, 1, "h1", None, None).unwrap();
    let doc2 = upsert_source_document(conn, "b.csv", 1, 1, "h2", None, None).unwrap();

    insert_tracked_node(
        conn,
        &root_node("x", ValueData::Number(100.0), Some(doc1.id), Some(1)),
    )
    .unwrap();

    // Re-record the same ID with a different value/source/line.
    insert_tracked_node(
        conn,
        &root_node("x", ValueData::Number(200.0), Some(doc2.id), Some(5)),
    )
    .unwrap();

    let node = get_tracked_node(conn, "x").unwrap().unwrap();
    assert_eq!(
        node.value,
        ValueData::Number(200.0),
        "value column must update"
    );
    assert_eq!(
        node.source_doc_id,
        Some(doc2.id),
        "source_doc_id must update"
    );
    assert_eq!(node.line_number, Some(5), "line_number must update");
}

// ---- T013 (US4): hook capture records the batch atomically ----

#[test]
fn hook_capture_records_batch_atomically() {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    use std::sync::Mutex;

    static CWD_MUTEX: Mutex<()> = Mutex::new(());
    let _guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let file = dir.path().join("data.csv");
    std::fs::write(&file, "placeholder\n").unwrap();

    let event = format!(
        r#"{{"event":"PostToolUse","tool_name":"ReadLocalFile","tool_input":{{"path":"{}"}},"tool_response":{{"content":"amount,120000\ncosts,60000\nprofit,60000\n"}}}}"#,
        file.display()
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["hook", "post", "--agent", "claude-code"])
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn rgt hook post");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(event.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "hook must fail open with success");

    let db = DbStore::open_in_project(dir.path()).unwrap();
    let nodes = rgt::store::queries::list_all_nodes(db.conn()).unwrap();
    assert_eq!(nodes.len(), 3, "all values must be recorded in the batch");
}

// ---- 029 / US1 (T005): schema version stamping and skew detection ----

#[test]
fn unversioned_store_is_stamped_to_schema_version() {
    use rgt::store::schema::{schema_version, stamp_or_migrate_schema, SCHEMA_VERSION};
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    // A fresh store is stamped on open; verify a second pass is a no-op.
    assert_eq!(schema_version(conn).unwrap(), SCHEMA_VERSION);
    stamp_or_migrate_schema(conn).unwrap();
    assert_eq!(schema_version(conn).unwrap(), SCHEMA_VERSION);
}

#[test]
fn newer_store_version_is_rejected() {
    use rgt::store::schema::{stamp_or_migrate_schema, SCHEMA_VERSION};
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    // Simulate a store created by a future build.
    conn.execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION + 1))
        .unwrap();
    let err = stamp_or_migrate_schema(conn).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("newer than this build supports"), "{}", msg);
}

#[test]
fn equal_version_is_a_noop() {
    use rgt::store::schema::{schema_version, SCHEMA_VERSION};
    let db = DbStore::open_in_memory().unwrap();
    assert_eq!(schema_version(db.conn()).unwrap(), SCHEMA_VERSION);
}
