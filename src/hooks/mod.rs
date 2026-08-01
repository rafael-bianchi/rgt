pub mod installer;
pub mod parser;

use crate::detection::{compute_blake3_hash, get_metadata_snapshot};
use crate::hooks::parser::{extract_values_from_content, parse_hook_payload};
use crate::store::queries::{insert_tracked_node, upsert_source_document};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode};
use chrono::Utc;
use std::io::{self, Read};
use std::path::Path;

pub fn handle_passive_hook_event(_event_type: &str) -> io::Result<()> {
    let mut stdin_str = String::new();
    io::stdin().read_to_string(&mut stdin_str)?;

    let payload = match parse_hook_payload(&stdin_str) {
        Ok(p) => p,
        Err(_) => return Ok(()), // Fail-open non-blocking
    };

    let path_str = payload
        .tool_input
        .as_ref()
        .and_then(|i| i.path.clone().or_else(|| i.file_path.clone()));

    let content = payload
        .tool_response
        .as_ref()
        .and_then(|r| r.content.clone());

    if let (Some(ref path), Some(ref content_text)) = (path_str, content) {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Ok(());
        }

        let meta = match get_metadata_snapshot(path_obj) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };

        let hash = match compute_blake3_hash(path_obj) {
            Ok(h) => h,
            Err(_) => return Ok(()),
        };

        let db = match DbStore::open_in_project(".") {
            Ok(d) => d,
            Err(_) => return Ok(()),
        };

        let doc =
            match upsert_source_document(db.conn(), path, meta.mtime_nsec, meta.file_size, &hash) {
                Ok(d) => d,
                Err(_) => return Ok(()),
            };

        let extracted = extract_values_from_content(content_text);
        let now = Utc::now();

        for ext in extracted {
            let node_id = TrackedNode::generate_root_id(path, ext.line_number, &ext.value);
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

            let _ = insert_tracked_node(db.conn(), &node);
        }
    }

    Ok(())
}
