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

    pub fn invalidate_file<P: AsRef<Path>>(
        &mut self,
        db: &DbStore,
        file_path: P,
        stale_reason: &str,
    ) -> Result<usize> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let conn = db.conn();

        let doc = match get_source_document_by_path(conn, &path_str)? {
            Some(d) => d,
            None => return Ok(0),
        };

        let root_nodes = list_nodes_by_source_doc(conn, doc.id)?;
        // FR-005: count each distinct node exactly once — a downstream node
        // reachable from multiple stale roots must not inflate the count.
        let mut invalidated: HashSet<String> = HashSet::new();

        for root in root_nodes {
            invalidated.insert(root.id.clone());
            mark_node_stale(conn, &root.id, stale_reason)?;

            let downstream = self.engine.get_downstream_dependents(&root.id);
            for child_id in downstream {
                invalidated.insert(child_id.clone());
                mark_node_stale(conn, &child_id, "PARENT_NODE_STALE")?;
            }
        }

        Ok(invalidated.len())
    }

    pub fn evaluate_and_invalidate_all<P: AsRef<Path>>(
        &mut self,
        db: &DbStore,
        project_root: P,
    ) -> Result<usize> {
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
            let dev: Option<u64> = row.get(5)?;
            let ino: Option<u64> = row.get(6)?;
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

        let mut invalidations = 0;
        for row in rows {
            let (
                doc_id,
                rel_path,
                stored_mtime,
                stored_size,
                stored_hash,
                stored_dev,
                stored_ino,
                last_checked_str,
            ) = row?;
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

            match crate::detection::evaluate_file_change(
                &full_path,
                stored_mtime,
                stored_size,
                &stored_hash,
                stored_dev,
                stored_ino,
                force_hash,
            ) {
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
                        conn,
                        &rel_path,
                        new_mtime_nsec,
                        new_file_size,
                        &stored_hash,
                        new_dev,
                        new_ino,
                    )?;
                }
                crate::detection::DetectionResult::Changed {
                    new_mtime_nsec,
                    new_file_size,
                    new_blake3_hash,
                    new_dev,
                    new_ino,
                } => {
                    upsert_source_document(
                        conn,
                        &rel_path,
                        new_mtime_nsec,
                        new_file_size,
                        &new_blake3_hash,
                        new_dev,
                        new_ino,
                    )?;
                    invalidations += self.invalidate_file(db, &rel_path, "SOURCE_FILE_MODIFIED")?;
                }
                crate::detection::DetectionResult::FileNotFound => {
                    invalidations += self.invalidate_file(db, &rel_path, "FILE_DELETED")?;
                }
                // FR-003: a permission/lock error is NOT a deletion; treat it as
                // "state unchanged" (update the checked timestamp, no stale marking).
                crate::detection::DetectionResult::CheckFailed => {
                    touch_source_document(conn, doc_id)?;
                }
            }
        }

        Ok(invalidations)
    }
}

/// Convenience helper to build the cascade from the database and evaluate/invalidate
/// all files in the project. Finds the project root directory or falls back to ".".
pub fn sync_staleness(db: &DbStore) -> Result<usize> {
    let mut cascade = InvalidationCascade::build_from_db(db)?;
    let root = DbStore::find_project_root(".").unwrap_or_else(|| std::path::PathBuf::from("."));
    cascade.evaluate_and_invalidate_all(db, root)
}
