use crate::detection::{compute_blake3_hash_from_bytes, get_metadata_snapshot};
use crate::hooks::parser::extract_values_from_content;
use crate::store::queries::{insert_tracked_node, upsert_source_document};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode};
use chrono::Utc;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const VALUE_LIMIT: usize = 10_000;

/// Decodes `bytes` as UTF-8, or returns an explicit "unsupported file format"
/// error for binary/non-UTF-8 content (FR-001) instead of a raw UTF-8 decode
/// error. Covers both file reads and `--stdin`.
fn read_utf8_content(source: &str, bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| {
        format!(
            "unsupported file format for `rgt record` at `{}`: expected plain text/CSV. \
             PDF, Excel, and other binary files are not natively parsed — use your AI agent's \
             own read/extraction tool (its output is captured automatically) or convert to text first.",
            source
        )
    })
}

pub fn execute_record(file: &str, stdin: bool) -> Result<(), String> {
    let path_obj = Path::new(file);

    if !path_obj.exists() {
        return Err(format!("file not found: {}", file));
    }

    // FR-002: hash the exact in-memory bytes that are parsed for values (no
    // second file read — avoids the §7.4 TOCTOU between extraction and hashing).
    let (content, hash) = if stdin {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        let hash = compute_blake3_hash_from_bytes(&buf);
        let content = read_utf8_content(file, buf)?;
        (content, hash)
    } else {
        let bytes = fs::read(file).map_err(|e| format!("failed to read file: {}", e))?;
        let hash = compute_blake3_hash_from_bytes(&bytes);
        let content = read_utf8_content(file, bytes)?;
        (content, hash)
    };

    let meta =
        get_metadata_snapshot(path_obj).map_err(|e| format!("failed to get metadata: {}", e))?;

    let mut db =
        DbStore::open_in_project(".").map_err(|e| format!("failed to open database: {}", e))?;
    let conn = db.conn_mut();

    let doc = upsert_source_document(
        conn,
        file,
        meta.mtime_nsec,
        meta.file_size,
        &hash,
        meta.dev,
        meta.ino,
    )
    .map_err(|e| format!("failed to upsert source document: {}", e))?;

    let mut extracted = extract_values_from_content(&content);

    let mut skipped = false;
    if extracted.len() > VALUE_LIMIT {
        extracted.truncate(VALUE_LIMIT);
        skipped = true;
    }

    let count = extracted.len();
    let now = Utc::now();

    let tx = conn
        .transaction()
        .map_err(|e| format!("failed to begin transaction: {}", e))?;

    for ext in &extracted {
        let node_id =
            TrackedNode::generate_root_id(file, ext.line_number, ext.occurrence, &ext.value);
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

        insert_tracked_node(&tx, &node).map_err(|e| format!("failed to insert node: {}", e))?;
    }

    tx.commit()
        .map_err(|e| format!("failed to commit transaction: {}", e))?;

    if skipped {
        eprintln!(
            "Warning: value limit ({}) reached, excess values skipped for {}",
            VALUE_LIMIT, file
        );
    }

    println!("Recorded {} values from {}", count, file);

    Ok(())
}
