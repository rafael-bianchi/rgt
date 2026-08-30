use std::fs;
use std::io;
use std::path::Path;

pub struct MetadataSnapshot {
    pub mtime_nsec: i64,
    pub file_size: u64,
    /// Resolved-target device id (Unix `st_dev`; Windows volume serial number).
    /// `None` on platforms without identity.
    pub dev: Option<u64>,
    /// Resolved-target inode (Unix `st_ino`; Windows file index). `None` when
    /// unavailable.
    pub ino: Option<u64>,
}

/// Captures a file's metadata snapshot (mtime in nanoseconds + size).
///
/// Uses nanosecond mtime on Unix; falls back to `modified()` duration on other platforms.
pub fn get_metadata_snapshot<P: AsRef<Path>>(path: P) -> io::Result<MetadataSnapshot> {
    let meta = fs::metadata(path)?;

    #[cfg(unix)]
    let (mtime_nsec, dev, ino) = {
        use std::os::unix::fs::MetadataExt;
        let mtime_nsec = meta.mtime() * 1_000_000_000 + meta.mtime_nsec();
        (mtime_nsec, Some(meta.dev()), Some(meta.ino()))
    };

    #[cfg(not(unix))]
    let (mtime_nsec, dev, ino) = {
        let modified = meta.modified()?;
        let duration = modified
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let mtime_nsec = duration.as_nanos() as i64;

        // `volume_serial_number`/`file_index` (`windows_by_handle`) are stable
        // only on the MSVC target; the GNU target compiles on stable by falling
        // back to no on-disk identity (path-based dedup).
        #[cfg(all(windows, target_env = "msvc"))]
        let (dev, ino) = {
            use std::os::windows::fs::MetadataExt;
            (
                meta.volume_serial_number().map(|v| v as u64),
                meta.file_index(),
            )
        };
        #[cfg(not(all(windows, target_env = "msvc")))]
        let (dev, ino) = (None, None);

        (mtime_nsec, dev, ino)
    };

    Ok(MetadataSnapshot {
        mtime_nsec,
        file_size: meta.len(),
        dev,
        ino,
    })
}

/// Tier 1 change check: returns `true` if mtime, size, or on-disk identity
/// (dev/ino, when both known) differs from the stored values.
pub fn tier1_check_changed(
    current: &MetadataSnapshot,
    stored_mtime_nsec: i64,
    stored_file_size: u64,
    stored_dev: Option<u64>,
    stored_ino: Option<u64>,
) -> bool {
    if current.mtime_nsec != stored_mtime_nsec || current.file_size != stored_file_size {
        return true;
    }
    // A repointed symlink / replaced file has a different inode even when
    // mtime+size match. Compare only when both sides have identity.
    if let (Some(cdev), Some(cino)) = (current.dev, current.ino) {
        if let (Some(sdev), Some(sino)) = (stored_dev, stored_ino) {
            if cdev != sdev || cino != sino {
                return true;
            }
        }
    }
    false
}
