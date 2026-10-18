use crate::types::{
    exact_duration_seconds, DerivationEdge, NodeType, SourceDocument, TrackedNode, ValueData,
    ValueKind,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, types::Type, Connection, Result};
use uuid::Uuid;

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
    let dev_i64 = dev.map(|v| v as i64);
    let ino_i64 = ino.map(|v| v as i64);
    let inserted = if let (Some(d), Some(i)) = (dev_i64, ino_i64) {
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

    // Re-key any duplicate casing or alias row that now resolves to the same identity.
    if let (Some(d), Some(i)) = (dev_i64, ino_i64) {
        let surviving = get_source_document_by_path(conn, file_path)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?;
        // Update all tracked_nodes referencing any duplicate row to the surviving id BEFORE deleting duplicate rows!
        conn.execute(
            "UPDATE tracked_nodes SET source_doc_id = ?1
             WHERE source_doc_id IN (
                 SELECT id FROM source_documents
                 WHERE dev = ?2 AND ino = ?3 AND id <> ?1
             )",
            params![surviving.id, d, i],
        )?;
        conn.execute(
            "DELETE FROM source_documents
             WHERE dev = ?1 AND ino = ?2 AND id <> ?3",
            params![d, i, surviving.id],
        )?;
        return Ok(surviving);
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
            dev: row.get::<_, Option<i64>>(6)?.map(|v| v as u64),
            ino: row.get::<_, Option<i64>>(7)?.map(|v| v as u64),
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
        ValueData::Duration(duration) => {
            let seconds = exact_duration_seconds(duration).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    error,
                )))
            })?;
            (None, None, Some(seconds))
        }
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

/// Atomically inserts a new canonical Duration or reuses a complete matching
/// historical/canonical result. Unlike `insert_tracked_node`, this path never
/// overwrites a node whose identity already belongs to another value/lineage.
pub fn insert_or_reuse_duration(
    conn: &mut Connection,
    node: &TrackedNode,
    parent_ids: &[String],
    operation: &str,
    legacy_candidates: &[String],
) -> Result<String, String> {
    let seconds = match &node.value {
        ValueData::Duration(duration) => exact_duration_seconds(duration)?,
        _ => return Err("Duration insertion requires a Duration value".into()),
    };
    if node.value_kind != ValueKind::Duration || node.node_type != NodeType::Derived {
        return Err("Duration insertion requires a derived Duration node".into());
    }
    let canonical_id = TrackedNode::generate_duration_derived_id(parent_ids, operation, seconds)?;
    if node.id != canonical_id {
        return Err(format!(
            "Duration node ID '{}' does not match its canonical identity '{canonical_id}'",
            node.id
        ));
    }
    let mut expected_parents = parent_ids.to_vec();
    if matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        expected_parents.sort();
    }
    // Identity keeps DATE_DIFF's ordered operand list, including a repeated
    // Date ID. The graph stores one edge per distinct parent ID.
    let mut expected_edge_parents = Vec::with_capacity(expected_parents.len());
    for parent_id in &expected_parents {
        if !expected_edge_parents.contains(parent_id) {
            expected_edge_parents.push(parent_id.clone());
        }
    }

    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("failed to begin Duration transaction: {error}"))?;

    let mut tx_any_parent_stale = node.is_stale;
    for parent_id in parent_ids {
        if let Ok(Some(pnode)) = get_tracked_node(&tx, parent_id) {
            if pnode.is_stale {
                tx_any_parent_stale = true;
                break;
            }
        }
    }
    let mut node_to_insert = node.clone();
    node_to_insert.is_stale = tx_any_parent_stale;
    if tx_any_parent_stale && node_to_insert.stale_reason.is_none() {
        node_to_insert.stale_reason = Some("PARENT_NODE_STALE".to_string());
    }
    let node = &node_to_insert;

    for candidate in legacy_candidates {
        if candidate == &canonical_id {
            continue;
        }
        if let Some(existing) = get_tracked_node(&tx, candidate).map_err(|error| {
            format!("failed to inspect historical Duration '{candidate}': {error}")
        })? {
            if duration_lineage_matches(
                &tx,
                &existing,
                &node.value,
                operation,
                &expected_edge_parents,
            )? {
                if node.is_stale != existing.is_stale || node.stale_reason != existing.stale_reason
                {
                    let mut updated = existing.clone();
                    updated.is_stale = node.is_stale;
                    updated.stale_reason = node.stale_reason.clone();
                    updated.updated_at = node.updated_at;
                    insert_tracked_node(&tx, &updated).map_err(|error| {
                        format!("failed to update reused Duration '{candidate}': {error}")
                    })?;
                }
                tx.commit()
                    .map_err(|error| format!("failed to finish Duration reuse: {error}"))?;
                return Ok(candidate.clone());
            }
        }
    }

    if let Some(existing) = get_tracked_node(&tx, &canonical_id).map_err(|error| {
        format!("failed to inspect canonical Duration '{canonical_id}': {error}")
    })? {
        if duration_lineage_matches(
            &tx,
            &existing,
            &node.value,
            operation,
            &expected_edge_parents,
        )? {
            if node.is_stale != existing.is_stale || node.stale_reason != existing.stale_reason {
                insert_tracked_node(&tx, node).map_err(|error| {
                    format!("failed to update reused Duration '{canonical_id}': {error}")
                })?;
            }
            tx.commit()
                .map_err(|error| format!("failed to finish Duration reuse: {error}"))?;
            return Ok(canonical_id);
        }
        return Err(format!(
            "canonical Duration ID collision at '{canonical_id}': stored value or parent evidence conflicts; existing data was left untouched"
        ));
    }

    insert_tracked_node(&tx, node)
        .map_err(|error| format!("failed to insert Duration '{canonical_id}': {error}"))?;
    for parent_id in &expected_edge_parents {
        insert_derivation_edge(&tx, parent_id, &canonical_id, operation, None)?;
    }
    tx.commit()
        .map_err(|error| format!("failed to commit Duration derivation: {error}"))?;
    Ok(canonical_id)
}

