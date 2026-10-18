use crate::detection::{compute_blake3_hash_from_bytes, get_metadata_snapshot};
use crate::hooks::parser::{extract_values_from_content, NumberFormat};
use crate::store::queries::get_setting;
use crate::store::DbStore;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const VALUE_LIMIT: usize = 10_000;

/// Structured outcome of a record operation (spec 041): returned by the core
/// so the CLI wrapper and the MCP `record_values` tool can both report the same
/// result without printing inside the logic.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecordOutcome {
    pub count: usize,
    pub skipped_limit: bool,
    pub skipped_malformed: usize,
    pub number_format: String,
}

/// Resolves the active number format: an explicit CLI flag wins, then the
/// persisted `number_format` setting, then `Auto` (FR-003).
pub fn resolve_number_format(
    conn: &rusqlite::Connection,
    explicit: Option<NumberFormat>,
) -> NumberFormat {
    if let Some(f) = explicit {
        return f;
    }
    match get_setting(conn, "number_format") {
        Ok(Some(raw)) => raw.parse().unwrap_or(NumberFormat::Auto),
        _ => NumberFormat::Auto,
    }
}

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

pub fn execute_record(
    file: &str,
    stdin: bool,
    explicit_format: Option<NumberFormat>,
) -> Result<(), String> {
    let outcome = execute_record_with_outcome(file, stdin, explicit_format)?;
    if outcome.skipped_limit {
        eprintln!(
            "Warning: value limit ({}) reached, excess values skipped for {}",
            VALUE_LIMIT, file
        );
    }
    if outcome.skipped_malformed > 0 {
        eprintln!(
            "Warning: {} number(s) in {} could not be parsed as {} and were skipped (not split into bogus nodes)",
            outcome.skipped_malformed,
            file,
            outcome.number_format
        );
    }
    println!("Recorded {} values from {}", outcome.count, file);
    Ok(())
}

/// Core record operation returning a structured [`RecordOutcome`] (no printing;
/// spec 041 FR-004). The CLI wrapper prints from the outcome; the MCP
/// `record_values` tool returns it.
pub fn execute_record_with_outcome(
    file: &str,
    stdin: bool,
    explicit_format: Option<NumberFormat>,
) -> Result<RecordOutcome, String> {
    execute_record_with_content(file, stdin, None, explicit_format)
}

/// Core record operation accepting optional in-memory `explicit_content` (used by
/// the MCP `record_values` tool so it never reads the JSON-RPC stdio stream).
pub fn execute_record_with_content(
    file: &str,
    stdin: bool,
    explicit_content: Option<String>,
    explicit_format: Option<NumberFormat>,
) -> Result<RecordOutcome, String> {
    let path_obj = Path::new(file);
    let (content, hash, meta) = if let Some(text) = explicit_content {
        let text_bytes = text.as_bytes();
        let hash = compute_blake3_hash_from_bytes(text_bytes);
        let meta = if file != "<stdin>" && path_obj.exists() {
            if let Ok(disk_bytes) = fs::read(path_obj) {
                if disk_bytes == text_bytes {
                    get_metadata_snapshot(path_obj).unwrap_or(crate::detection::MetadataSnapshot {
                        mtime_nsec: 0,
                        file_size: text_bytes.len() as u64,
                        dev: None,
                        ino: None,
                    })
                } else {
                    crate::detection::MetadataSnapshot {
                        mtime_nsec: 0,
                        file_size: text_bytes.len() as u64,
                        dev: None,
                        ino: None,
                    }
                }
            } else {
                crate::detection::MetadataSnapshot {
                    mtime_nsec: 0,
                    file_size: text_bytes.len() as u64,
                    dev: None,
                    ino: None,
                }
            }
        } else {
            crate::detection::MetadataSnapshot {
                mtime_nsec: 0,
                file_size: text_bytes.len() as u64,
                dev: None,
                ino: None,
            }
        };
        (text, hash, meta)
    } else if stdin {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("failed to read stdin: {}", e))?;
        let content = read_utf8_content(file, buf.clone())?;
        let hash = compute_blake3_hash_from_bytes(&buf);
        let meta = if file != "<stdin>" && path_obj.exists() {
            if let Ok(disk_bytes) = fs::read(path_obj) {
                if disk_bytes == buf {
                    get_metadata_snapshot(path_obj).unwrap_or(crate::detection::MetadataSnapshot {
                        mtime_nsec: 0,
                        file_size: buf.len() as u64,
                        dev: None,
                        ino: None,
                    })
                } else {
                    crate::detection::MetadataSnapshot {
                        mtime_nsec: 0,
                        file_size: buf.len() as u64,
                        dev: None,
                        ino: None,
                    }
                }
            } else {
                crate::detection::MetadataSnapshot {
                    mtime_nsec: 0,
                    file_size: buf.len() as u64,
                    dev: None,
                    ino: None,
                }
            }
        } else {
            crate::detection::MetadataSnapshot {
                mtime_nsec: 0,
                file_size: buf.len() as u64,
                dev: None,
                ino: None,
            }
        };
        (content, hash, meta)
    } else {
        if !path_obj.exists() {
            return Err(format!("file not found: {}", file));
        }
        let meta = get_metadata_snapshot(path_obj)
            .map_err(|e| format!("failed to get metadata: {}", e))?;
        let bytes = fs::read(file).map_err(|e| format!("failed to read file: {}", e))?;
        let hash = compute_blake3_hash_from_bytes(&bytes);
        let content = read_utf8_content(file, bytes)?;
        (content, hash, meta)
    };

    let effective_root =
        DbStore::find_project_root(".").unwrap_or_else(|| std::path::PathBuf::from("."));
    let stored_path = DbStore::normalize_path_for_project(file, &effective_root);

    let mut db = DbStore::open_in_project(&effective_root)
        .map_err(|e| format!("failed to open database: {}", e))?;
    let conn = db.conn_mut();

    let number_format = resolve_number_format(conn, explicit_format);

    let extraction = extract_values_from_content(&content, number_format);
    let skipped_malformed = extraction.skipped_malformed;
    let mut extracted = extraction.values;

    let mut skipped = false;
    if extracted.len() > VALUE_LIMIT {
        extracted.truncate(VALUE_LIMIT);
        skipped = true;
    }

    let count = extracted.len();

    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| format!("failed to begin transaction: {}", e))?;

    crate::store::queries::record_file_batch(&tx, &stored_path, &meta, &hash, &extracted, None)
        .map_err(|e| format!("failed to record file values: {}", e))?;

    tx.commit()
        .map_err(|e| format!("failed to commit transaction: {}", e))?;

    Ok(RecordOutcome {
        count,
        skipped_limit: skipped,
        skipped_malformed,
        number_format: number_format.as_str().to_string(),
    })
}
