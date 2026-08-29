use crate::types::{DerivationEdge, NodeType, SourceDocument, TrackedNode, ValueData, ValueKind};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, Result};

/// Inserts or updates a source document row by file path (or, when identity is
/// available, by dev/ino). Returns the upserted document.
pub fn upsert_source_document(
    conn: &Connection,
    file_path: &str,
    mtime_nsec: i64,
    file_size: u64,
    blake3_hash: &str,
    dev: Option<u64>,
    ino: Option<u64>,
) -> Result<SourceDocument> {
    let now = Utc::now().to_rfc3339();

    // FR-005: dedupe on on-disk identity (dev/ino) when both are present, so the
    // same file reached via different casings is one row. Fall back to the path.
    let inserted = if let (Some(d), Some(i)) = (dev, ino) {
        conn.execute(
            "INSERT INTO source_documents (file_path, mtime_nsec, file_size, blake3_hash, last_checked_at, dev, ino)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(file_path) DO UPDATE SET
                mtime_nsec = excluded.mtime_nsec,
                file_size = excluded.file_size,
                blake3_hash = excluded.blake3_hash,
                last_checked_at = excluded.last_checked_at,
                dev = excluded.dev,
                ino = excluded.ino",
            params![file_path, mtime_nsec, file_size, blake3_hash, now, d, i],
        )
    } else {
        conn.execute(
            "INSERT INTO source_documents (file_path, mtime_nsec, file_size, blake3_hash, last_checked_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(file_path) DO UPDATE SET
                mtime_nsec = excluded.mtime_nsec,
                file_size = excluded.file_size,
                blake3_hash = excluded.blake3_hash,
                last_checked_at = excluded.last_checked_at",
            params![file_path, mtime_nsec, file_size, blake3_hash, now],
        )
    };
    inserted?;

    // Re-key any duplicate casing row that now resolves to the same identity.
    if let (Some(d), Some(i)) = (dev, ino) {
        conn.execute(
            "DELETE FROM source_documents
             WHERE dev = ?1 AND ino = ?2 AND file_path <> ?3",
            params![d, i, file_path],
        )?;
    }

    get_source_document_by_path(conn, file_path)?
        .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
}

/// Updates a source document's `last_checked_at` to now (after a change check).
pub fn touch_source_document(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE source_documents SET last_checked_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), id],
    )?;
    Ok(())
}

