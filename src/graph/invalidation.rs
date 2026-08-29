use crate::graph::engine::GraphEngine;
use crate::store::queries::{
    get_child_edges, get_source_document_by_path, list_all_nodes, list_nodes_by_source_doc,
    mark_node_stale, touch_source_document, upsert_source_document,
};
use crate::store::DbStore;
use chrono::{DateTime, Utc};
use rusqlite::Result;
use std::path::Path;

pub struct InvalidationCascade {
    engine: GraphEngine,
}

impl InvalidationCascade {
    pub fn build_from_db(db: &DbStore) -> Result<Self> {
        let conn = db.conn();
        let mut engine = GraphEngine::new();

        let all_nodes = list_all_nodes(conn)?;
        for node in &all_nodes {
            engine.get_or_add_node(&node.id);
        }

        for node in &all_nodes {
            let edges = get_child_edges(conn, &node.id)?;
            for edge in edges {
                // FR-002: surface a cycle encountered during reload rather than
                // silently dropping the edge (no `let _ =`).
                engine
                    .add_derivation_edge(
                        &edge.parent_node_id,
                        &edge.child_node_id,
                        &edge.operation_type,
                    )
                    .map_err(|e| {
                        rusqlite::Error::SqliteFailure(
                            rusqlite::ffi::Error::new(19),
                            Some(format!("cycle in persisted derivation graph: {}", e)),
                        )
                    })?;
            }
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
        let mut total_invalidated = 0;

        for root in root_nodes {
            mark_node_stale(conn, &root.id, stale_reason)?;
            total_invalidated += 1;

            let downstream = self.engine.get_downstream_dependents(&root.id);
            for child_id in downstream {
                mark_node_stale(conn, &child_id, "PARENT_NODE_STALE")?;
                total_invalidated += 1;
            }
        }

        Ok(total_invalidated)
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
                    touch_source_document(conn, doc_id)?;
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
