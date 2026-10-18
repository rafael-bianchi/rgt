use crate::store::schema::{initialize_schema, stamp_or_migrate_schema};
use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn schema_init_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug)]
pub struct DbStore {
    conn: Connection,
}

impl DbStore {
    /// Walks up parent directories to locate an existing `.rgt/store.db`.
    pub fn find_project_root<P: AsRef<Path>>(start: P) -> Option<std::path::PathBuf> {
        let mut curr = start.as_ref().to_path_buf();
        if curr.is_relative() {
            if let Ok(abs) = std::fs::canonicalize(&curr) {
                curr = abs;
            }
        }
        loop {
            if curr.join(".rgt").join("store.db").is_file() {
                return Some(curr);
            }
            if !curr.pop() {
                break;
            }
        }
        None
    }

    /// When a relative `file_path` is recorded from a subdirectory inside `project_root`,
    /// rebases it relative to `project_root` so subsequent `project_root.join(rel_path)`
    /// checks in `evaluate_and_invalidate_all` locate the file on disk. When CWD equals
    /// `project_root` (or `file_path` is absolute or `<stdin>`), preserves `file_path` verbatim.
    pub fn normalize_path_for_project(file_path: &str, project_root: &Path) -> String {
        if file_path == "<stdin>" {
            return file_path.to_string();
        }
        let rebased = if !Path::new(file_path).is_relative() {
            file_path.to_string()
        } else {
            let Ok(cwd_canon) = std::env::current_dir().and_then(std::fs::canonicalize) else {
                return file_path.replace('\\', "/");
            };
            let Ok(root_canon) = std::fs::canonicalize(project_root) else {
                return file_path.replace('\\', "/");
            };
            if cwd_canon == root_canon {
                file_path.to_string()
            } else if let Ok(sub_rel) = cwd_canon.strip_prefix(&root_canon) {
                sub_rel.join(file_path).to_string_lossy().to_string()
            } else {
                file_path.to_string()
            }
        };
        rebased.replace('\\', "/")
    }

    /// Initializes or opens `.rgt/store.db` directly in `project_root` without walking
    /// up parent directories (used by `rgt init`, analogous to `git init`).
    pub fn init_in_project<P: AsRef<Path>>(project_root: P) -> Result<Self> {
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
        Self::open_with_timeout(db_path, std::time::Duration::from_secs(5))
    }

    pub fn open_existing_in_project<P: AsRef<Path>>(project_root: P) -> Result<Option<Self>> {
        Self::open_existing_in_project_with_timeout(project_root, std::time::Duration::from_secs(5))
    }

    pub fn open_existing_in_project_with_timeout<P: AsRef<Path>>(
        project_root: P,
        busy_timeout: std::time::Duration,
    ) -> Result<Option<Self>> {
        let Some(effective_root) = Self::find_project_root(&project_root) else {
            return Ok(None);
        };
        let db_path = effective_root.join(".rgt").join("store.db");
        if !db_path.is_file() {
            return Ok(None);
        }
        Self::open_with_timeout(db_path, busy_timeout).map(Some)
    }

    pub fn open_in_project<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        Self::open_in_project_with_timeout(project_root, std::time::Duration::from_secs(5))
    }

    pub fn open_in_project_with_timeout<P: AsRef<Path>>(
        project_root: P,
        busy_timeout: std::time::Duration,
    ) -> Result<Self> {
        let effective_root = Self::find_project_root(&project_root)
            .unwrap_or_else(|| project_root.as_ref().to_path_buf());
        let rgt_dir = effective_root.join(".rgt");
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
        // Fail fast if the database was created by a newer build BEFORE executing any DDL.
        let current_version = crate::store::schema::schema_version(&conn)?;
        if current_version > crate::store::schema::SCHEMA_VERSION {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some(format!(
                    "store schema version {} is newer than this build supports (max {}); upgrade `rgt`",
                    current_version, crate::store::schema::SCHEMA_VERSION
                )),
            ));
        }
        initialize_schema(&conn)?;
        // FR-001: stamp or migrate the schema version.
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
