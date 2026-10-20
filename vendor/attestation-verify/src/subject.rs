//! The artifact digest being verified.

use std::fmt;

use sha2::{Digest as _, Sha256};

use crate::error::{Error, ParseError};

/// A SHA-256 digest identifying the artifact bytes to verify.
///
/// This is the crate's one representation of "which artifact" — callers
/// either hash bytes themselves ([`Subject::sha256_of`]) or supply an
/// already-computed hex digest ([`Subject::from_digest_hex`]). Internally
/// it is always exactly 32 bytes; there is no way to construct an invalid
/// one.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Subject([u8; 32]);

impl Subject {
    /// Computes the SHA-256 digest of `bytes` and wraps it as a `Subject`.
    #[must_use]
    pub fn sha256_of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Subject(out)
    }

    /// Parses an already-computed SHA-256 digest from its hex
    /// representation.
    ///
    /// Strict: `s` must be exactly 64 hexadecimal characters (mixed case is
    /// accepted on input; the digest is always stored and displayed in
    /// canonical lowercase). Anything else — wrong length, non-hex
    /// characters, surrounding whitespace — is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::MalformedDigest`] if `s` is not exactly 64 hex
    /// characters.
    pub fn from_digest_hex(s: &str) -> Result<Self, Error> {
        let mut out = [0u8; 32];
        hex::decode_to_slice(s, &mut out).map_err(|e| {
            Error::Parse(ParseError::MalformedDigest(format!(
                "expected 64 hex characters (sha256): {e}"
            )))
        })?;
        Ok(Subject(out))
    }

    /// The raw 32-byte digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The digest as a canonical lowercase hex string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Subject").field(&self.to_hex()).finish()
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::Subject;
    use crate::error::{Error, ParseError};

    /// sha256("") — a well-known test vector.
    const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn sha256_of_matches_known_vector() -> Result<(), Box<dyn std::error::Error>> {
        let subject = Subject::sha256_of(b"");
        if subject.to_hex() != SHA256_EMPTY {
            return Err(format!("unexpected digest: {}", subject.to_hex()).into());
        }
        Ok(())
    }

    #[test]
    fn from_digest_hex_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let subject = Subject::from_digest_hex(SHA256_EMPTY)?;
        if subject.to_hex() != SHA256_EMPTY {
            return Err("round trip mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn from_digest_hex_canonicalizes_case() -> Result<(), Box<dyn std::error::Error>> {
        let lower = Subject::from_digest_hex(&"a".repeat(64))?;
        let upper = Subject::from_digest_hex(&"A".repeat(64))?;
        if lower != upper {
            return Err("case-insensitive parse should produce equal Subjects".into());
        }
        if upper.to_hex() != "a".repeat(64) {
            return Err("to_hex should always be lowercase".into());
        }
        Ok(())
    }

    #[test]
    fn from_digest_hex_rejects_63_chars() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(&"a".repeat(63))
    }

    #[test]
    fn from_digest_hex_rejects_65_chars() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(&"a".repeat(65))
    }

    #[test]
    fn from_digest_hex_rejects_non_hex() -> Result<(), Box<dyn std::error::Error>> {
        expect_malformed(&"z".repeat(64))
    }

    fn expect_malformed(input: &str) -> Result<(), Box<dyn std::error::Error>> {
        match Subject::from_digest_hex(input) {
            Err(Error::Parse(ParseError::MalformedDigest(_))) => Ok(()),
            other => Err(format!("expected MalformedDigest error, got {other:?}").into()),
        }
    }
}
