//! Sigstore bundle (`application/vnd.dev.sigstore.bundle.v0.3+json`)
//! parsing, and the two multi-bundle container shapes GitHub actually
//! serves them in.
//!
//! Every type here models only the subset of the real protojson format
//! this crate verifies (DESIGN.md "Core decisions" item 2): unknown
//! top-level and nested fields are tolerated (the format evolves), but
//! every field that *is* modeled is strictly typed — including the
//! protojson quirk where `int64`/`uint64` fields (`logIndex`,
//! `integratedTime`, `treeSize`) are encoded as JSON strings, not numbers.

use serde::Deserialize;

use crate::error::{Error, ParseError, ResourceLimitError, UnsupportedError};
use crate::limits;
use crate::parse_util;
use crate::statement::Statement;
use crate::strict_json;

/// The only `mediaType` this crate accepts for a Sigstore bundle.
pub const BUNDLE_MEDIA_TYPE: &str = "application/vnd.dev.sigstore.bundle.v0.3+json";

// ---------------------------------------------------------------------
// Public, hardened data model
// ---------------------------------------------------------------------

/// A parsed and structurally-hardened Sigstore bundle.
///
/// Parsing enforces `mediaType`, strict base64/hex on every binary field,
/// strict integer parsing on protojson's stringified `int64` fields, and
/// exactly one DSSE signature. It does **not** verify anything
/// cryptographically — see [`crate::Verifier`].
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Always [`BUNDLE_MEDIA_TYPE`]; kept on the struct so callers and
    /// logs can see what was validated without a separate lookup.
    pub media_type: String,
    /// The certificate and transparency-log evidence bundled with the
    /// signature.
    pub verification_material: VerificationMaterial,
    /// The signed envelope wrapping the in-toto statement.
    pub dsse_envelope: DsseEnvelope,
}

impl Bundle {
    /// Parses exactly one Sigstore bundle from `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitError::InputTooLarge`] if `bytes` exceeds the
    /// crate's input-size limit, [`UnsupportedError::MediaType`] if
    /// `mediaType` is not [`BUNDLE_MEDIA_TYPE`], [`ParseError::Json`] for
    /// malformed or structurally-wrong JSON (including duplicate object
    /// keys), and other [`ParseError`] variants for fields that fail
    /// strict decoding.
    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        parse_util::check_input_size(bytes)?;
        let value = strict_json::parse_strict(bytes)?;
        let raw: RawBundle =
            serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawBundle) -> Result<Self, Error> {
        let RawBundle {
            media_type,
            verification_material,
            dsse_envelope,
        } = raw;
        if media_type != BUNDLE_MEDIA_TYPE {
            return Err(Error::Unsupported(UnsupportedError::MediaType {
                found: media_type,
            }));
        }
        let verification_material = VerificationMaterial::from_raw(verification_material)?;
        let dsse_envelope = DsseEnvelope::from_raw(dsse_envelope)?;
        Ok(Bundle {
            media_type,
            verification_material,
            dsse_envelope,
        })
    }

    /// Decodes and parses the DSSE envelope's payload as an in-toto
    /// [`Statement`].
    ///
    /// This is a pure parsing step — it does not check the DSSE signature
    /// or otherwise authenticate the payload. See [`crate::Verifier`] for
    /// verification.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] or [`UnsupportedError`] if the payload is
    /// not a well-formed, supported in-toto statement.
    pub fn statement(&self) -> Result<Statement, Error> {
        Statement::from_payload(&self.dsse_envelope.payload)
    }
}

/// The certificate and transparency-log evidence carried in a bundle
/// (`verificationMaterial` in the protojson).
#[derive(Debug, Clone)]
pub struct VerificationMaterial {
    /// The signer's leaf certificate, raw DER bytes.
    pub certificate: Certificate,
    /// Rekor transparency-log entries for this bundle. Empty for
    /// GitHub-initiated release attestations, which use an RFC 3161
    /// timestamp instead.
    pub tlog_entries: Vec<TlogEntry>,
    /// RFC 3161 timestamp-authority evidence, if any.
    pub timestamp_verification_data: TimestampVerificationData,
}

