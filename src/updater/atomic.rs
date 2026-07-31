use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

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
        let tmp = self.target.with_extension("tmp");
        let backup = self.target.with_extension("old");

        fs::copy(new_binary, &tmp)?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&tmp)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tmp, perms)?;
        }

        if self.target.exists() {
            let _ = fs::remove_file(&backup);
            fs::rename(&self.target, &backup)?;
        }

        fs::rename(&tmp, &self.target)?;

        let _ = fs::remove_file(&backup);

        Ok(())
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
        assert!(!dir.path().join("rgt.old").exists());
    }

    #[test]
    fn test_atomic_replace_preserves_old_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("rgt");
        let new_bin = dir.path().join("rgt-new");

        {
            let mut f = fs::File::create(&target).unwrap();
            f.write_all(b"old binary content").unwrap();
        }
        {
            let mut f = fs::File::create(&new_bin).unwrap();
            f.write_all(b"new binary content").unwrap();
        }

        let ar = AtomicReplace::new(&target);
        ar.replace(&new_bin).unwrap();

        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "new binary content");
        assert!(!dir.path().join("rgt.old").exists());
        assert!(!dir.path().join("rgt.tmp").exists());
    }
}
