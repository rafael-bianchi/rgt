pub mod tier1;
pub mod tier2_blake3;

pub use tier1::*;
pub use tier2_blake3::*;

use std::io;
use std::path::Path;

/// How long a file may go without a full content hash before detection forces
/// one, even when Tier 1 (mtime+size) matches (FR-001). Bounds the
/// same-mtime/same-size undetected window.
pub const REHASH_THRESHOLD: chrono::Duration = chrono::Duration::hours(1);

pub enum DetectionResult {
    Unchanged,
    Changed {
        new_mtime_nsec: i64,
        new_file_size: u64,
        new_blake3_hash: String,
        new_dev: Option<u64>,
        new_ino: Option<u64>,
    },
    FileNotFound,
    /// Any I/O error that is NOT `io::ErrorKind::NotFound` (permissions, locks,
    /// path-replaced-by-directory, ...). Treated as "state unchanged" — never
    /// mislabeled as a deletion (FR-003).
    CheckFailed,
}

/// Evaluates whether a file has changed using two-tier detection.
///
/// Tier 1 compares mtime + size (and on-disk identity, when both known); if
/// unchanged, returns `Unchanged` without hashing. Tier 2 computes a BLAKE3
/// hash only when Tier 1 indicates a potential change OR `force_hash` is set
/// (FR-001: files not fully checked within `REHASH_THRESHOLD` are force-hashed
/// so a same-mtime/same-size rewrite is eventually caught).
pub fn evaluate_file_change<P: AsRef<Path>>(
    path: P,
    stored_mtime_nsec: i64,
    stored_file_size: u64,
    stored_blake3_hash: &str,
    stored_dev: Option<u64>,
    stored_ino: Option<u64>,
    force_hash: bool,
) -> DetectionResult {
    let path_ref = path.as_ref();
    let meta = match get_metadata_snapshot(path_ref) {
        Ok(m) => m,
        Err(e) => return map_io_error(e),
    };

    let tier1_same = !tier1_check_changed(
        &meta,
        stored_mtime_nsec,
        stored_file_size,
        stored_dev,
        stored_ino,
    );
    if tier1_same && !force_hash {
        return DetectionResult::Unchanged;
    }

    let new_hash = match compute_blake3_hash(path_ref) {
        Ok(h) => h,
        Err(e) => return map_io_error(e),
    };

    if new_hash == stored_blake3_hash {
        return DetectionResult::Unchanged;
    }

    DetectionResult::Changed {
        new_mtime_nsec: meta.mtime_nsec,
        new_file_size: meta.file_size,
        new_blake3_hash: new_hash,
        new_dev: meta.dev,
        new_ino: meta.ino,
    }
}

/// Maps an I/O error to a `DetectionResult`: `NotFound` → `FileNotFound`;
/// anything else → `CheckFailed` (FR-003).
fn map_io_error(e: io::Error) -> DetectionResult {
    match e.kind() {
        io::ErrorKind::NotFound => DetectionResult::FileNotFound,
        _ => DetectionResult::CheckFailed,
    }
}