/// Fetches a source document by file path.
pub fn get_source_document_by_path(
    conn: &Connection,
    file_path: &str,
) -> Result<Option<SourceDocument>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_path, mtime_nsec, file_size, blake3_hash, last_checked_at, dev, ino
         FROM source_documents WHERE file_path = ?1",
    )?;

    let mut rows = stmt.query(params![file_path])?;
    if let Some(row) = rows.next()? {
        let last_checked_str: String = row.get(5)?;
        let last_checked_at = DateTime::parse_from_rfc3339(&last_checked_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(Some(SourceDocument {
            id: row.get(0)?,
            file_path: row.get(1)?,
            mtime_nsec: row.get(2)?,
            file_size: row.get(3)?,
            blake3_hash: row.get(4)?,
            last_checked_at,
            dev: row.get(6)?,
            ino: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

/// Inserts a tracked node (root or derived) via idempotent upsert by node ID.
pub fn insert_tracked_node(conn: &Connection, node: &TrackedNode) -> Result<()> {
    let (num_val, date_val, dur_val) = match &node.value {
        ValueData::Number(n) => (Some(*n), None, None),
        ValueData::Date(d) => (None, Some(d.to_rfc3339()), None),
        ValueData::Duration(dur) => (None, None, Some(dur.num_seconds())),
    };

    conn.execute(
        "INSERT INTO tracked_nodes (
            id, node_type, value_kind, number_val, date_val, duration_secs,
            source_doc_id, line_number, is_stale, stale_reason, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        ON CONFLICT(id) DO UPDATE SET
            value_kind = excluded.value_kind,
            number_val = excluded.number_val,
            date_val = excluded.date_val,
            duration_secs = excluded.duration_secs,
            source_doc_id = excluded.source_doc_id,
            line_number = excluded.line_number,
            is_stale = excluded.is_stale,
            stale_reason = excluded.stale_reason,
            updated_at = excluded.updated_at",
        params![
            node.id,
            node.node_type.to_string(),
            node.value_kind.to_string(),
            num_val,
            date_val,
            dur_val,
            node.source_doc_id,
            node.line_number,
            if node.is_stale { 1 } else { 0 },
            node.stale_reason,
            node.created_at.to_rfc3339(),
            node.updated_at.to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Fetches a tracked node by ID, if it exists.
pub fn get_tracked_node(conn: &Connection, id: &str) -> Result<Option<TrackedNode>> {
    let mut stmt = conn.prepare(
        "SELECT id, node_type, value_kind, number_val, date_val, duration_secs,
                source_doc_id, line_number, is_stale, stale_reason, created_at, updated_at
         FROM tracked_nodes WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        let node_type_str: String = row.get(1)?;
        let value_kind_str: String = row.get(2)?;
        let num_val: Option<f64> = row.get(3)?;
        let date_val: Option<String> = row.get(4)?;
        let dur_secs: Option<i64> = row.get(5)?;
        let is_stale_int: i32 = row.get(8)?;

        let value_kind = value_kind_str.parse().unwrap_or(ValueKind::Number);
        let value = match value_kind {
            ValueKind::Number => ValueData::Number(num_val.unwrap_or(0.0)),
            ValueKind::Date => {
                let dt_str = date_val.unwrap_or_default();
                let dt = DateTime::parse_from_rfc3339(&dt_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                ValueData::Date(dt)
            }
            ValueKind::Duration => ValueData::Duration(Duration::seconds(dur_secs.unwrap_or(0))),
        };

        let created_at_str: String = row.get(10)?;
        let updated_at_str: String = row.get(11)?;

        Ok(Some(TrackedNode {
            id: row.get(0)?,
            node_type: node_type_str.parse().unwrap_or(NodeType::Root),
            value_kind,
            value,
            source_doc_id: row.get(6)?,
            line_number: row.get(7)?,
            is_stale: is_stale_int != 0,
            stale_reason: row.get(9)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }))
    } else {
        Ok(None)
    }
}

/// Inserts a derivation edge between a parent node and a derived node.
///
/// Rejects (before writing) any edge that would create a cycle — including a
/// self-loop — so the graph stays acyclic at the persistent write path, not
/// only in the transient in-memory graph (FR-001).
pub fn insert_derivation_edge(
    conn: &Connection,
    parent_id: &str,
    child_id: &str,
    operation_type: &str,
    expression: Option<&str>,
) -> Result<(), String> {
    if parent_id == child_id {
        return Err(format!(
            "refusing to insert a self-loop derivation edge (parent == child == {})",
            parent_id
        ));
    }

    // A new edge `parent -> child` would close a cycle iff `parent` is already
    // reachable from `child`. Check via a recursive CTE (UNION, so the
    // recursion terminates even on a corrupted cyclic DB).
    let mut stmt = conn
        .prepare(
            "WITH RECURSIVE reachable(id) AS (
                 SELECT child_node_id FROM derivation_edges WHERE parent_node_id = ?1
                 UNION
                 SELECT e.child_node_id
                   FROM derivation_edges e
                   JOIN reachable r ON e.parent_node_id = r.id
             )
             SELECT 1 FROM reachable WHERE id = ?2 LIMIT 1",
        )
        .map_err(|e| format!("failed to prepare cycle check: {e}"))?;
    let mut rows = stmt
        .query(params![child_id, parent_id])
        .map_err(|e| format!("failed to run cycle check: {e}"))?;
    if rows
        .next()
        .map_err(|e| format!("failed to read cycle check: {e}"))?
        .is_some()
    {
        return Err(format!(
            "refusing to insert derivation edge {} -> {}: it would create a cycle",
            parent_id, child_id
        ));
    }

    conn.execute(
        "INSERT INTO derivation_edges (parent_node_id, child_node_id, operation_type, expression)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(parent_node_id, child_node_id) DO NOTHING",
        params![parent_id, child_id, operation_type, expression],
    )
    .map_err(|e| format!("failed to insert derivation edge: {e}"))?;
    Ok(())
}

/// Lists the direct children (dependent nodes) of a parent node.
pub fn get_child_edges(conn: &Connection, parent_id: &str) -> Result<Vec<DerivationEdge>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_node_id, child_node_id, operation_type, expression
         FROM derivation_edges WHERE parent_node_id = ?1",
    )?;

    let rows = stmt.query_map(params![parent_id], |row| {
        Ok(DerivationEdge {
            id: row.get(0)?,
            parent_node_id: row.get(1)?,
            child_node_id: row.get(2)?,
            operation_type: row.get(3)?,
            expression: row.get(4)?,
        })
    })?;

    let mut edges = Vec::new();
    for edge in rows {
        edges.push(edge?);
    }
    Ok(edges)
}

/// Lists the direct parents (dependencies) of a derived node.
pub fn get_parent_edges(conn: &Connection, child_id: &str) -> Result<Vec<DerivationEdge>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_node_id, child_node_id, operation_type, expression
         FROM derivation_edges WHERE child_node_id = ?1",
    )?;

    let rows = stmt.query_map(params![child_id], |row| {
        Ok(DerivationEdge {
            id: row.get(0)?,
            parent_node_id: row.get(1)?,
            child_node_id: row.get(2)?,
            operation_type: row.get(3)?,
            expression: row.get(4)?,
        })
    })?;

    let mut edges = Vec::new();
    for edge in rows {
        edges.push(edge?);
    }
    Ok(edges)
}

/// Marks a node as stale with the given reason.
pub fn mark_node_stale(conn: &Connection, node_id: &str, reason: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tracked_nodes SET is_stale = 1, stale_reason = ?1, updated_at = ?2 WHERE id = ?3",
        params![reason, now, node_id],
    )?;
    Ok(())
}

/// Lists all nodes currently marked stale.
pub fn list_stale_nodes(conn: &Connection) -> Result<Vec<TrackedNode>> {
    let mut stmt = conn.prepare("SELECT id FROM tracked_nodes WHERE is_stale = 1")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        Ok(id)
    })?;

    let mut nodes = Vec::new();
    for id_res in rows {
        let id = id_res?;
        if let Some(node) = get_tracked_node(conn, &id)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}

/// Lists all tracked nodes in the graph.
pub fn list_all_nodes(conn: &Connection) -> Result<Vec<TrackedNode>> {
    let mut stmt = conn.prepare("SELECT id FROM tracked_nodes")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        Ok(id)
    })?;

    let mut nodes = Vec::new();
    for id_res in rows {
        let id = id_res?;
        if let Some(node) = get_tracked_node(conn, &id)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}

/// Lists all nodes that were read from a given source document.
pub fn list_nodes_by_source_doc(conn: &Connection, source_doc_id: i64) -> Result<Vec<TrackedNode>> {
    let mut stmt = conn.prepare("SELECT id FROM tracked_nodes WHERE source_doc_id = ?1")?;
    let rows = stmt.query_map(params![source_doc_id], |row| {
        let id: String = row.get(0)?;
        Ok(id)
    })?;

    let mut nodes = Vec::new();
    for id_res in rows {
        let id = id_res?;
        if let Some(node) = get_tracked_node(conn, &id)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}