impl VerificationMaterial {
    fn from_raw(raw: RawVerificationMaterial) -> Result<Self, Error> {
        let RawVerificationMaterial {
            certificate,
            tlog_entries,
            timestamp_verification_data,
        } = raw;
        parse_util::check_count(&tlog_entries, limits::MAX_TLOG_ENTRIES, |actual, limit| {
            ResourceLimitError::TooManyTlogEntries { actual, limit }
        })?;
        let certificate = Certificate::from_raw(&certificate)?;
        let tlog_entries = tlog_entries
            .into_iter()
            .map(TlogEntry::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        let timestamp_verification_data =
            TimestampVerificationData::from_raw(&timestamp_verification_data)?;
        Ok(VerificationMaterial {
            certificate,
            tlog_entries,
            timestamp_verification_data,
        })
    }
}

/// A leaf certificate, as raw DER bytes (`certificate.rawBytes`).
///
/// No X.509 parsing happens in this crate yet; the bytes are only
/// base64-decoded and held opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    /// DER-encoded certificate bytes.
    pub raw_bytes: Vec<u8>,
}

impl Certificate {
    fn from_raw(raw: &RawCertificate) -> Result<Self, Error> {
        let raw_bytes = parse_util::strict_base64("certificate.rawBytes", &raw.raw_bytes)?;
        Ok(Certificate { raw_bytes })
    }
}

/// One Rekor transparency-log entry (`tlogEntries[]`).
#[derive(Debug, Clone)]
pub struct TlogEntry {
    /// The entry's global index in the log.
    pub log_index: u64,
    /// The log's key id (`logId.keyId`), base64-decoded.
    pub log_id_key_id: Vec<u8>,
    /// The Rekor entry kind, e.g. `"dsse"`.
    pub kind: String,
    /// The Rekor entry kind's version, e.g. `"0.0.1"`.
    pub version: String,
    /// Unix seconds at which the entry was integrated into the log, per
    /// the log itself. Authenticated only by the inclusion promise (SET),
    /// not by the inclusion proof alone — see DESIGN.md "Time-evidence
    /// model".
    pub integrated_time: u64,
    /// The log's signed promise to include this entry (SET), if present.
    pub inclusion_promise: Option<InclusionPromise>,
    /// The Merkle inclusion proof against a checkpoint, if present.
    pub inclusion_proof: Option<InclusionProof>,
    /// The canonicalized Rekor entry body, base64-decoded.
    pub canonicalized_body: Vec<u8>,
}

impl TlogEntry {
    fn from_raw(raw: RawTlogEntry) -> Result<Self, Error> {
        let RawTlogEntry {
            log_index,
            log_id,
            kind_version,
            integrated_time,
            inclusion_promise,
            inclusion_proof,
            canonicalized_body,
        } = raw;
        let log_index = parse_util::strict_stringified_u64("tlogEntries[].logIndex", &log_index)?;
        let log_id_key_id = parse_util::strict_base64("tlogEntries[].logId.keyId", &log_id.key_id)?;
        let integrated_time =
            parse_util::strict_stringified_u64("tlogEntries[].integratedTime", &integrated_time)?;
        let inclusion_promise = inclusion_promise
            .as_ref()
            .map(InclusionPromise::from_raw)
            .transpose()?;
        let inclusion_proof = inclusion_proof.map(InclusionProof::from_raw).transpose()?;
        let canonicalized_body =
            parse_util::strict_base64("tlogEntries[].canonicalizedBody", &canonicalized_body)?;
        Ok(TlogEntry {
            log_index,
            log_id_key_id,
            kind: kind_version.kind,
            version: kind_version.version,
            integrated_time,
            inclusion_promise,
            inclusion_proof,
            canonicalized_body,
        })
    }
}

/// The log's signed promise (SET) to include an entry
/// (`inclusionPromise`).
#[derive(Debug, Clone)]
pub struct InclusionPromise {
    /// The signed entry timestamp, base64-decoded.
    pub signed_entry_timestamp: Vec<u8>,
}

impl InclusionPromise {
    fn from_raw(raw: &RawInclusionPromise) -> Result<Self, Error> {
        let signed_entry_timestamp = parse_util::strict_base64(
            "inclusionPromise.signedEntryTimestamp",
            &raw.signed_entry_timestamp,
        )?;
        Ok(InclusionPromise {
            signed_entry_timestamp,
        })
    }
}

