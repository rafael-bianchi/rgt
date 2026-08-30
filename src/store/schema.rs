use rusqlite::{Connection, Result};

/// Initializes the SQLite schema (WAL mode, tables, indexes) idempotently.
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
            last_checked_at TEXT NOT NULL,
            dev INTEGER,
            ino INTEGER
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

        CREATE TABLE IF NOT EXISTS project_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS derivation_edges (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_node_id TEXT NOT NULL REFERENCES tracked_nodes(id) ON DELETE CASCADE,
            child_node_id TEXT NOT NULL REFERENCES tracked_nodes(id) ON DELETE CASCADE,
            operation_type TEXT NOT NULL,
            expression TEXT,
            UNIQUE(parent_node_id, child_node_id),
            CHECK(parent_node_id <> child_node_id)
        );

        CREATE INDEX IF NOT EXISTS idx_derivation_edges_parent 
        ON derivation_edges(parent_node_id);

        CREATE INDEX IF NOT EXISTS idx_derivation_edges_child 
        ON derivation_edges(child_node_id);
        ",
    )?;

    // Idempotently add the on-disk identity columns to pre-existing databases
    // (additive `ALTER TABLE ... ADD COLUMN`, guarded by PRAGMA table_info), then
    // index them. The index must come after the columns exist.
    ensure_column(conn, "source_documents", "dev", "INTEGER")?;
    ensure_column(conn, "source_documents", "ino", "INTEGER")?;
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_source_documents_identity
         ON source_documents(dev, ino);",
    )?;
    Ok(())
}

/// Adds a column to a table if it does not already exist (idempotent, additive).
fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<()> {
    let exists: bool = conn
        .prepare(&format!("PRAGMA table_info({})", table))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column);
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column, decl
        ))?;
    }
    Ok(())
}
