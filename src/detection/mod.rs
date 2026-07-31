pub mod tier1;
pub mod tier2_blake3;

pub use tier1::*;
pub use tier2_blake3::*;

use std::path::Path;

pub enum DetectionResult {
    Unchanged,
    Changed {
        new_mtime_nsec: i64,
        new_file_size: u64,
        new_blake3_hash: String,
    },
    FileNotFound,
}

pub fn evaluate_file_change<P: AsRef<Path>>(
    path: P,
    stored_mtime_nsec: i64,
    stored_file_size: u64,
    stored_blake3_hash: &str,
) -> DetectionResult {
    let path_ref = path.as_ref();
    let meta = match get_metadata_snapshot(path_ref) {
        Ok(m) => m,
        Err(_) => return DetectionResult::FileNotFound,
    };

    if !tier1_check_changed(&meta, stored_mtime_nsec, stored_file_size) {
        return DetectionResult::Unchanged;
    }

    let new_hash = match compute_blake3_hash(path_ref) {
        Ok(h) => h,
        Err(_) => return DetectionResult::FileNotFound,
    };

    if new_hash == stored_blake3_hash {
        return DetectionResult::Unchanged;
    }

    DetectionResult::Changed {
        new_mtime_nsec: meta.mtime_nsec,
        new_file_size: meta.file_size,
        new_blake3_hash: new_hash,
    }
}
