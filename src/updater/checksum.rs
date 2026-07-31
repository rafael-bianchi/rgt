use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ChecksumEntry {
    pub hash: String,
    pub filename: String,
}

pub fn parse_checksums(content: &str) -> Vec<ChecksumEntry> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.splitn(2, "  ");
            let hash = parts.next()?.trim().to_string();
            let filename = parts.next()?.trim().to_string();
            if hash.len() == 64 && !filename.is_empty() {
                Some(ChecksumEntry { hash, filename })
            } else {
                None
            }
        })
        .collect()
}

pub fn find_checksum<'a>(entries: &'a [ChecksumEntry], filename: &str) -> Option<&'a ChecksumEntry> {
    entries.iter().find(|e| e.filename == filename)
}

pub fn compute_sha256(path: &Path) -> io::Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn verify_checksum(path: &Path, expected_hash: &str) -> io::Result<bool> {
    let actual = compute_sha256(path)?;
    Ok(actual == expected_hash.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_valid_checksums() {
        let content = "\
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  rgt-v0.1.0-aarch64-apple-darwin.tar.gz
a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3  rgt-v0.1.0-x86_64-apple-darwin.tar.gz
2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae  rgt-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
";
        let entries = parse_checksums(content);
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0].filename,
            "rgt-v0.1.0-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn test_parse_skips_malformed_lines() {
        let content = "invalid line\nshort  rgt.tar.gz\n";
        let entries = parse_checksums(content);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_find_checksum() {
        let entries = vec![
            ChecksumEntry {
                hash: "abc123".to_string(),
                filename: "rgt-linux.tar.gz".to_string(),
            },
            ChecksumEntry {
                hash: "def456".to_string(),
                filename: "rgt-macos.tar.gz".to_string(),
            },
        ];
        let found = find_checksum(&entries, "rgt-linux.tar.gz");
        assert!(found.is_some());
        assert_eq!(found.unwrap().hash, "abc123");
    }

    #[test]
    fn test_compute_and_verify_sha256() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let path = tmp.path();

        let hash = compute_sha256(path).unwrap();
        assert!(verify_checksum(path, &hash).unwrap());
        assert!(!verify_checksum(path, "0000000000000000000000000000000000000000000000000000000000000000").unwrap());
    }
}
