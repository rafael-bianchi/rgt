use crate::graph::engine::GraphEngine;
use crate::store::queries::{
    get_source_document_by_path, list_nodes_by_source_doc, mark_node_stale, touch_source_document,
    upsert_source_document,
};
use crate::store::DbStore;
use chrono::{DateTime, Utc};
use rusqlite::Result;
use std::collections::HashSet;
use std::path::Path;

pub struct InvalidationCascade {
    engine: GraphEngine,
}

impl InvalidationCascade {
    pub fn build_from_db(db: &DbStore) -> Result<Self> {
        let conn = db.conn();
        let mut engine = GraphEngine::new();

        // 1. Bulk query all nodes
        let mut node_stmt = conn.prepare("SELECT id FROM tracked_nodes")?;
        let node_rows = node_stmt.query_map([], |row| row.get::<_, String>(0))?;
        for id in node_rows {
            engine.get_or_add_node(&id?);
        }

        // 2. Bulk query all derivation edges
        let mut edge_stmt = conn.prepare(
            "SELECT parent_node_id, child_node_id, operation_type FROM derivation_edges",
        )?;
        let edge_rows = edge_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for edge in edge_rows {
            let (parent_id, child_id, op) = edge?;
            engine.add_derivation_edge_unchecked(&parent_id, &child_id, &op);
        }

        // 3. Single-pass acyclicity check
        if engine.is_cyclic() {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(19),
                Some("cycle in persisted derivation graph".into()),
            ));
        }

        Ok(Self { engine })
    }

    pub fn invalidate_file_conn<P: AsRef<Path>>(
        &mut self,
        conn: &rusqlite::Connection,
        file_path: P,
        stale_reason: &str,
        invalidated: &mut HashSet<String>,
    ) -> Result<usize> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();

        let doc = match get_source_document_by_path(conn, &path_str)? {
            Some(d) => d,
            None => return Ok(0),
        };

        let root_nodes = list_nodes_by_source_doc(conn, doc.id)?;
        let mut count = 0;

        for root in root_nodes {
            if invalidated.insert(root.id.clone()) {
                mark_node_stale(conn, &root.id, stale_reason)?;
                count += 1;
            }

            let downstream = self.engine.get_downstream_dependents(&root.id);
            for child_id in downstream {
                if invalidated.insert(child_id.clone()) {
                    mark_node_stale(conn, &child_id, "PARENT_NODE_STALE")?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    pub fn invalidate_file<P: AsRef<Path>>(
        &mut self,
        db: &DbStore,
        file_path: P,
        stale_reason: &str,
    ) -> Result<usize> {
        let mut invalidated = HashSet::new();
        self.invalidate_file_conn(db.conn(), file_path, stale_reason, &mut invalidated)
    }

    pub fn evaluate_and_invalidate_all<P: AsRef<Path>>(
        &mut self,
        db: &mut DbStore,
        project_root: P,
    ) -> Result<usize> {
        self.evaluate_and_invalidate_all_with_deadline(db, project_root, None)
    }

    pub fn evaluate_and_invalidate_all_with_deadline<P: AsRef<Path>>(
        &mut self,
        db: &mut DbStore,
        project_root: P,
        deadline: Option<std::time::Instant>,
    ) -> Result<usize> {
        let collected_rows = {
            let conn = db.conn();
            let mut stmt = conn.prepare(
                "SELECT id, file_path, mtime_nsec, file_size, blake3_hash, dev, ino, last_checked_at
                 FROM source_documents",
            )?;
            let rows = stmt.query_map([], |row| {
                let id: i64 = row.get(0)?;
                let rel_path: String = row.get(1)?;
                let mtime_nsec: i64 = row.get(2)?;
                let file_size: u64 = row.get(3)?;
                let hash: String = row.get(4)?;
                let dev: Option<u64> = row.get::<_, Option<i64>>(5)?.map(|v| v as u64);
                let ino: Option<u64> = row.get::<_, Option<i64>>(6)?.map(|v| v as u64);
                let last_checked: String = row.get(7)?;
                Ok((
                    id,
                    rel_path,
                    mtime_nsec,
                    file_size,
                    hash,
                    dev,
                    ino,
                    last_checked,
                ))
            })?;

            let mut collected = Vec::new();
            for r in rows {
                collected.push(r?);
            }
            collected
        };

        let mut changes = Vec::new();
        for (
            doc_id,
            rel_path,
            stored_mtime,
            stored_size,
            stored_hash,
            stored_dev,
            stored_ino,
            last_checked_str,
        ) in collected_rows
        {
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(1),
                        Some("sync_staleness deadline exceeded".into()),
                    ));
                }
            }
            // Virtual sources (<stdin>) have no on-disk file to evaluate for changes
            if rel_path == "<stdin>" {
                continue;
            }
            let full_path = project_root.as_ref().join(&rel_path);

            let last_checked = DateTime::parse_from_rfc3339(&last_checked_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            // FR-001: force a full hash when the file hasn't been fully checked
            // within the threshold, so a same-mtime/same-size rewrite is caught.
            let force_hash = Utc::now() - last_checked > crate::detection::REHASH_THRESHOLD;

            let detection = crate::detection::evaluate_file_change(
                &full_path,
                stored_mtime,
                stored_size,
                &stored_hash,
                stored_dev,
                stored_ino,
                force_hash,
            );
            changes.push((doc_id, rel_path, stored_hash, detection));
        }

        let tx = db
            .conn_mut()
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let mut invalidated: HashSet<String> = HashSet::new();

        for (doc_id, rel_path, stored_hash, detection) in changes {
            if let Some(dl) = deadline {
                if std::time::Instant::now() >= dl {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(1),
                        Some("sync_staleness deadline exceeded".into()),
                    ));
                }
            }
            match detection {
                crate::detection::DetectionResult::Unchanged => {
                    // Tier 1 matched and force_hash was false: Tier 2 hash was not run.
                    // Preserve last_checked_at so REHASH_THRESHOLD will eventually trigger.
                }
                crate::detection::DetectionResult::VerifiedUnchanged {
                    new_mtime_nsec,
                    new_file_size,
                    new_dev,
                    new_ino,
                } => {
                    // Tier 2 hash verified content is identical: update last_checked_at
                    // and synchronize any updated mtime/size/dev/ino in source_documents.
                    upsert_source_document(
                        &tx,
                        &rel_path,
                        new_mtime_nsec,
                        new_file_size,
                        &stored_hash,
                        new_dev,
                        new_ino,
                    )?;
                }
                crate::detection::DetectionResult::Changed {
                    new_mtime_nsec: _,
                    new_file_size: _,
                    new_blake3_hash: _,
                    new_dev: _,
                    new_ino: _,
                } => {
                    // Verify inside the IMMEDIATE transaction that no concurrent `record_file_batch`
                    // has already updated `source_documents.blake3_hash` since our pre-lock snapshot.
                    if let Some(current_doc) = get_source_document_by_path(&tx, &rel_path)? {
                        if current_doc.blake3_hash == stored_hash {
                            self.invalidate_file_conn(
                                &tx,
                                &rel_path,
                                "SOURCE_FILE_MODIFIED",
                                &mut invalidated,
                            )?;
                        }
                    }
                }
                crate::detection::DetectionResult::FileNotFound => {
                    self.invalidate_file_conn(&tx, &rel_path, "FILE_DELETED", &mut invalidated)?;
                }
                // FR-003: a permission/lock error is NOT a deletion; treat it as
                // "state unchanged" (update the checked timestamp, no stale marking).
                crate::detection::DetectionResult::CheckFailed => {
                    touch_source_document(&tx, doc_id)?;
                }
            }
        }

        tx.commit()?;
        Ok(invalidated.len())
    }
}

/// Convenience helper to build the cascade from the database and evaluate/invalidate
/// all files in the project. Finds the project root directory or falls back to ".".
pub fn sync_staleness(db: &mut DbStore) -> Result<usize> {
    sync_staleness_with_deadline(db, None)
}

pub fn sync_staleness_with_deadline(
    db: &mut DbStore,
    deadline: Option<std::time::Instant>,
) -> Result<usize> {
    let mut cascade = InvalidationCascade::build_from_db(db)?;
    let root = DbStore::find_project_root(".").unwrap_or_else(|| std::path::PathBuf::from("."));
    cascade.evaluate_and_invalidate_all_with_deadline(db, root, deadline)
}
