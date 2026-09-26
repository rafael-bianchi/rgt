use rgt::store::{
    queries::insert_capture_association,
    schema::{initialize_schema, schema_version},
    DbStore,
};
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn version_one_store_migrates_capture_table_atomically_and_preserves_rows() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("store.db");
    {
        let conn = Connection::open(&path).unwrap();
        initialize_schema(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES ('historical-a','ROOT','NUMBER',3.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
             INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES ('historical-b','ROOT','NUMBER',4.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
             INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES ('historical-derived','DERIVED','NUMBER',7.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
             INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES ('historical-a','historical-derived','EXPRESSION','a + b');
             INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES ('historical-b','historical-derived','EXPRESSION','a + b');",
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 1").unwrap();
    }

    let db = DbStore::open(&path).unwrap();
    assert_eq!(schema_version(db.conn()).unwrap(), 2);
    let preserved: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM tracked_nodes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(preserved, 3);
    let preserved_edges: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM derivation_edges", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(preserved_edges, 2);
    let capture_table: i64 = db
        .conn()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='capture_associations'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(capture_table, 1);
}

#[test]
fn unversioned_store_migrates_directly_to_v2_without_backfilling_attribution() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    {
        let conn = Connection::open(&path).unwrap();
        initialize_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('old-node','ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 0").unwrap();
    }
    let db = DbStore::open(&path).unwrap();
    assert_eq!(schema_version(db.conn()).unwrap(), 2);
    let captures: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM capture_associations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(captures, 0);
}

#[test]
fn failed_v1_to_v2_migration_rolls_back_the_version_stamp() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("broken.db");
    {
        let conn = Connection::open(&path).unwrap();
        initialize_schema(&conn).unwrap();
        conn.execute_batch(
            "CREATE TABLE capture_associations (preexisting TEXT);
             PRAGMA user_version = 0;",
        )
        .unwrap();
    }
    assert!(DbStore::open(&path).is_err());
    let conn = Connection::open(&path).unwrap();
    assert_eq!(schema_version(&conn).unwrap(), 0);
}

#[test]
fn migration_rejects_newer_stores_and_capture_rows_cascade_with_nodes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("newer.db");
    {
        let conn = Connection::open(&path).unwrap();
        initialize_schema(&conn).unwrap();
        conn.execute_batch("PRAGMA user_version = 3").unwrap();
    }
    let error = DbStore::open(&path).err().unwrap();
    assert!(error.to_string().contains("newer than this build supports"));

    let db = DbStore::open_in_memory().unwrap();
    db.conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('captured','ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    insert_capture_association(db.conn(), "captured", "cursor").unwrap();
    db.conn()
        .execute("DELETE FROM tracked_nodes WHERE id='captured'", [])
        .unwrap();
    let captures: i64 = db
        .conn()
        .query_row("SELECT count(*) FROM capture_associations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(captures, 0);
}
