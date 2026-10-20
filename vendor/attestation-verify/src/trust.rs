//! Trusted-root (`application/vnd.dev.sigstore.trustedroot+json;version=0.1`)
//! parsing: Fulcio certificate authorities, Rekor transparency logs,
//! Certificate Transparency logs, and timestamp authorities.
//!
//! Format-compatible with `gh attestation trusted-root` output (split into
//! one JSON document per root; see `tests/fixtures/trusted-roots/`).

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::bundle::Certificate;
use crate::error::{Error, ParseError, ResourceLimitError, UnsupportedError};
use crate::limits;
use crate::parse_util;
use crate::strict_json;
use crate::time;

/// The only trusted-root `mediaType` this crate accepts.
pub const TRUSTED_ROOT_MEDIA_TYPE: &str =
    "application/vnd.dev.sigstore.trustedroot+json;version=0.1";

/// The Sigstore public-good trusted root, embedded at build time
/// (`assets/trusted_root_public_good.json`, identical to
/// `tests/fixtures/trusted-roots/public-good.json`).
const EMBEDDED_PUBLIC_GOOD: &str = include_str!("../assets/trusted_root_public_good.json");

/// A parsed and structurally-hardened trusted-root document: the
/// certificate authorities, transparency logs, and timestamp authorities a
/// [`crate::Verifier`] may trust.
///
/// `PartialEq`/`Eq` are hand-implemented rather than derived: they compare
/// only the parsed trust content (`media_type`, `tlogs`, `ctlogs`,
/// `certificate_authorities`, `timestamp_authorities`), not the
/// `fingerprint`/`source` provenance metadata below, so two trust stores
/// loaded from byte-identical content are equal regardless of which
/// constructor built them.
#[derive(Debug, Clone)]
pub struct TrustStore {
    /// Always [`TRUSTED_ROOT_MEDIA_TYPE`].
    pub media_type: String,
    /// Rekor transparency logs.
    pub tlogs: Vec<TransparencyLog>,
    /// Certificate Transparency logs, used to verify embedded SCTs
    /// (DESIGN.md "X.509 / Fulcio validation profile"). Empty for trust
    /// roots that carry no CT logs (e.g. GitHub's own trust root, which
    /// has no transparency logs of any kind).
    pub ctlogs: Vec<CtLog>,
    /// Fulcio certificate authorities.
    pub certificate_authorities: Vec<CertificateAuthority>,
    /// RFC 3161 timestamp authorities. Same shape as
    /// [`CertificateAuthority`] in the source format, modeled with the
    /// same type here.
    pub timestamp_authorities: Vec<CertificateAuthority>,
    /// Lowercase-hex SHA-256 of the exact input JSON bytes this trust
    /// store was parsed from, computed once at parse time. Reported in
    /// every [`crate::verifier::VerificationReport`]
    /// (`TrustSnapshotInfo`) so operators can tell exactly which trust-
    /// root snapshot decided a result (DESIGN.md "Core decisions" item 3,
    /// "Trust-root operations").
    pub fingerprint: String,
    /// Where this trust store was loaded from: `"embedded-public-good"`
    /// for [`TrustStore::embedded_public_good`], `"external"` for a
    /// caller-supplied [`TrustStore::from_json`].
    pub source: String,
}

impl PartialEq for TrustStore {
    fn eq(&self, other: &Self) -> bool {
        self.media_type == other.media_type
            && self.tlogs == other.tlogs
            && self.ctlogs == other.ctlogs
            && self.certificate_authorities == other.certificate_authorities
            && self.timestamp_authorities == other.timestamp_authorities
    }
}

impl Eq for TrustStore {}

