pub mod editor;
pub mod glue;
pub mod installer;
pub mod parser;
pub mod paths;
pub mod registration;

use crate::detection::{compute_blake3_hash_from_bytes, get_metadata_snapshot};
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
use std::sync::mpsc;
use std::time::Duration;

const MAX_HOOK_INPUT_BYTES: usize = 8 * 1024 * 1024;
const HOOK_STDIN_DEADLINE: Duration = Duration::from_millis(250);

/// Processes a passive hook event (PreToolUse/PostToolUse) by reading tool event
/// JSON from stdin, normalizing it to the internal completed-read model
/// (per the `--agent` dialect), and recording numeric/date values only when a
/// qualifying result contains complete source content that matches the
/// current file snapshot. A path without returned content never triggers a
/// disk reread or creates source-backed values. For Claude Code native PDF
/// reads, the base64 envelope is decoded and extracted locally (spec 033).
/// Fails open (never blocks the agent): every error path returns `Ok(())` with
/// no stdout, except the Claude Code `additionalContext` injection which is
/// additive.
pub fn handle_passive_hook_event(event_type: &str, agent: Option<&str>) -> io::Result<()> {
    if !event_type.eq_ignore_ascii_case("post") {
        return Ok(());
    }
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

    let Some(stdin_str) = read_bounded_hook_stdin() else {
        return Ok(());
    };

    let capture = match normalize_agent_event(Some(&canonical_agent), &stdin_str) {
        Some(c) => c,
        None => return Ok(()), // Fail-open non-blocking
    };

    if !capture.is_value_capture_eligible() {
        return Ok(());
    }

    // PDF envelope: decode, extract locally, record, and (claude-code only)
    // inject the extracted text back via PostToolUse additionalContext.
    if let Some(base64_str) = capture.pdf_base64.as_deref() {
        return handle_pdf_capture(event_type, Some(&canonical_agent), &capture, base64_str);
    }

    let (Some(path), Some(content)) = (
        capture.read_target.as_deref(),
        capture.completed_content.as_deref(),
    ) else {
        return Ok(()); // Nothing capturable
    };

    let path_obj = Path::new(path);
    let Some((meta, hash)) = matching_file_snapshot(path_obj, content.as_bytes()) else {
        return Ok(()); // Path-only, partial, or changed snapshots never create roots.
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

    let mut extraction = extract_values_from_content(content, number_format);
    for value in &mut extraction.values {
        let Some(source_line) = value.line_number else {
            return Ok(());
        };
        value.line_number = capture
            .source_line_mapping
            .as_ref()
            .and_then(|lines| lines.get(source_line.saturating_sub(1) as usize))
            .copied();
        if value.line_number.is_none() {
            return Ok(());
        }
    }
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

/// Read hook input on a detached worker so a client that keeps stdin open
/// cannot hold the passive process. The byte limit bounds allocation and the
/// elapsed deadline also covers a slow stream that never reaches EOF.
fn read_bounded_hook_stdin() -> Option<String> {
    let mut input = ProcessStdin;
    read_bounded_input(&mut input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookReadFailure {
    TimedOut,
    Failed,
}

trait HookInputSource {
    fn read(&mut self, maximum_bytes: usize, timeout: Duration)
        -> Result<Vec<u8>, HookReadFailure>;
}

struct ProcessStdin;

impl HookInputSource for ProcessStdin {
    fn read(
        &mut self,
        maximum_bytes: usize,
        timeout: Duration,
    ) -> Result<Vec<u8>, HookReadFailure> {
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut bytes = Vec::with_capacity(64 * 1024);
            let result = io::stdin()
                .take(maximum_bytes as u64)
                .read_to_end(&mut bytes);
            let _ = sender.send((result, bytes));
        });

        match receiver.recv_timeout(timeout) {
            Ok((Ok(_), bytes)) => Ok(bytes),
            Ok((Err(_), _)) => Err(HookReadFailure::Failed),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(HookReadFailure::TimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(HookReadFailure::Failed),
        }
    }
}

fn read_bounded_input(source: &mut impl HookInputSource) -> Option<String> {
    let bytes = source
        .read(MAX_HOOK_INPUT_BYTES + 1, HOOK_STDIN_DEADLINE)
        .ok()?;
    if bytes.len() > MAX_HOOK_INPUT_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
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
    let path = capture.read_target.as_deref();
    let recorded = path
        .map(|p| record_pdf_values(p, &decoded, &extracted, agent))
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
fn record_pdf_values(
    path: &str,
    source_bytes: &[u8],
    text: &str,
    agent: Option<&str>,
) -> io::Result<bool> {
    let path_obj = Path::new(path);
    let Some((meta, hash)) = matching_file_snapshot(path_obj, source_bytes) else {
        return Ok(false);
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

/// Reads a stable file snapshot and verifies that its exact bytes equal the
/// completed result delivered by the client. Metadata is sampled around the
/// read to reject files changed while RGT was checking them.
fn matching_file_snapshot(
    path: &Path,
    expected_bytes: &[u8],
) -> Option<(crate::detection::tier1::MetadataSnapshot, String)> {
    let before = get_metadata_snapshot(path).ok()?;
    let current = fs::read(path).ok()?;
    let after = get_metadata_snapshot(path).ok()?;
    let unchanged = before.mtime_nsec == after.mtime_nsec
        && before.file_size == after.file_size
        && before.dev == after.dev
        && before.ino == after.ino;
    if !unchanged || current != expected_bytes || after.file_size != current.len() as u64 {
        return None;
    }
    let hash = compute_blake3_hash_from_bytes(&current);
    Some((after, hash))
}

#[cfg(test)]
mod input_tests {
    use super::*;

    struct InjectedInput(Result<Vec<u8>, HookReadFailure>);

    impl HookInputSource for InjectedInput {
        fn read(
            &mut self,
            maximum_bytes: usize,
            timeout: Duration,
        ) -> Result<Vec<u8>, HookReadFailure> {
            assert_eq!(maximum_bytes, MAX_HOOK_INPUT_BYTES + 1);
            assert_eq!(timeout, HOOK_STDIN_DEADLINE);
            self.0.clone()
        }
    }

    #[test]
    fn bounded_input_accepts_limit_and_fails_open_on_oversize_or_stall() {
        let mut exact = InjectedInput(Ok(vec![b'a'; MAX_HOOK_INPUT_BYTES]));
        assert_eq!(
            read_bounded_input(&mut exact).unwrap().len(),
            MAX_HOOK_INPUT_BYTES
        );

        let mut oversized = InjectedInput(Ok(vec![b'a'; MAX_HOOK_INPUT_BYTES + 1]));
        assert!(read_bounded_input(&mut oversized).is_none());

        let mut stalled = InjectedInput(Err(HookReadFailure::TimedOut));
        assert!(read_bounded_input(&mut stalled).is_none());
    }
}
