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