fn duration_lineage_matches(
    conn: &Connection,
    existing: &TrackedNode,
    expected_value: &ValueData,
    operation: &str,
    expected_parents: &[String],
) -> Result<bool, String> {
    if existing.node_type != NodeType::Derived
        || existing.value_kind != ValueKind::Duration
        || &existing.value != expected_value
        || existing.source_doc_id.is_some()
        || existing.line_number.is_some()
    {
        return Ok(false);
    }
    let edges = get_parent_edges(conn, &existing.id)
        .map_err(|error| format!("failed to inspect Duration parent edges: {error}"))?;
    if edges.len() != expected_parents.len() {
        return Ok(false);
    }
    Ok(edges.iter().zip(expected_parents).all(|(edge, expected)| {
        edge.parent_node_id == *expected
            && edge.child_node_id == existing.id
            && edge.operation_type == operation
            && edge.expression.is_none()
    }))
}

/// Records that a known hook-capable coding agent captured a value. Repeated
/// observations by the same agent are idempotent; a different agent adds one
/// separate association. This is intended to run in the value insert's
/// transaction.
pub fn insert_capture_association(
    conn: &Connection,
    node_id: &str,
    agent_name: &str,
) -> Result<()> {
    if !crate::hooks::installer::is_capture_capable_agent(agent_name) {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "unsupported capture agent '{agent_name}'"
        )));
    }
    conn.execute(
        "INSERT INTO capture_associations (node_id, agent_name) VALUES (?1, ?2)
         ON CONFLICT(node_id, agent_name) DO NOTHING",
        params![node_id, agent_name],
    )?;
    Ok(())
}