impl TrustStore {
    /// Parses a single trusted-root JSON document.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitError::InputTooLarge`] if `bytes` exceeds
    /// the input-size limit, [`UnsupportedError::MediaType`] if
    /// `mediaType` is not [`TRUSTED_ROOT_MEDIA_TYPE`], and the usual
    /// [`ParseError`] variants for malformed or strict-decoding failures
    /// (including malformed RFC 3339 `validFor` timestamps and oversized
    /// certificate chains).
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        parse_util::check_input_size(bytes)?;
        let value = strict_json::parse_strict(bytes)?;
        let raw: RawTrustedRoot =
            serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;
        let fingerprint = hex::encode(Sha256::digest(bytes));
        Self::from_raw(raw, fingerprint, "external".to_owned())
    }

    /// The embedded Sigstore public-good trusted root.
    ///
    /// Returns `Result` (never panics) even though the embedded bytes are
    /// fixed at build time: `unwrap`/`expect` are denied by this crate's
    /// lints, and surfacing a real `Error` keeps this consistent with
    /// every other constructor rather than special-casing "this one can't
    /// fail."
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the embedded document somehow fails to
    /// parse (would indicate a packaging bug in this crate, not a caller
    /// mistake).
    pub fn embedded_public_good() -> Result<Self, Error> {
        let mut store = Self::from_json(EMBEDDED_PUBLIC_GOOD.as_bytes())?;
        "embedded-public-good".clone_into(&mut store.source);
        Ok(store)
    }

    fn from_raw(raw: RawTrustedRoot, fingerprint: String, source: String) -> Result<Self, Error> {
        let RawTrustedRoot {
            media_type,
            tlogs,
            ctlogs,
            certificate_authorities,
            timestamp_authorities,
        } = raw;
        if media_type != TRUSTED_ROOT_MEDIA_TYPE {
            return Err(Error::Unsupported(UnsupportedError::MediaType {
                found: media_type,
            }));
        }
        let tlogs = tlogs
            .into_iter()
            .map(TransparencyLog::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let ctlogs = ctlogs
            .into_iter()
            .map(CtLog::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let certificate_authorities = certificate_authorities
            .into_iter()
            .map(CertificateAuthority::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let timestamp_authorities = timestamp_authorities
            .into_iter()
            .map(CertificateAuthority::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TrustStore {
            media_type,
            tlogs,
            ctlogs,
            certificate_authorities,
            timestamp_authorities,
            fingerprint,
            source,
        })
    }
}

/// A Rekor transparency log (`tlogs[]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyLog {
    /// The log's base URL, e.g. `"https://rekor.sigstore.dev"`.
    pub base_url: String,
    /// The log's Merkle-tree hash algorithm, e.g. `"SHA2_256"`.
    pub hash_algorithm: String,
    /// The log's signing key.
    pub public_key: PublicKey,
    /// The log's id (`logId.keyId`), base64-decoded.
    pub log_id_key_id: Vec<u8>,
    /// A distinct key id for checkpoint (signed-note) verification, if the
    /// log uses one, base64-decoded.
    pub checkpoint_key_id: Option<Vec<u8>>,
}

impl TransparencyLog {
    fn from_raw(raw: RawTlog) -> Result<Self, Error> {
        let RawTlog {
            base_url,
            hash_algorithm,
            public_key,
            log_id,
            checkpoint_key_id,
        } = raw;
        let public_key = PublicKey::from_raw(public_key)?;
        let log_id_key_id = parse_util::strict_base64("tlogs[].logId.keyId", &log_id.key_id)?;
        let checkpoint_key_id = checkpoint_key_id
            .as_deref()
            .map(|k| parse_util::strict_base64("tlogs[].checkpointKeyId", k))
            .transpose()?;
        Ok(TransparencyLog {
            base_url,
            hash_algorithm,
            public_key,
            log_id_key_id,
            checkpoint_key_id,
        })
    }
}

/// A Certificate Transparency log (`ctlogs[]`), used to verify embedded
/// SCTs (DESIGN.md "X.509 / Fulcio validation profile").
///
/// Same shape as [`TransparencyLog`] in the source format minus
/// `checkpointKeyId`: CT logs are verified via SCTs, not Rekor-style
/// signed checkpoints, so the source format never carries one for these
/// entries. Modeled as its own type (rather than reusing
/// [`TransparencyLog`], as [`CertificateAuthority`] is reused for
/// `timestamp_authorities`) so a CT log can never be mistaken for
/// carrying checkpoint-verification data it does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtLog {
    /// The log's base URL, e.g. `"https://ctfe.sigstore.dev/2022"`.
    pub base_url: String,
    /// The log's hash algorithm, e.g. `"SHA2_256"`.
    pub hash_algorithm: String,
    /// The log's signing key.
    pub public_key: PublicKey,
    /// The log's id (`logId.keyId`), base64-decoded.
    pub log_id_key_id: Vec<u8>,
}

impl CtLog {
    fn from_raw(raw: RawTlog) -> Result<Self, Error> {
        let RawTlog {
            base_url,
            hash_algorithm,
            public_key,
            log_id,
            checkpoint_key_id: _,
        } = raw;
        let public_key = PublicKey::from_raw(public_key)?;
        let log_id_key_id = parse_util::strict_base64("ctlogs[].logId.keyId", &log_id.key_id)?;
        Ok(CtLog {
            base_url,
            hash_algorithm,
            public_key,
            log_id_key_id,
        })
    }
}

