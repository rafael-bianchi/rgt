use std::fs;
use std::io;
use std::path::Path;

pub struct MetadataSnapshot {
    pub mtime_nsec: i64,
    pub file_size: u64,
}

/// Captures a file's metadata snapshot (mtime in nanoseconds + size).
///
/// Uses nanosecond mtime on Unix; falls back to `modified()` duration on other platforms.
pub fn get_metadata_snapshot<P: AsRef<Path>>(path: P) -> io::Result<MetadataSnapshot> {
    let meta = fs::metadata(path)?;

    #[cfg(unix)]
    let mtime_nsec = {
        use std::os::unix::fs::MetadataExt;
        meta.mtime() * 1_000_000_000 + meta.mtime_nsec()
    };

    #[cfg(not(unix))]
    let mtime_nsec = {
        let modified = meta.modified()?;
        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        duration.as_nanos() as i64
    };

    Ok(MetadataSnapshot {
        mtime_nsec,
        file_size: meta.len(),
    })
}

/// Tier 1 change check: returns `true` if mtime or size differs from the stored values.
pub fn tier1_check_changed(
    current: &MetadataSnapshot,
    stored_mtime_nsec: i64,
    stored_file_size: u64,
) -> bool {
    current.mtime_nsec != stored_mtime_nsec || current.file_size != stored_file_size
}