/// A Merkle inclusion proof against a signed checkpoint
/// (`inclusionProof`).
#[derive(Debug, Clone)]
pub struct InclusionProof {
    /// The leaf's index in the tree.
    pub log_index: u64,
    /// The Merkle tree root hash, base64-decoded.
    pub root_hash: Vec<u8>,
    /// The tree size the proof was computed against.
    pub tree_size: u64,
    /// The proof's sibling hashes, each base64-decoded.
    pub hashes: Vec<Vec<u8>>,
    /// The signed checkpoint this proof is anchored to.
    pub checkpoint: Checkpoint,
}

impl InclusionProof {
    fn from_raw(raw: RawInclusionProof) -> Result<Self, Error> {
        let RawInclusionProof {
            log_index,
            root_hash,
            tree_size,
            hashes,
            checkpoint,
        } = raw;
        let log_index = parse_util::strict_stringified_u64("inclusionProof.logIndex", &log_index)?;
        let root_hash = parse_util::strict_base64("inclusionProof.rootHash", &root_hash)?;
        let tree_size = parse_util::strict_stringified_u64("inclusionProof.treeSize", &tree_size)?;
        let hashes = hashes
            .iter()
            .map(|h| parse_util::strict_base64("inclusionProof.hashes[]", h))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(InclusionProof {
            log_index,
            root_hash,
            tree_size,
            hashes,
            checkpoint: Checkpoint {
                envelope: checkpoint.envelope,
            },
        })
    }
}

/// A signed checkpoint (signed-note) anchoring an inclusion proof.
///
/// The envelope text is kept as an opaque string; parsing the
/// checkpoint/signed-note format itself is future verification work, not
/// part of this crate's parsing layer.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// The raw signed-note text.
    pub envelope: String,
}

/// RFC 3161 timestamp-authority evidence carried in a bundle
/// (`timestampVerificationData`).
#[derive(Debug, Clone, Default)]
pub struct TimestampVerificationData {
    /// RFC 3161 timestamp tokens, if any. GitHub-initiated release
    /// attestations carry exactly one and no tlog entries; workflow
    /// attestations currently carry none.
    pub rfc3161_timestamps: Vec<Rfc3161Timestamp>,
}

impl TimestampVerificationData {
    fn from_raw(raw: &RawTimestampVerificationData) -> Result<Self, Error> {
        let rfc3161_timestamps = raw
            .rfc3161_timestamps
            .iter()
            .map(Rfc3161Timestamp::from_raw)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TimestampVerificationData { rfc3161_timestamps })
    }
}

/// One RFC 3161 timestamp token (`rfc3161Timestamps[]`).
#[derive(Debug, Clone)]
pub struct Rfc3161Timestamp {
    /// The DER-encoded timestamp token, base64-decoded.
    pub signed_timestamp: Vec<u8>,
}

impl Rfc3161Timestamp {
    fn from_raw(raw: &RawRfc3161Timestamp) -> Result<Self, Error> {
        let signed_timestamp = parse_util::strict_base64(
            "rfc3161Timestamps[].signedTimestamp",
            &raw.signed_timestamp,
        )?;
        Ok(Rfc3161Timestamp { signed_timestamp })
    }
}

/// A DSSE (Dead Simple Signing Envelope) wrapping the in-toto statement
/// (`dsseEnvelope`).
#[derive(Debug, Clone)]
pub struct DsseEnvelope {
    /// The base64-decoded payload bytes (an in-toto statement, for the
    /// statements this crate understands).
    pub payload: Vec<u8>,
    /// The payload's media type. Parsing accepts any string by design --
    /// which type is acceptable is a verification decision, not a shape
    /// question -- but [`crate::Verifier::verify_digest`] requires
    /// `"application/vnd.in-toto+json"` before interpreting the payload,
    /// since the type is covered by the DSSE PAE signature.
    pub payload_type: String,
    /// The envelope's one signature. Bundles with zero or more than one
    /// signature are rejected at parse time
    /// ([`ParseError::DsseSignatureCount`]) rather than modeled as a list,
    /// since exactly one is the only shape this crate ever accepts.
    pub signature: DsseSignature,
}