/// A public key with a validity window (`publicKey`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    /// SPKI DER bytes, base64-decoded.
    pub raw_bytes: Vec<u8>,
    /// The key's algorithm/format label, e.g.
    /// `"PKIX_ECDSA_P256_SHA_256"` or `"PKIX_ED25519"`.
    pub key_details: String,
    /// The window during which this key is trusted.
    pub valid_for: ValidityPeriod,
}

impl PublicKey {
    fn from_raw(raw: RawPublicKey) -> Result<Self, Error> {
        let RawPublicKey {
            raw_bytes,
            key_details,
            valid_for,
        } = raw;
        let raw_bytes = parse_util::strict_base64("publicKey.rawBytes", &raw_bytes)?;
        let valid_for = ValidityPeriod::from_raw(&valid_for)?;
        Ok(PublicKey {
            raw_bytes,
            key_details,
            valid_for,
        })
    }
}

/// A certificate authority or timestamp authority
/// (`certificateAuthorities[]` / `timestampAuthorities[]` — the source
/// format uses the same shape for both).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateAuthority {
    /// The CA's X.509 distinguished-name subject.
    pub subject: CaSubject,
    /// The CA's URI, e.g. `"https://fulcio.sigstore.dev"`.
    pub uri: String,
    /// The certificate chain, root-independent order as given in the
    /// document, each entry raw DER bytes.
    pub certificates: Vec<Certificate>,
    /// The window during which this CA is trusted.
    pub valid_for: ValidityPeriod,
}

impl CertificateAuthority {
    fn from_raw(raw: RawCertificateAuthority) -> Result<Self, Error> {
        let RawCertificateAuthority {
            subject,
            uri,
            cert_chain,
            valid_for,
        } = raw;
        parse_util::check_count(
            &cert_chain.certificates,
            limits::MAX_CERTIFICATES_PER_CHAIN,
            |actual, limit| ResourceLimitError::TooManyCertificates { actual, limit },
        )?;
        let certificates = cert_chain
            .certificates
            .into_iter()
            .map(|entry| {
                let raw_bytes = parse_util::strict_base64(
                    "certChain.certificates[].rawBytes",
                    &entry.raw_bytes,
                )?;
                Ok(Certificate { raw_bytes })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let valid_for = ValidityPeriod::from_raw(&valid_for)?;
        Ok(CertificateAuthority {
            subject: CaSubject {
                organization: subject.organization,
                common_name: subject.common_name,
            },
            uri,
            certificates,
            valid_for,
        })
    }
}

/// The X.509 distinguished-name subject of a [`CertificateAuthority`].
///
/// Named distinctly from [`crate::Subject`] (the artifact digest type) to
/// avoid confusion: this is "whose certificate," not "which artifact."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaSubject {
    /// The `O=` (organization) field.
    pub organization: String,
    /// The `CN=` (common name) field.
    pub common_name: String,
}

/// A trust window: `[start, end)` in unix seconds, `end` absent meaning
/// still valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidityPeriod {
    /// Start of the validity window, unix seconds.
    pub start: i64,
    /// End of the validity window, unix seconds, if bounded.
    pub end: Option<i64>,
}

