use crate::detection::{compute_blake3_hash, get_metadata_snapshot};
use crate::hooks::parser::extract_values_from_content;
use crate::store::queries::{insert_tracked_node, upsert_source_document};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode};
use chrono::Utc;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const VALUE_LIMIT: usize = 10_000;

pub fn execute_record(file: &str, stdin: bool) -> Result<(), String> {
    let path_obj = Path::new(file);

    if !path_obj.exists() {
        return Err(format!("file not found: {}", file));
    }

    let content = if stdin {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        buf
    } else {
        fs::read_to_string(file).map_err(|e| format!("failed to read file: {}", e))?
    };

    let meta =
        get_metadata_snapshot(path_obj).map_err(|e| format!("failed to get metadata: {}", e))?;
    let hash =
        compute_blake3_hash(path_obj).map_err(|e| format!("failed to compute hash: {}", e))?;

    let db =
        DbStore::open_in_project(".").map_err(|e| format!("failed to open database: {}", e))?;
    let conn = db.conn();

    let doc = upsert_source_document(conn, file, meta.mtime_nsec, meta.file_size, &hash)
        .map_err(|e| format!("failed to upsert source document: {}", e))?;

    let mut extracted = extract_values_from_content(&content);

    let mut skipped = false;
    if extracted.len() > VALUE_LIMIT {
        extracted.truncate(VALUE_LIMIT);
        skipped = true;
    }

    let count = extracted.len();
    let now = Utc::now();

    for ext in &extracted {
        let node_id = TrackedNode::generate_root_id(file, ext.line_number, &ext.value);
        let node = TrackedNode {
            id: node_id,
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

        insert_tracked_node(conn, &node).map_err(|e| format!("failed to insert node: {}", e))?;
    }

    if skipped {
        eprintln!(
            "Warning: value limit ({}) reached, excess values skipped for {}",
            VALUE_LIMIT, file
        );
    }

    println!("Recorded {} values from {}", count, file);

    Ok(())
}