impl DsseEnvelope {
    fn from_raw(raw: RawDsseEnvelope) -> Result<Self, Error> {
        let RawDsseEnvelope {
            payload,
            payload_type,
            signatures,
        } = raw;
        let [raw_signature] = <[RawDsseSignature; 1]>::try_from(signatures)
            .map_err(|v| Error::Parse(ParseError::DsseSignatureCount { count: v.len() }))?;
        let payload = parse_util::strict_base64("dsseEnvelope.payload", &payload)?;
        let signature = DsseSignature::from_raw(raw_signature)?;
        Ok(DsseEnvelope {
            payload,
            payload_type,
            signature,
        })
    }
}

/// One DSSE signature (`dsseEnvelope.signatures[]`).
#[derive(Debug, Clone)]
pub struct DsseSignature {
    /// The signature bytes, base64-decoded.
    pub sig: Vec<u8>,
    /// The signing key's id, if the envelope carries one.
    pub keyid: Option<String>,
}

impl DsseSignature {
    fn from_raw(raw: RawDsseSignature) -> Result<Self, Error> {
        let sig = parse_util::strict_base64("dsseEnvelope.signatures[].sig", &raw.sig)?;
        Ok(DsseSignature {
            sig,
            keyid: raw.keyid,
        })
    }
}

// ---------------------------------------------------------------------
// BundleSet: the two multi-bundle container shapes
// ---------------------------------------------------------------------

/// A collection of bundles, parsed from one of the two container shapes
/// GitHub actually serves: `gh attestation download`'s JSONL output
/// ([`BundleSet::from_json_lines`]) or the attestations API response
/// ([`BundleSet::from_github_response`]).
///
/// There is deliberately no "sniff the input and pick a container format"
/// constructor — the caller knows which one it has.
#[derive(Debug, Clone)]
pub struct BundleSet {
    /// Every inline bundle found, in input order.
    pub bundles: Vec<Bundle>,
    /// Per-entry acquisition metadata. Populated only by
    /// [`BundleSet::from_github_response`]; always empty for
    /// [`BundleSet::from_json_lines`], which has no such metadata to
    /// carry.
    pub entries: Vec<AttestationEntry>,
}

/// One entry of a GitHub attestations API response
/// (`GET /repos/{owner}/{repo}/attestations/sha256:{digest}`).
///
/// As of 2026-07 the API no longer inlines bundle JSON: entries carry a
/// short-lived `bundle_url` instead, and fetching/decompressing that is
/// out of scope for this sans-io crate (see DESIGN.md). `bundle` is
/// `Some` only on the rare/future response that does inline it.
#[derive(Debug, Clone)]
pub struct AttestationEntry {
    /// Who produced the attestation: `"github"` or `"user"` in observed
    /// responses. Kept as an open string rather than an enum since the
    /// API is understood to evolve.
    pub initiator: String,
    /// The numeric repository id the attestation belongs to.
    pub repository_id: Option<u64>,
    /// A short-lived signed URL serving the (raw-snappy-compressed) bundle
    /// JSON, when the bundle is not inline.
    pub bundle_url: Option<String>,
    /// The inline bundle, when the response provides one.
    pub bundle: Option<Bundle>,
}

impl BundleSet {
    /// Parses `gh attestation download`'s JSONL output: one complete
    /// bundle JSON object per line. Blank lines are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitError::InputTooLarge`] if `bytes` exceeds
    /// the input-size limit, [`ParseError::EmptyContainer`] if no line
    /// yields a bundle, [`ResourceLimitError::TooManyBundles`] if more
    /// than the per-set limit parse, and the usual [`Bundle::from_json`]
    /// errors for any individual line.
    pub fn from_json_lines(bytes: &[u8]) -> Result<Self, Error> {
        parse_util::check_input_size(bytes)?;

        let mut bundles = Vec::new();
        for line in bytes.split(|&b| b == b'\n') {
            let trimmed = line.trim_ascii();
            if trimmed.is_empty() {
                continue;
            }
            let value = strict_json::parse_strict(trimmed)?;
            let raw: RawBundle =
                serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;
            bundles.push(Bundle::from_raw(raw)?);
        }

        if bundles.is_empty() {
            return Err(Error::Parse(ParseError::EmptyContainer(
                "BundleSet: no bundle lines in JSONL input",
            )));
        }
        parse_util::check_count(&bundles, limits::MAX_BUNDLES_PER_SET, |actual, limit| {
            ResourceLimitError::TooManyBundles { actual, limit }
        })?;

        Ok(BundleSet {
            bundles,
            entries: Vec::new(),
        })
    }

