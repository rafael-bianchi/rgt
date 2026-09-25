pub mod editor;
pub mod glue;
pub mod installer;
pub mod parser;
pub mod paths;

use crate::detection::{
    compute_blake3_hash, compute_blake3_hash_from_bytes, get_metadata_snapshot,
};
use crate::extract::pdf::{decode_pdf_base64, extract_pdf_text, render_additional_context};
use crate::hooks::parser::{extract_values_from_content, normalize_agent_event, NumberFormat};
use crate::store::queries::{
    get_setting, insert_capture_association, insert_tracked_node, upsert_source_document,
};
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
/// file is read from disk as a fallback. For Claude Code native PDF reads, the
/// base64 envelope is decoded and extracted locally (spec 033). Fails open
/// (never blocks the agent): every error path returns `Ok(())` with no stdout,
/// except the Claude Code `additionalContext` injection which is additive.
pub fn handle_passive_hook_event(event_type: &str, agent: Option<&str>) -> io::Result<()> {
    let canonical_agent = match agent {
        None => Some("claude-code".to_string()),
        Some(name) => crate::hooks::installer::resolve_agent_name(name),
    };
    let Some(canonical_agent) = canonical_agent else {
        return Ok(()); // Unknown dialects remain fail-open and carry no attribution.
    };
    if !crate::hooks::installer::is_capture_capable_agent(&canonical_agent) {
        return Ok(()); // Rules-only agents have no event capture path.
    }

    let mut stdin_str = String::new();
    io::stdin().read_to_string(&mut stdin_str)?;

    let capture = match normalize_agent_event(Some(&canonical_agent), &stdin_str) {
        Some(c) => c,
        None => return Ok(()), // Fail-open non-blocking
    };

    // PDF envelope: decode, extract locally, record, and (claude-code only)
    // inject the extracted text back via PostToolUse additionalContext.
    if let Some(base64_str) = capture.pdf_base64.as_deref() {
        return handle_pdf_capture(event_type, Some(&canonical_agent), &capture, base64_str);
    }

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
        if insert_capture_association(&tx, &node.id, &canonical_agent).is_err() {
            return Ok(()); // value and attribution roll back together
        }
    }

    if tx.commit().is_err() {
        return Ok(());
    }

    Ok(())
}

/// Handles a Claude Code PDF envelope capture (spec 033): base64-decode, run
/// local text extraction, record the extracted values into the provenance
/// graph (agent-agnostic), and for `--agent claude-code` inject the extracted
/// text back via `PostToolUse` `additionalContext` on stdout. Every error path
/// fails open: no crash, no non-zero exit, no stdout except the additive
/// claude-code injection.
fn handle_pdf_capture(
    _event_type: &str,
    agent: Option<&str>,
    capture: &crate::hooks::parser::NormalizedCapture,
    base64_str: &str,
) -> io::Result<()> {
    let decoded = match decode_pdf_base64(base64_str) {
        Some(d) => d,
        None => return Ok(()), // fail open: malformed base64
    };
    let extracted = match extract_pdf_text(&decoded) {
        Some(t) => t,
        None => return Ok(()), // fail open: extraction panicked
    };

    let is_claude = matches!(agent, None | Some("claude-code"));

    // Recording half is agent-agnostic (FR-002/FR-007): record any values even
    // when stdout injection is unsupported or the text is empty.
    let path = capture.path.as_deref();
    let recorded = path
        .map(|p| record_pdf_values(p, &extracted, agent))
        .transpose()?
        .unwrap_or(false);

    // Injection half is claude-code-only (FR-003/FR-004/FR-007).
    if is_claude {
        let context = render_additional_context(&extracted);
        // A successfully-recorded or empty extraction still injects; only a
        // genuinely failed decode/extract (handled above) skips the injection.
        let _ = recorded;
        println!(
            "{}",
            serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PostToolUse",
                    "additionalContext": context,
                }
            })
        );
    }

    Ok(())
}

/// Records values extracted from PDF text into the provenance graph, mirroring
/// the text-capture path. Returns whether a source document was upserted.
fn record_pdf_values(path: &str, text: &str, agent: Option<&str>) -> io::Result<bool> {
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        return Ok(false);
    }
    let meta = match get_metadata_snapshot(path_obj) {
        Ok(m) => m,
        Err(_) => return Ok(false),
    };
    let hash = match compute_blake3_hash(path_obj) {
        Ok(h) => h,
        Err(_) => return Ok(false),
    };

    let db = match DbStore::open_in_project(".") {
        Ok(d) => d,
        Err(_) => return Ok(false),
    };
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
        Err(_) => return Ok(false),
    };

    let extraction = extract_values_from_content(text, number_format);
    if extraction.values.is_empty() {
        return Ok(true);
    }
    let now = Utc::now();
    let tx = match db.conn().unchecked_transaction() {
        Ok(tx) => tx,
        Err(_) => return Ok(false),
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
            return Ok(true); // rollback on drop; fail open
        }
        if let Some(agent_name) = agent {
            if insert_capture_association(&tx, &node.id, agent_name).is_err() {
                return Ok(true); // rollback value and attribution together
            }
        }
    }
    if tx.commit().is_err() {
        return Ok(true);
    }
    Ok(true)
}
