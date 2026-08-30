pub mod editor;
pub mod glue;
pub mod installer;
pub mod parser;
pub mod paths;

use crate::detection::{
    compute_blake3_hash, compute_blake3_hash_from_bytes, get_metadata_snapshot,
};
use crate::hooks::parser::{extract_values_from_content, normalize_agent_event, NumberFormat};
use crate::store::queries::{get_setting, insert_tracked_node, upsert_source_document};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode};
use chrono::Utc;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Processes a passive hook event (PreToolUse/PostToolUse) by reading tool event
/// JSON from stdin, normalizing it to the internal `{path, content}` model
/// (per the `--agent` dialect), and recording numeric/date values to the
/// provenance graph. When the event carries no content but a valid path, the
/// file is read from disk as a fallback. Fails open (never blocks the agent):
/// every error path returns `Ok(())` with no stdout.
pub fn handle_passive_hook_event(_event_type: &str, agent: Option<&str>) -> io::Result<()> {
    let mut stdin_str = String::new();
    io::stdin().read_to_string(&mut stdin_str)?;

    let capture = match normalize_agent_event(agent, &stdin_str) {
        Some(c) => c,
        None => return Ok(()), // Fail-open non-blocking
    };

    let Some(path) = capture.path.as_deref() else {
        return Ok(()); // Nothing capturable
    };

    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Ok(());
    }

    let meta = match get_metadata_snapshot(path_obj) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    // FR-002: the fingerprint must hash the same bytes that are extracted. When
    // content comes from the agent's `tool_response`, the file is hashed (the
    // correct staleness identity); when content is read from disk (fallback),
    // the exact bytes read are hashed (no second read → no TOCTOU).
    let (content, hash) = match capture.content.as_deref() {
        Some(c) => {
            let hash = match compute_blake3_hash(path_obj) {
                Ok(h) => h,
                Err(_) => return Ok(()),
            };
            (c.to_string(), hash)
        }
        None => {
            let bytes = match fs::read(path_obj) {
                Ok(b) => b,
                Err(_) => return Ok(()), // Fail-open non-blocking
            };
            let hash = compute_blake3_hash_from_bytes(&bytes);
            (String::from_utf8_lossy(&bytes).into_owned(), hash)
        }
    };

    let db = match DbStore::open_in_project(".") {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };

    // FR-003: the passive path uses the persisted format (no flag available).
    let number_format = match get_setting(db.conn(), "number_format") {
        Ok(Some(raw)) => raw.parse().unwrap_or(NumberFormat::Auto),
        _ => NumberFormat::Auto,
    };

    let doc = match upsert_source_document(
        db.conn(),
        path,
        meta.mtime_nsec,
        meta.file_size,
        &hash,
        meta.dev,
        meta.ino,
    ) {
        Ok(d) => d,
        Err(_) => return Ok(()),
    };

    let extraction = extract_values_from_content(&content, number_format);
    let now = Utc::now();

    // FR-007: record the batch in a single transaction (all-or-nothing). On any
    // error, the transaction is dropped (rolled back) and the hook still fails
    // open — never blocks or partially records.
    let tx = match db.conn().unchecked_transaction() {
        Ok(tx) => tx,
        Err(_) => return Ok(()),
    };

    for ext in extraction.values {
        let node_id =
            TrackedNode::generate_root_id(path, ext.line_number, ext.occurrence, &ext.value);
        let node = TrackedNode {
            id: node_id,
            node_type: NodeType::Root,
            value_kind: ext.value.kind(),
            value: ext.value,
            source_doc_id: Some(doc.id),
            line_number: ext.line_number,
            is_stale: false,
            stale_reason: None,
            created_at: now,
            updated_at: now,
        };

        if insert_tracked_node(&tx, &node).is_err() {
            return Ok(()); // rollback on drop; fail open
        }
    }

    if tx.commit().is_err() {
        return Ok(());
    }

    Ok(())
}