    /// Parses a GitHub attestations API response. Collects any inline
    /// bundles and every entry's acquisition metadata.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::EmptyContainer`] if the response has zero
    /// attestation entries, [`UnsupportedError::BundlesNotInline`] if it
    /// has entries but none carries an inline bundle (the current GitHub
    /// API shape — see `tests/fixtures/README.md`), and the usual size,
    /// count, and [`Bundle::from_json`]-equivalent errors otherwise.
    pub fn from_github_response(bytes: &[u8]) -> Result<Self, Error> {
        parse_util::check_input_size(bytes)?;
        let value = strict_json::parse_strict(bytes)?;
        let raw: RawGithubResponse =
            serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;

        if raw.attestations.is_empty() {
            return Err(Error::Parse(ParseError::EmptyContainer(
                "attestations API response: no attestation entries",
            )));
        }
        parse_util::check_count(
            &raw.attestations,
            limits::MAX_BUNDLES_PER_SET,
            |actual, limit| ResourceLimitError::TooManyBundles { actual, limit },
        )?;

        let mut entries = Vec::with_capacity(raw.attestations.len());
        let mut bundles = Vec::new();
        for raw_entry in raw.attestations {
            let bundle = raw_entry
                .bundle
                .map(|v| {
                    let raw_bundle: RawBundle = serde_json::from_value(v)
                        .map_err(|e| Error::Parse(ParseError::Json(e.to_string())))?;
                    Bundle::from_raw(raw_bundle)
                })
                .transpose()?;
            if let Some(bundle) = &bundle {
                bundles.push(bundle.clone());
            }
            entries.push(AttestationEntry {
                initiator: raw_entry.initiator,
                repository_id: raw_entry.repository_id,
                bundle_url: raw_entry.bundle_url,
                bundle,
            });
        }

        if bundles.is_empty() {
            return Err(Error::Unsupported(UnsupportedError::BundlesNotInline));
        }

        Ok(BundleSet { bundles, entries })
    }
}

// ---------------------------------------------------------------------
// Raw (untrusted-shape) mirrors: plain serde derive, tolerant of unknown
// fields, strictness applied on conversion to the public types above.
// ---------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawBundle {
    media_type: String,
    verification_material: RawVerificationMaterial,
    dsse_envelope: RawDsseEnvelope,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVerificationMaterial {
    certificate: RawCertificate,
    #[serde(default)]
    tlog_entries: Vec<RawTlogEntry>,
    #[serde(default)]
    timestamp_verification_data: RawTimestampVerificationData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCertificate {
    raw_bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTlogEntry {
    log_index: String,
    log_id: RawLogId,
    kind_version: RawKindVersion,
    integrated_time: String,
    #[serde(default)]
    inclusion_promise: Option<RawInclusionPromise>,
    #[serde(default)]
    inclusion_proof: Option<RawInclusionProof>,
    canonicalized_body: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLogId {
    key_id: String,
}

#[derive(Deserialize)]
struct RawKindVersion {
    kind: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInclusionPromise {
    signed_entry_timestamp: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInclusionProof {
    log_index: String,
    root_hash: String,
    tree_size: String,
    #[serde(default)]
    hashes: Vec<String>,
    checkpoint: RawCheckpoint,
}

#[derive(Deserialize)]
struct RawCheckpoint {
    envelope: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RawTimestampVerificationData {
    #[serde(default)]
    rfc3161_timestamps: Vec<RawRfc3161Timestamp>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRfc3161Timestamp {
    signed_timestamp: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDsseEnvelope {
    payload: String,
    payload_type: String,
    #[serde(default)]
    signatures: Vec<RawDsseSignature>,
}

#[derive(Deserialize)]
struct RawDsseSignature {
    sig: String,
    #[serde(default)]
    keyid: Option<String>,
}

#[derive(Deserialize)]
struct RawGithubResponse {
    attestations: Vec<RawAttestationEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawAttestationEntry {
    initiator: String,
    #[serde(default)]
    repository_id: Option<u64>,
    #[serde(default)]
    bundle_url: Option<String>,
    #[serde(default)]
    bundle: Option<serde_json::Value>,
}
