use rusqlite::{Connection, Result};

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS source_documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            mtime_nsec INTEGER NOT NULL,
            file_size INTEGER NOT NULL,
            blake3_hash TEXT NOT NULL,
            last_checked_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_source_documents_path 
        ON source_documents(file_path);

        CREATE TABLE IF NOT EXISTS tracked_nodes (
            id TEXT PRIMARY KEY,
            node_type TEXT NOT NULL CHECK(node_type IN ('ROOT', 'DERIVED')),
            value_kind TEXT NOT NULL CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION')),
            number_val REAL,
            date_val TEXT,
            duration_secs INTEGER,
            source_doc_id INTEGER REFERENCES source_documents(id) ON DELETE SET NULL,
            line_number INTEGER,
            is_stale INTEGER NOT NULL DEFAULT 0 CHECK(is_stale IN (0, 1)),
            stale_reason TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tracked_nodes_source_doc 
        ON tracked_nodes(source_doc_id);

        CREATE INDEX IF NOT EXISTS idx_tracked_nodes_stale 
        ON tracked_nodes(is_stale);

        CREATE TABLE IF NOT EXISTS derivation_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_node_id TEXT NOT NULL REFERENCES tracked_nodes(id) ON DELETE CASCADE,
            child_node_id TEXT NOT NULL REFERENCES tracked_nodes(id) ON DELETE CASCADE,
            operation_type TEXT NOT NULL,
            expression TEXT,
            UNIQUE(parent_node_id, child_node_id)
        );

        CREATE INDEX IF NOT EXISTS idx_derivation_edges_parent 
        ON derivation_edges(parent_node_id);

        CREATE INDEX IF NOT EXISTS idx_derivation_edges_child 
        ON derivation_edges(child_node_id);
        ",
    )?;
    Ok(())
}
