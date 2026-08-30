use crate::store::schema::{initialize_schema, stamp_or_migrate_schema};
use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;

pub struct DbStore {
    conn: Connection,
}

impl DbStore {
    pub fn open_in_project<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let rgt_dir = project_root.as_ref().join(".rgt");
        if !rgt_dir.exists() {
            fs::create_dir_all(&rgt_dir).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("Failed to create .rgt dir: {}", e)),
                )
            })?;
        }

        let db_path = rgt_dir.join("store.db");
        Self::open(db_path)
    }

    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // FR-003: wait on locks instead of failing immediately with SQLITE_BUSY
        // (SQLite's default busy_timeout is 0). Bounded to avoid indefinite hangs.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        initialize_schema(&conn)?;
        // FR-001: stamp or migrate the schema version; fail fast on a newer store.
        stamp_or_migrate_schema(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        initialize_schema(&conn)?;
        stamp_or_migrate_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[allow(dead_code)]
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}
