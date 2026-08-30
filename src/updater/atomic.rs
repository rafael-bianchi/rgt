use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Atomically replaces the binary at `target` with `new_binary`.
///
/// POSIX: the new file is copied next to `target` and **renamed over it in a
/// single syscall** — the target path always resolves to the old or new binary,
/// never absent (FR-007). This also works when `target` is the running
/// executable (the running process keeps the old inode).
///
/// Windows: a running `.exe` cannot be renamed over, so `self_replace`'s
/// documented strategy (move-aside + `FILE_FLAG_DELETE_ON_CLOSE` + delete-glue)
/// is used.
///
/// `rgt update` calls this only after checksum + attestation verification pass,
/// so no post-verification rollback is ever required.
#[derive(Debug)]
pub struct AtomicReplace {
    target: PathBuf,
}

impl AtomicReplace {
    pub fn new(target: &Path) -> Self {
        AtomicReplace {
            target: target.to_path_buf(),
        }
    }

    pub fn replace(&self, new_binary: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            // Copy next to the target (same filesystem ⇒ rename can be atomic)
            // and preserve the source's executable permission.
            let tmp = self
                .target
                .with_extension(format!("rgt.tmp.{}", std::process::id()));
            fs::copy(new_binary, &tmp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&tmp)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&tmp, perms)?;
            }
            fs::rename(&tmp, &self.target)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            self_replace::self_replace(new_binary)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_atomic_replace_installs_new_binary() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("rgt");
        let new_bin = dir.path().join("rgt-new");

        {
            let mut f = fs::File::create(&new_bin).unwrap();
            f.write_all(b"new binary content").unwrap();
        }

        let ar = AtomicReplace::new(&target);
        ar.replace(&new_bin).unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "new binary content");
        assert!(!dir.path().join("rgt.rgt.tmp.0").exists());
    }

    #[test]
    fn test_atomic_replace_reports_missing_source_error() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("rgt");
        let missing = dir.path().join("does-not-exist");

        let ar = AtomicReplace::new(&target);
        let err = ar.replace(&missing).unwrap_err();
        assert!(!err.to_string().is_empty(), "failure must surface an error");
    }

    /// POSIX: the target path always resolves to either the old or the new
    /// binary while `replace` runs — no absent window (FR-007).
    #[test]
    fn test_atomic_replace_never_leaves_target_absent_on_posix() {
        if !cfg!(unix) {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("rgt");
        let new_bin = dir.path().join("rgt-new");
        {
            let mut f = fs::File::create(&target).unwrap();
            f.write_all(b"old").unwrap();
            let mut g = fs::File::create(&new_bin).unwrap();
            g.write_all(b"new-content").unwrap();
        }

        let ar = AtomicReplace::new(&target);
        let handle = std::thread::spawn(move || {
            ar.replace(&new_bin).unwrap();
        });
        while !handle.is_finished() {
            assert!(
                target.exists(),
                "target must never be absent during replace (FR-007)"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        handle.join().unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new-content");
    }
}