/// Lists durable capture associations in stable `(node_id, agent_name)` order.
#[allow(dead_code)]
pub fn list_capture_associations(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement = conn.prepare(
        "SELECT node_id, agent_name FROM capture_associations ORDER BY node_id, agent_name",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

const SELECT_TRACKED_NODES_SQL: &str =
    "SELECT id, node_type, value_kind, number_val, date_val, duration_secs,
            source_doc_id, line_number, is_stale, stale_reason, created_at, updated_at
     FROM tracked_nodes";

pub fn row_to_tracked_node(row: &rusqlite::Row) -> Result<TrackedNode> {
    let node_type_str: String = row.get(1)?;
    let value_kind_str: String = row.get(2)?;
    let num_val: Option<f64> = row.get(3)?;
    let date_val: Option<String> = row.get(4)?;
    let dur_secs: Option<i64> = row.get(5)?;
    let is_stale_int: i32 = row.get(8)?;

    let value_kind = value_kind_str.parse().map_err(|error: String| {
        value_conversion_error(2, Type::Text, format!("invalid value_kind: {error}"))
    })?;
    let value = match value_kind {
        ValueKind::Number => {
            if date_val.is_some() || dur_secs.is_some() {
                return Err(value_conversion_error(
                    2,
                    Type::Text,
                    "number row has conflicting typed columns".into(),
                ));
            }
            let number = num_val.ok_or_else(|| {
                value_conversion_error(3, Type::Null, "number row is missing number_val".into())
            })?;
            if !number.is_finite() {
                return Err(value_conversion_error(
                    3,
                    Type::Real,
                    "number_val is not finite".into(),
                ));
            }
            ValueData::Number(number)
        }
        ValueKind::Date => {
            if num_val.is_some() || dur_secs.is_some() {
                return Err(value_conversion_error(
                    2,
                    Type::Text,
                    "date row has conflicting typed columns".into(),
                ));
            }
            let dt_str = date_val.ok_or_else(|| {
                value_conversion_error(4, Type::Null, "date row is missing date_val".into())
            })?;
            let dt = DateTime::parse_from_rfc3339(&dt_str)
                .map_err(|error| {
                    value_conversion_error(4, Type::Text, format!("date_val is invalid: {error}"))
                })?
                .with_timezone(&Utc);
            ValueData::Date(dt)
        }
        ValueKind::Duration => {
            if num_val.is_some() || date_val.is_some() {
                return Err(value_conversion_error(
                    2,
                    Type::Text,
                    "duration row has conflicting typed columns".into(),
                ));
            }
            let seconds = dur_secs.ok_or_else(|| {
                value_conversion_error(
                    5,
                    Type::Null,
                    "duration row is missing duration_secs".into(),
                )
            })?;
            let duration = Duration::try_seconds(seconds).ok_or_else(|| {
                value_conversion_error(
                    5,
                    Type::Integer,
                    format!("duration_secs '{seconds}' is out of range"),
                )
            })?;
            ValueData::Duration(duration)
        }
    };

    let created_at_str: String = row.get(10)?;
    let updated_at_str: String = row.get(11)?;

    Ok(TrackedNode {
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
    })
}

/// Fetches a tracked node by ID, if it exists.
pub fn get_tracked_node(conn: &Connection, id: &str) -> Result<Option<TrackedNode>> {
    let sql = format!("{SELECT_TRACKED_NODES_SQL} WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row_to_tracked_node(row)?))
    } else {
        Ok(None)
    }
}

fn value_conversion_error(index: usize, column_type: Type, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        column_type,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
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
         ON CONFLICT(parent_node_id, child_node_id) DO UPDATE SET
            operation_type = excluded.operation_type,
            expression = excluded.expression",
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
         FROM derivation_edges WHERE child_node_id = ?1 ORDER BY id",
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

/// Cascades staleness downstream from a set of starting parent node IDs via BFS.
pub fn cascade_invalidation_downstream(
    conn: &Connection,
    start_node_ids: &[String],
    reason: &str,
) -> Result<usize> {
    use std::collections::{HashSet, VecDeque};
    let mut visited: HashSet<String> = start_node_ids.iter().cloned().collect();
    let mut queue: VecDeque<String> = start_node_ids.iter().cloned().collect();
    let mut count = 0;

    let mut stmt =
        conn.prepare("SELECT child_node_id FROM derivation_edges WHERE parent_node_id = ?1")?;

    while let Some(parent_id) = queue.pop_front() {
        let mut rows = stmt.query(params![parent_id])?;
        while let Some(row) = rows.next()? {
            let child_id: String = row.get(0)?;
            if visited.insert(child_id.clone()) {
                mark_node_stale(conn, &child_id, reason)?;
                queue.push_back(child_id);
                count += 1;
            }
        }
    }
    Ok(count)
}

fn get_source_document_by_dev_ino(
    conn: &Connection,
    dev: u64,
    ino: u64,
) -> Result<Option<SourceDocument>> {
    if dev == 0 && ino == 0 {
        return Ok(None);
    }
    let mut stmt = conn.prepare(
        "SELECT id, file_path, mtime_nsec, file_size, blake3_hash, last_checked_at, dev, ino
         FROM source_documents WHERE dev = ?1 AND ino = ?2 LIMIT 1",
    )?;
    let mut rows = stmt.query(params![dev as i64, ino as i64])?;
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

/// Updates the line number of an existing tracked node in place.
pub fn update_node_line_number(
    conn: &Connection,
    id: &str,
    line_number: Option<u32>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tracked_nodes SET line_number = ?1, updated_at = ?2 WHERE id = ?3",
        params![line_number, now, id],
    )?;
    Ok(())
}

/// Atomically records a batch of extracted values for a file into the store,
/// performing invalidation of removed/modified root nodes and their downstream
/// derivations if the file content hash has changed.
pub fn record_file_batch(
    conn: &Connection,
    file_path: &str,
    meta: &crate::detection::MetadataSnapshot,
    hash: &str,
    extracted: &[crate::hooks::parser::ExtractedValue],
    agent_association: Option<&str>,
) -> Result<SourceDocument> {
    use std::collections::{HashMap, HashSet};

    // 1. Check if the file already exists (by path or by `(dev, ino)` alias) and whether the hash is changing.
    // Skip for `<stdin>` so consecutive `rgt record --stdin` calls accumulate without invalidating prior stdin values.
    let existing_doc = if file_path != "<stdin>" {
        match get_source_document_by_path(conn, file_path)? {
            Some(doc) => Some(doc),
            None => match (meta.dev, meta.ino) {
                (Some(dev), Some(ino)) => get_source_document_by_dev_ino(conn, dev, ino)?,
                _ => None,
            },
        }
    } else {
        None
    };

    let effective_file_path = if let Some(ref doc) = existing_doc {
        doc.file_path.as_str()
    } else {
        file_path
    };

    let mut matched_ext_node_ids: HashMap<usize, String> = HashMap::new();

    if file_path != "<stdin>" {
        if let Some(ref existing_doc) = existing_doc {
            if existing_doc.blake3_hash != hash {
                // Content modified! Reconcile line shifts and find root nodes that are no longer present.
                let existing_roots = list_nodes_by_source_doc(conn, existing_doc.id)?;
                let mut active_roots: Vec<TrackedNode> =
                    existing_roots.into_iter().filter(|r| !r.is_stale).collect();
                active_roots.sort_by(|a, b| {
                    a.line_number
                        .unwrap_or(0)
                        .cmp(&b.line_number.unwrap_or(0))
                        .then_with(|| a.id.cmp(&b.id))
                });

                let mut matched_root_ids: HashSet<String> = HashSet::new();

                // Pass 1: exact same-line matches.
                for (ext_idx, ext) in extracted.iter().enumerate() {
                    let expected_id = TrackedNode::generate_root_id(
                        effective_file_path,
                        ext.line_number,
                        ext.occurrence,
                        &ext.value,
                    );
                    if let Some(pos) = active_roots.iter().position(|r| {
                        !matched_root_ids.contains(&r.id)
                            && r.id == expected_id
                            && r.line_number == ext.line_number
                            && r.value == ext.value
                    }) {
                        matched_ext_node_ids.insert(ext_idx, active_roots[pos].id.clone());
                        matched_root_ids.insert(active_roots[pos].id.clone());
                    }
                }

                // Pass 2: pure line shifts for identical values.
                let mut remaining_roots_by_val: HashMap<
                    (crate::types::ValueKind, String),
                    Vec<TrackedNode>,
                > = HashMap::new();
                for r in &active_roots {
                    if !matched_root_ids.contains(&r.id) {
                        remaining_roots_by_val
                            .entry((r.value_kind.clone(), r.value.to_string_repr()))
                            .or_default()
                            .push(r.clone());
                    }
                }

                let mut remaining_ext_by_val: HashMap<
                    (crate::types::ValueKind, String),
                    Vec<(usize, &crate::hooks::parser::ExtractedValue)>,
                > = HashMap::new();
                for (ext_idx, ext) in extracted.iter().enumerate() {
                    if !matched_ext_node_ids.contains_key(&ext_idx) {
                        remaining_ext_by_val
                            .entry((ext.value.kind(), ext.value.to_string_repr()))
                            .or_default()
                            .push((ext_idx, ext));
                    }
                }

                for (key, roots) in remaining_roots_by_val {
                    if let Some(exts) = remaining_ext_by_val.get(&key) {
                        let count = roots.len().min(exts.len());
                        for i in 0..count {
                            let root = &roots[i];
                            let (ext_idx, ext) = exts[i];
                            update_node_line_number(conn, &root.id, ext.line_number)?;
                            matched_ext_node_ids.insert(ext_idx, root.id.clone());
                            matched_root_ids.insert(root.id.clone());
                        }
                    }
                }

                // Stale invalidation: only active roots that could not be matched are marked stale.
                let mut stale_root_ids = Vec::new();
                for root in active_roots {
                    if !matched_root_ids.contains(&root.id) {
                        mark_node_stale(conn, &root.id, "SOURCE_FILE_MODIFIED")?;
                        stale_root_ids.push(root.id);
                    }
                }

                if !stale_root_ids.is_empty() {
                    cascade_invalidation_downstream(conn, &stale_root_ids, "PARENT_NODE_STALE")?;
                }
            } else {
                for (i, ext) in extracted.iter().enumerate() {
                    let node_id = TrackedNode::generate_root_id(
                        effective_file_path,
                        ext.line_number,
                        ext.occurrence,
                        &ext.value,
                    );
                    matched_ext_node_ids.insert(i, node_id);
                }
            }
        }
    }

    // 2. Upsert source document with updated metadata & hash
    let doc = upsert_source_document(
        conn,
        file_path,
        meta.mtime_nsec,
        meta.file_size,
        hash,
        meta.dev,
        meta.ino,
    )?;

    // 3. Insert newly extracted root nodes
    let now = Utc::now();
    for (ext_idx, ext) in extracted.iter().enumerate() {
        if let Some(retained_id) = matched_ext_node_ids.get(&ext_idx) {
            if let Some(agent) = agent_association {
                insert_capture_association(conn, retained_id, agent)?;
            }
            continue;
        }

        let node_id = TrackedNode::generate_root_id(
            effective_file_path,
            ext.line_number,
            ext.occurrence,
            &ext.value,
        );
        let node = TrackedNode {
            id: node_id.clone(),
            node_type: NodeType::Root,
            value_kind: ext.value.kind(),
            value: ext.value.clone(),
            source_doc_id: Some(doc.id),
            line_number: ext.line_number,
            is_stale: false,
            stale_reason: None,
            created_at: now,
            updated_at: now,
        };
        insert_tracked_node(conn, &node)?;
        if let Some(agent) = agent_association {
            insert_capture_association(conn, &node_id, agent)?;
        }
    }

    Ok(doc)
}

/// Lists all nodes currently marked stale.
pub fn list_stale_nodes(conn: &Connection) -> Result<Vec<TrackedNode>> {
    let sql = format!("{SELECT_TRACKED_NODES_SQL} WHERE is_stale = 1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_tracked_node)?;
    rows.collect()
}

/// Lists all tracked nodes in the graph.
pub fn list_all_nodes(conn: &Connection) -> Result<Vec<TrackedNode>> {
    let mut stmt = conn.prepare(SELECT_TRACKED_NODES_SQL)?;
    let rows = stmt.query_map([], row_to_tracked_node)?;
    rows.collect()
}

/// Lists all nodes that were read from a given source document.
pub fn list_nodes_by_source_doc(conn: &Connection, source_doc_id: i64) -> Result<Vec<TrackedNode>> {
    let sql = format!("{SELECT_TRACKED_NODES_SQL} WHERE source_doc_id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![source_doc_id], row_to_tracked_node)?;
    rows.collect()
}

/// Reads a per-project setting (key/value), e.g. `number_format` (FR-003).
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM project_settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

/// Writes a per-project setting (upsert by key), e.g. `number_format` (FR-003).
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO project_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Returns the immutable, per-store RDF project token, creating it once on
/// first use. Concurrent openers race through `ON CONFLICT DO NOTHING`, then
/// all read the same committed value.
pub fn get_or_create_rdf_project_id(conn: &Connection) -> Result<String> {
    if let Some(project_id) = get_setting(conn, "rdf_project_id")? {
        if project_id.len() != 32
            || !project_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
                Some("invalid rdf_project_id: expected 32 lowercase hexadecimal characters".into()),
            ));
        }
        return Ok(project_id);
    }

    let candidate = Uuid::new_v4().simple().to_string();
    conn.execute(
        "INSERT INTO project_settings (key, value) VALUES ('rdf_project_id', ?1)
         ON CONFLICT(key) DO NOTHING",
        params![candidate],
    )?;
    let project_id =
        get_setting(conn, "rdf_project_id")?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    if project_id.len() != 32
        || !project_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
            Some("invalid rdf_project_id: expected 32 lowercase hexadecimal characters".into()),
        ));
    }
    Ok(project_id)
}

/// Deletes obsolete nodes — stale nodes with no derived dependents (not a
/// parent in any `derivation_edges` row) — and returns the number deleted
/// (FR-003). The caller owns the transaction; deleting a node cascades to its
/// own `derivation_edges` rows via `ON DELETE CASCADE`.
pub fn delete_obsolete_nodes(conn: &Connection) -> Result<usize> {
    let count = conn.execute(
        "DELETE FROM tracked_nodes
         WHERE is_stale = 1
           AND id NOT IN (SELECT parent_node_id FROM derivation_edges)",
        [],
    )?;
    Ok(count)
}
