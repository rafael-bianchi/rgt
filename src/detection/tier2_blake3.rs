use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Computes the BLAKE3 hex hash of a file's contents (streamed in 64KB chunks).
pub fn compute_blake3_hash<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Computes the BLAKE3 hex hash of an in-memory byte slice (used to hash the
/// exact bytes that were extracted — no second file read, avoiding the §7.4
/// TOCTOU).
pub fn compute_blake3_hash_from_bytes(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}