impl ValidityPeriod {
    fn from_raw(raw: &RawValidFor) -> Result<Self, Error> {
        let start = time::parse_rfc3339("validFor.start", &raw.start)?;
        let end = raw
            .end
            .as_deref()
            .map(|e| time::parse_rfc3339("validFor.end", e))
            .transpose()?;
        Ok(ValidityPeriod { start, end })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTrustedRoot {
    media_type: String,
    #[serde(default)]
    tlogs: Vec<RawTlog>,
    #[serde(default)]
    ctlogs: Vec<RawTlog>,
    #[serde(default)]
    certificate_authorities: Vec<RawCertificateAuthority>,
    #[serde(default)]
    timestamp_authorities: Vec<RawCertificateAuthority>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTlog {
    base_url: String,
    hash_algorithm: String,
    public_key: RawPublicKey,
    log_id: RawLogId,
    #[serde(default)]
    checkpoint_key_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPublicKey {
    raw_bytes: String,
    key_details: String,
    valid_for: RawValidFor,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLogId {
    key_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCertificateAuthority {
    subject: RawCaSubject,
    uri: String,
    cert_chain: RawCertChain,
    valid_for: RawValidFor,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCaSubject {
    organization: String,
    common_name: String,
}

#[derive(Deserialize)]
struct RawCertChain {
    certificates: Vec<RawCertEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCertEntry {
    raw_bytes: String,
}

#[derive(Deserialize)]
struct RawValidFor {
    start: String,
    #[serde(default)]
    end: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{TRUSTED_ROOT_MEDIA_TYPE, TrustStore};
    use crate::error::{Error, UnsupportedError};

    fn minimal_valid_json() -> String {
        format!(
            r#"{{
                "mediaType": "{TRUSTED_ROOT_MEDIA_TYPE}",
                "tlogs": [],
                "certificateAuthorities": [
                    {{
                        "subject": {{"organization": "example.test", "commonName": "example"}},
                        "uri": "https://fulcio.example.test",
                        "certChain": {{"certificates": [{{"rawBytes": "aGVsbG8="}}]}},
                        "validFor": {{"start": "2021-01-01T00:00:00Z"}}
                    }}
                ],
                "timestampAuthorities": []
            }}"#
        )
    }

    /// A single `ctlogs[]` entry with the same shape as a `tlogs[]` entry
    /// (DESIGN.md task: "Model it identically"), minus `checkpointKeyId`.
    fn minimal_json_with_one_ctlog() -> String {
        format!(
            r#"{{
                "mediaType": "{TRUSTED_ROOT_MEDIA_TYPE}",
                "tlogs": [],
                "ctlogs": [
                    {{
                        "baseUrl": "https://ctfe.example.test/2022",
                        "hashAlgorithm": "SHA2_256",
                        "publicKey": {{
                            "rawBytes": "aGVsbG8=",
                            "keyDetails": "PKIX_ECDSA_P256_SHA_256",
                            "validFor": {{"start": "2022-01-01T00:00:00Z"}}
                        }},
                        "logId": {{"keyId": "d29ybGQ="}}
                    }}
                ],
                "certificateAuthorities": [],
                "timestampAuthorities": []
            }}"#
        )
    }

    #[test]
    fn parses_minimal_valid_trust_root() -> Result<(), Box<dyn std::error::Error>> {
        let store = TrustStore::from_json(minimal_valid_json().as_bytes())?;
        if store.certificate_authorities.len() != 1 {
            return Err("expected exactly one CA".into());
        }
        let ca = &store.certificate_authorities[0];
        if ca.valid_for.start != 1_609_459_200 {
            return Err("unexpected validFor.start".into());
        }
        if ca.valid_for.end.is_some() {
            return Err("expected no validFor.end".into());
        }
        Ok(())
    }

    #[test]
    fn ctlogs_defaults_to_empty_when_absent() -> Result<(), Box<dyn std::error::Error>> {
        // `minimal_valid_json()` has no "ctlogs" key at all (matching
        // `github.json`'s real shape).
        let store = TrustStore::from_json(minimal_valid_json().as_bytes())?;
        if !store.ctlogs.is_empty() {
            return Err(format!("expected 0 ctlogs, got {}", store.ctlogs.len()).into());
        }
        Ok(())
    }

    #[test]
    fn parses_one_ctlog() -> Result<(), Box<dyn std::error::Error>> {
        let store = TrustStore::from_json(minimal_json_with_one_ctlog().as_bytes())?;
        if store.ctlogs.len() != 1 {
            return Err(format!("expected 1 ctlog, got {}", store.ctlogs.len()).into());
        }
        let ctlog = &store.ctlogs[0];
        if ctlog.base_url != "https://ctfe.example.test/2022" {
            return Err(format!("unexpected baseUrl: {}", ctlog.base_url).into());
        }
        if ctlog.hash_algorithm != "SHA2_256" {
            return Err(format!("unexpected hashAlgorithm: {}", ctlog.hash_algorithm).into());
        }
        if ctlog.public_key.raw_bytes != b"hello" {
            return Err("unexpected publicKey.rawBytes".into());
        }
        if ctlog.public_key.key_details != "PKIX_ECDSA_P256_SHA_256" {
            return Err("unexpected publicKey.keyDetails".into());
        }
        if ctlog.public_key.valid_for.start != 1_640_995_200 {
            return Err("unexpected publicKey.validFor.start".into());
        }
        if ctlog.log_id_key_id != b"world" {
            return Err("unexpected logId.keyId".into());
        }
        Ok(())
    }

    #[test]
    fn embedded_public_good_parses() -> Result<(), Box<dyn std::error::Error>> {
        TrustStore::embedded_public_good()?;
        Ok(())
    }

    #[test]
    fn rejects_wrong_media_type() -> Result<(), Box<dyn std::error::Error>> {
        let json = minimal_valid_json().replace(TRUSTED_ROOT_MEDIA_TYPE, "application/x-bogus");
        match TrustStore::from_json(json.as_bytes()) {
            Err(Error::Unsupported(UnsupportedError::MediaType { .. })) => Ok(()),
            other => Err(format!("expected MediaType error, got {other:?}").into()),
        }
    }
}
