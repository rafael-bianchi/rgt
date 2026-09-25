use crate::store::schema::{initialize_schema, stamp_or_migrate_schema};
use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn schema_init_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct DbStore {
    conn: Connection,
}

impl DbStore {
    pub fn open_in_project<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        Self::open_in_project_with_timeout(project_root, std::time::Duration::from_secs(5))
    }

    pub fn open_in_project_with_timeout<P: AsRef<Path>>(
        project_root: P,
        busy_timeout: std::time::Duration,
    ) -> Result<Self> {
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
        Self::open_with_timeout(db_path, busy_timeout)
    }

    #[allow(dead_code)]
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::open_with_timeout(db_path, std::time::Duration::from_secs(5))
    }

    pub fn open_with_timeout<P: AsRef<Path>>(
        db_path: P,
        busy_timeout: std::time::Duration,
    ) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // FR-003: wait on locks instead of failing immediately with SQLITE_BUSY
        // (SQLite's default busy_timeout is 0). Bounded to avoid indefinite hangs.
        conn.busy_timeout(busy_timeout)?;
        // `PRAGMA journal_mode=WAL` can report SQLITE_BUSY while several
        // threads initialize the same new store. Serialize initialization in
        // this process; SQLite still coordinates independent processes.
        let _schema_guard = schema_init_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        initialize_schema(&conn)?;
        // FR-001: stamp or migrate the schema version; fail fast on a newer store.
        stamp_or_migrate_schema(&conn)?;
        // Establish the immutable export namespace as part of opening any
        // current or legacy store, before a caller can copy or export it.
        super::queries::get_or_create_rdf_project_id(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        initialize_schema(&conn)?;
        stamp_or_migrate_schema(&conn)?;
        super::queries::get_or_create_rdf_project_id(&conn)?;
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
