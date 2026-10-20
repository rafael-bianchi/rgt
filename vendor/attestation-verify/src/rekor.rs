//! Rekor v1 transparency-log entry verification.
//!
//! Pure verification functions implementing `DESIGN.md` "Time-evidence
//! model" steps 1-5, composed by [`verify_tlog_entry`] and run as step 5
//! of [`crate::Verifier::verify_digest`]'s chain. Every check here is
//! independently unit-tested; see [`verify_tlog_entry`] for how they
//! compose.
//!
//! ## The canonicalized body
//!
//! `tlogEntries[].canonicalizedBody` (base64) decodes to a Rekor `dsse`
//! kind, version `0.0.1` entry body:
//!
//! ```json
//! {
//!   "apiVersion": "0.0.1",
//!   "kind": "dsse",
//!   "spec": {
//!     "envelopeHash": {"algorithm": "sha256", "value": "<hex>"},
//!     "payloadHash": {"algorithm": "sha256", "value": "<hex>"},
//!     "signatures": [{"signature": "<base64>", "verifier": "<base64 PEM>"}]
//!   }
//! }
//! ```
//!
//! `signatures[].verifier` is base64 of PEM certificate *text* (not raw
//! DER directly) — a second, nested layer of encoding. `envelopeHash` is
//! not a documented interoperable canonicalization; see
//! [`compute_envelope_hash`] for how it was reverse-engineered against
//! the real golden fixture.
//!
//! ## The checkpoint (signed note)
//!
//! `inclusionProof.checkpoint.envelope` is a c2sp.org-style signed note:
//! `<origin>\n<treeSize>\n<base64 rootHash>\n`, optional extension lines,
//! a blank line, then one or more `— <name> <base64>` signature lines.
//! The real fixture's signatures are ASN.1 DER-encoded ECDSA (not raw
//! 64-byte r||s) behind a 4-byte key hint. The hint is attacker-controlled
//! candidate metadata used to avoid trying unrelated keys; the signature-line
//! name is an attacker-controlled priority hint used to try likely candidates
//! first. Neither field authenticates a deployment or log identity. The
//! origin is retained from the signed body and checked against the caller's
//! [`CheckpointOriginPolicy`] only after the selected key verifies it.

use sha2::{Digest, Sha256};

use crate::bundle::{Bundle, DsseEnvelope, InclusionProof, TlogEntry};
use crate::dsse::{EcdsaVerifyingKey, LeafCertificateInfo};
use crate::error::{
    ContentBindingError, Error, ParseError, ResourceLimitError, TimestampError, TransparencyError,
    UnsupportedError,
};
use crate::limits;
use crate::parse_util;
use crate::policy::CheckpointOriginPolicy;
use crate::strict_json;
use crate::trust::{TransparencyLog, TrustStore, ValidityPeriod};

/// The only Rekor entry `kind` this crate models.
const TLOG_ENTRY_KIND: &str = "dsse";
/// The only Rekor `dsse`-kind `apiVersion` this crate models.
///
/// Rekor v2 entries use a different kind/version pair entirely (and
/// Ed25519 signatures); they are rejected by the kind/version check
/// below with a typed [`UnsupportedError`], never silently
/// misinterpreted. See `DESIGN.md` "Rekor v1 / v2 scope".
// TODO(DESIGN.md "Rekor v1 / v2 scope", v0.1.x): implement Rekor v2
// (Ed25519, tiles/sharded log) verification. Mandatory eventually per
// DESIGN.md; out of scope for this task.
const TLOG_ENTRY_API_VERSION: &str = "0.0.1";

// ---------------------------------------------------------------------
// 1. Canonicalized body model + binding
// ---------------------------------------------------------------------

/// A parsed and structurally-hardened Rekor `dsse`/`0.0.1` entry body
/// (the decoded `canonicalizedBody`).
#[derive(Debug, Clone)]
pub(crate) struct TlogEntryBody {
    /// Always [`TLOG_ENTRY_KIND`].
    pub(crate) kind: String,
    /// Always [`TLOG_ENTRY_API_VERSION`].
    pub(crate) api_version: String,
    /// `spec.envelopeHash.value`, strict hex-decoded.
    pub(crate) envelope_hash: [u8; 32],
    /// `spec.payloadHash.value`, strict hex-decoded.
    pub(crate) payload_hash: [u8; 32],
    /// `spec.signatures[0].signature`, base64-decoded.
    pub(crate) signature: Vec<u8>,
    /// `spec.signatures[0].verifier`: base64 of PEM certificate text,
    /// decoded here down to raw DER bytes.
    pub(crate) verifier_certificate_der: Vec<u8>,
}

/// Parses a Rekor entry body from `canonicalized_body` (the *decoded*
/// bytes of `tlogEntries[].canonicalizedBody`; callers pass the bytes,
/// not the base64 string).
///
/// # Errors
///
/// Returns [`ParseError::Json`] for malformed JSON (including duplicate
/// keys), [`UnsupportedError::TlogEntryKindVersion`] if `kind`/
/// `apiVersion` is not the `dsse`/`0.0.1` pair this crate implements,
/// [`ParseError::DsseSignatureCount`] if `spec.signatures` does not have
/// exactly one entry, and other [`ParseError`] variants for fields that
/// fail strict decoding.
pub(crate) fn parse_tlog_entry_body(canonicalized_body: &[u8]) -> Result<TlogEntryBody, Error> {
    let value = strict_json::parse_strict(canonicalized_body)?;
    let raw: RawTlogEntryBody =
        serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;

    if raw.kind != TLOG_ENTRY_KIND || raw.api_version != TLOG_ENTRY_API_VERSION {
        return Err(Error::Unsupported(UnsupportedError::TlogEntryKindVersion {
            kind: raw.kind,
            version: raw.api_version,
        }));
    }

    let [raw_signature] = <[RawDsseSpecSignature; 1]>::try_from(raw.spec.signatures)
        .map_err(|v| Error::Parse(ParseError::DsseSignatureCount { count: v.len() }))?;

    let envelope_hash =
        parse_util::strict_hex("spec.envelopeHash.value", &raw.spec.envelope_hash.value)?;
    let payload_hash =
        parse_util::strict_hex("spec.payloadHash.value", &raw.spec.payload_hash.value)?;
    let signature =
        parse_util::strict_base64("spec.signatures[].signature", &raw_signature.signature)?;
    let verifier_pem_bytes =
        parse_util::strict_base64("spec.signatures[].verifier", &raw_signature.verifier)?;
    let verifier_pem_text = String::from_utf8(verifier_pem_bytes).map_err(|_| {
        Error::Parse(ParseError::Base64 {
            field: "spec.signatures[].verifier",
            reason: "decoded PEM body is not valid UTF-8".to_owned(),
        })
    })?;
    let verifier_certificate_der = parse_util::strict_pem(
        "spec.signatures[].verifier",
        "CERTIFICATE",
        &verifier_pem_text,
    )?;

    Ok(TlogEntryBody {
        kind: raw.kind,
        api_version: raw.api_version,
        envelope_hash,
        payload_hash,
        signature,
        verifier_certificate_der,
    })
}

/// Rekor's `dsse` entry type records `envelopeHash` as the SHA-256 of the
/// *original submitter's* DSSE envelope object, serialized the way Go's
/// `encoding/json.Marshal` on a `payload`/`payloadType`/`signatures`
/// struct (with `signatures[].keyid` tagged `omitempty`) would: compact
/// (no extra whitespace), fields in that fixed struct order, `keyid`
/// entirely omitted when empty or absent.
///
/// This is not a documented interoperable canonicalization — it was
/// reverse-engineered by cross-checking candidate serializations against
/// `envelopeHash.value` in the real golden fixture
/// (`tests/fixtures/github-cli/tarball-user-slsa-provenance.json`) until
/// the SHA-256 matched bit-for-bit (see this module's tests).
fn compute_envelope_hash(envelope: &DsseEnvelope) -> [u8; 32] {
    compute_envelope_hash_with_empty_keyid(envelope, false)
}

// GitHub's JS submitter includes an empty keyid while the Go submitter omits
// it. Bundle serialization can erase that distinction. Both serializations
// must still match the authenticated log body's hash; no signed bytes change.
fn compute_envelope_hash_with_empty_keyid(
    envelope: &DsseEnvelope,
    include_empty_keyid: bool,
) -> [u8; 32] {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let mut json = String::from("{\"payload\":");
    json.push_str(&json_string(&STANDARD.encode(&envelope.payload)));
    json.push_str(",\"payloadType\":");
    json.push_str(&json_string(&envelope.payload_type));
    json.push_str(",\"signatures\":[{");
    if let Some(keyid) = envelope
        .signature
        .keyid
        .as_deref()
        .filter(|k| !k.is_empty())
    {
        json.push_str("\"keyid\":");
        json.push_str(&json_string(keyid));
        json.push(',');
    }
    json.push_str("\"sig\":");
    json.push_str(&json_string(&STANDARD.encode(&envelope.signature.sig)));
    if include_empty_keyid {
        json.push_str(",\"keyid\":\"\"");
    }
    json.push_str("}]}");

    Sha256::digest(json.as_bytes()).into()
}

/// JSON-string-escapes `s` (quotes, backslashes, control characters),
/// via `serde_json`'s `Value::String` `Display` impl — infallible for a
/// valid Rust `&str`, so this needs no error handling of its own.
fn json_string(s: &str) -> String {
    serde_json::Value::String(s.to_owned()).to_string()
}

/// Binds a parsed Rekor entry body to the bundle it was extracted from:
/// every check in `DESIGN.md` "Rekor-entry <-> bundle binding" that this
/// task's scope covers.
///
/// # Errors
///
/// Returns a distinct [`ContentBindingError`] variant for whichever
/// binding fails first: `kindVersion`, DSSE signature, verifier
/// certificate, payload hash, then envelope hash.
pub(crate) fn check_entry_binding(
    tlog_entry: &TlogEntry,
    body: &TlogEntryBody,
    envelope: &DsseEnvelope,
    leaf_certificate_der: &[u8],
) -> Result<(), Error> {
    if tlog_entry.kind != body.kind || tlog_entry.version != body.api_version {
        return Err(Error::ContentBinding(
            ContentBindingError::TlogEntryKindVersionMismatch,
        ));
    }
    if body.signature != envelope.signature.sig {
        return Err(Error::ContentBinding(
            ContentBindingError::TlogEntrySignatureMismatch,
        ));
    }
    if body.verifier_certificate_der != leaf_certificate_der {
        return Err(Error::ContentBinding(
            ContentBindingError::TlogEntryCertificateMismatch,
        ));
    }
    let computed_payload_hash: [u8; 32] = Sha256::digest(&envelope.payload).into();
    if body.payload_hash != computed_payload_hash {
        return Err(Error::ContentBinding(
            ContentBindingError::TlogEntryPayloadHashMismatch,
        ));
    }
    let empty_keyid = envelope
        .signature
        .keyid
        .as_deref()
        .is_none_or(str::is_empty);
    if body.envelope_hash != compute_envelope_hash(envelope)
        && !(empty_keyid
            && body.envelope_hash == compute_envelope_hash_with_empty_keyid(envelope, true))
    {
        return Err(Error::ContentBinding(
            ContentBindingError::TlogEntryEnvelopeHashMismatch,
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// 2. SET (inclusion promise) verification
// ---------------------------------------------------------------------

/// Builds the exact byte string the log's inclusion promise (SET) is an
/// ECDSA-P256-SHA256 signature over: the compact JSON object
/// `{"body":"<base64 canonicalizedBody>","integratedTime":<int>,"logID":"<lowercase hex>","logIndex":<int>}`,
/// with this fixed field order. This is RFC 8785-style canonical JSON
/// (sorted keys, minimal separators) for this one closed field set, hand
/// -built rather than via a general JCS library — confirmed against the
/// real fixture and the real public-good log key (see this module's
/// tests).
fn set_signed_bytes(
    canonicalized_body: &[u8],
    integrated_time: u64,
    log_key_id: &[u8],
    log_index: u64,
) -> Vec<u8> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    format!(
        "{{\"body\":\"{}\",\"integratedTime\":{integrated_time},\"logID\":\"{}\",\"logIndex\":{log_index}}}",
        STANDARD.encode(canonicalized_body),
        hex::encode(log_key_id),
    )
    .into_bytes()
}

/// Verifies a tlog entry's inclusion promise (SET) against `log_key`.
///
/// A SET is required: an inclusion proof alone does not authenticate
/// `integratedTime` (`DESIGN.md` "Time-evidence model"), so a missing SET
/// is an error, not a skipped check.
///
/// # Parameters
///
/// * `log_key_id` — the SHA-256 of the selected trusted key's SPKI bytes,
///   recomputed from key material rather than read from any label. This
///   matches [`select_log_key`]'s hygiene of never trusting the trust
///   store's own pre-parsed `logId.keyId` label.
///
/// # Errors
///
/// Returns [`TransparencyError::SetMissing`] if the entry carries no
/// inclusion promise, and [`TransparencyError::SetInvalid`] if it does
/// not verify.
pub(crate) fn verify_set(
    tlog_entry: &TlogEntry,
    log_key_id: &[u8],
    log_key: &EcdsaVerifyingKey,
) -> Result<(), Error> {
    let Some(promise) = &tlog_entry.inclusion_promise else {
        return Err(Error::Transparency(TransparencyError::SetMissing));
    };
    let message = set_signed_bytes(
        &tlog_entry.canonicalized_body,
        tlog_entry.integrated_time,
        log_key_id,
        tlog_entry.log_index,
    );
    if log_key.verify_der(&message, &promise.signed_entry_timestamp) {
        Ok(())
    } else {
        Err(Error::Transparency(TransparencyError::SetInvalid))
    }
}

/// Selects the trust store's transparency log whose `SubjectPublicKeyInfo`
/// hashes (SHA-256) to `wanted_log_id` — freshly computed from each
/// candidate's key material rather than trusting the trust store's own
/// pre-parsed `logId.keyId` label, and rather than trusting the bundle's
/// claim about which log signed it any further than "some trusted key
/// hashes to this id."
///
/// # Errors
///
/// Returns [`TransparencyError::UnknownLogKey`] if no trust-store log
/// matches, and whatever [`EcdsaVerifyingKey::from_spki_der`] returns if
/// the matched log's key uses an algorithm this crate does not
/// implement.
pub(crate) fn select_log_key<'a>(
    trust_store: &'a TrustStore,
    wanted_log_id: &[u8],
) -> Result<(&'a TransparencyLog, EcdsaVerifyingKey), Error> {
    let log = trust_store
        .tlogs
        .iter()
        .find(|log| Sha256::digest(&log.public_key.raw_bytes).as_slice() == wanted_log_id)
        .ok_or(Error::Transparency(TransparencyError::UnknownLogKey))?;
    let key = EcdsaVerifyingKey::from_spki_der(&log.public_key.raw_bytes)?;
    Ok((log, key))
}

// ---------------------------------------------------------------------
// 3. Merkle inclusion proof (RFC 6962)
// ---------------------------------------------------------------------

/// RFC 6962 domain-separation prefix for leaf hashes.
const LEAF_HASH_PREFIX: u8 = 0x00;
/// RFC 6962 domain-separation prefix for interior node hashes.
const NODE_HASH_PREFIX: u8 = 0x01;

fn rfc6962_leaf_hash(canonicalized_body: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([LEAF_HASH_PREFIX]);
    hasher.update(canonicalized_body);
    hasher.finalize().into()
}

fn rfc6962_node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([NODE_HASH_PREFIX]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Recomputes a Merkle tree root from an inclusion proof, following the
/// standard RFC 6962 / Trillian audit-path algorithm: walk from the leaf
/// toward the root, consuming one proof hash per level except where the
/// current node is the unpaired rightmost node at its level (which
/// promotes to the next level unchanged, consuming nothing).
///
/// Returns `None` if `leaf_index >= tree_size`, if the proof runs out of
/// hashes before reaching the root (too short), or if hashes remain
/// unused once the root is reached (too long) — the caller turns any
/// `None` into [`TransparencyError::InclusionProofInvalid`].
fn recompute_root(
    leaf_hash: [u8; 32],
    leaf_index: u64,
    tree_size: u64,
    hashes: &[[u8; 32]],
) -> Option<[u8; 32]> {
    if leaf_index >= tree_size {
        return None;
    }
    let mut node_index = leaf_index;
    let mut last_node = tree_size - 1;
    let mut node_hash = leaf_hash;
    let mut remaining = hashes.iter();

    while last_node > 0 {
        if node_index % 2 == 1 {
            node_hash = rfc6962_node_hash(remaining.next()?, &node_hash);
        } else if node_index < last_node {
            node_hash = rfc6962_node_hash(&node_hash, remaining.next()?);
        }
        // else: node_index == last_node and even — the unpaired
        // rightmost node at this level, promoted unchanged.
        node_index /= 2;
        last_node /= 2;
    }

    if remaining.next().is_some() {
        return None;
    }
    Some(node_hash)
}

/// Verifies a Merkle inclusion proof: recomputes the tree root from
/// `canonicalized_body` and `proof`, and requires it to equal
/// `proof.root_hash`.
///
/// # Errors
///
/// Returns [`TransparencyError::InclusionProofInvalid`] if any hash in
/// `proof` (root or sibling) is not exactly 32 bytes, if
/// `proof.log_index >= proof.tree_size`, if the proof is the wrong
/// length (too few or too many hashes) for that index/size pair, or if
/// the recomputed root does not match.
pub(crate) fn verify_inclusion_proof(
    canonicalized_body: &[u8],
    proof: &InclusionProof,
) -> Result<(), Error> {
    let invalid = || Error::Transparency(TransparencyError::InclusionProofInvalid);

    let root_hash: [u8; 32] = proof
        .root_hash
        .as_slice()
        .try_into()
        .map_err(|_| invalid())?;
    let hashes = proof
        .hashes
        .iter()
        .map(|h| <[u8; 32]>::try_from(h.as_slice()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;

    let leaf_hash = rfc6962_leaf_hash(canonicalized_body);
    let computed =
        recompute_root(leaf_hash, proof.log_index, proof.tree_size, &hashes).ok_or_else(invalid)?;

    if computed == root_hash {
        Ok(())
    } else {
        Err(invalid())
    }
}

// ---------------------------------------------------------------------
// 4. Checkpoint (signed note)
// ---------------------------------------------------------------------

/// The `"— "` (U+2014 EM DASH, U+0020 SPACE) marker c2sp.org-style signed
/// notes use to prefix each signature line.
const CHECKPOINT_SIGNATURE_PREFIX: &str = "\u{2014} ";

/// The truncated key identifier carried ahead of each signed-note signature.
const CHECKPOINT_KEY_HINT_BYTES: usize = 4;

#[derive(Debug)]
struct CheckpointSignature<'a> {
    /// The attacker-controlled signature-line key name. It is not covered by
    /// the signed note and is used only to prioritize candidates
    /// (`DESIGN.md` "Rekor-entry <-> bundle binding").
    name: &'a str,
    key_hint: [u8; CHECKPOINT_KEY_HINT_BYTES],
    signature: Vec<u8>,
}

/// A parsed checkpoint (signed note) envelope.
#[derive(Debug)]
pub(crate) struct ParsedCheckpoint<'a> {
    /// The opaque origin line from the signed note body.  It is retained
    /// byte-for-byte (apart from UTF-8's inherent representation) so the
    /// caller can compare it only after signature authentication.
    origin: &'a str,
    tree_size: u64,
    root_hash: Vec<u8>,
    /// The note body: `<origin>\n<treeSize>\n<rootHash>\n` plus any
    /// extension lines, through the final newline before the blank
    /// line. This exact substring is what each signature is computed
    /// over.
    note_body: &'a str,
    /// Parsed signature lines. Verification uses the public 4-byte key hint
    /// to select candidates and the unauthenticated name to order them before
    /// expensive cryptographic work; neither value authenticates log identity.
    signatures: Vec<CheckpointSignature<'a>>,
}

pub(crate) fn parse_checkpoint(envelope: &str) -> Result<ParsedCheckpoint<'_>, Error> {
    let malformed = |reason: &str| Error::Parse(ParseError::Checkpoint(reason.to_owned()));

    let blank_line_at = envelope
        .find("\n\n")
        .ok_or_else(|| malformed("missing blank line separating body from signatures"))?;
    let note_body = &envelope[..=blank_line_at];
    let body_text = &envelope[..blank_line_at];
    let signature_text = &envelope[blank_line_at + 2..];

    // Keep the origin's exact bytes.  `str::lines()` strips a trailing `\r`,
    // which would silently normalize a signed origin before policy matching.
    let (origin, body_without_origin) = body_text
        .split_once('\n')
        .ok_or_else(|| malformed("missing origin line"))?;
    if origin.is_empty() {
        return Err(malformed("missing origin line"));
    }
    let mut body_lines = body_without_origin.lines();
    let tree_size_line = body_lines
        .next()
        .ok_or_else(|| malformed("missing treeSize line"))?;
    let tree_size: u64 = tree_size_line
        .parse()
        .map_err(|_| malformed("treeSize line is not a valid unsigned integer"))?;
    let root_hash_line = body_lines
        .next()
        .ok_or_else(|| malformed("missing rootHash line"))?;
    let root_hash = parse_util::strict_base64("checkpoint rootHash line", root_hash_line)?;
    // Any further `body_lines` are extensions: opaque and unvalidated in
    // this task's scope.

    // Bound the signature-line count *before* decoding any of them.
    // An attacker can put the expected public key hint on every line, and
    // these lines are covered by neither the SET nor the inclusion proof,
    // so on an otherwise genuine bundle their number is entirely attacker-
    // chosen: without this check one 8 MiB input buys ~10^5 ECDSA
    // verifications, all spent before the certificate chain is looked at.
    // Counting non-empty lines only, since empty ones are skipped below
    // and cost nothing -- the bound is on verifications, not on bytes.
    let signature_line_count = signature_text
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    if signature_line_count > limits::MAX_CHECKPOINT_SIGNATURES {
        return Err(Error::ResourceLimit(
            ResourceLimitError::TooManyCheckpointSignatures {
                actual: signature_line_count,
                limit: limits::MAX_CHECKPOINT_SIGNATURES,
            },
        ));
    }

    let mut signatures = Vec::with_capacity(signature_line_count);
    for line in signature_text.lines() {
        if line.is_empty() {
            continue;
        }
        let rest = line
            .strip_prefix(CHECKPOINT_SIGNATURE_PREFIX)
            .ok_or_else(|| malformed("signature line missing \"\u{2014} \" marker"))?;
        let (name, blob_b64) = rest
            .split_once(' ')
            .ok_or_else(|| malformed("signature line missing name/signature separator"))?;
        let blob = parse_util::strict_base64("checkpoint signature line", blob_b64)?;
        if blob.len() <= CHECKPOINT_KEY_HINT_BYTES {
            return Err(malformed("signature blob shorter than the 4-byte key hint"));
        }
        let key_hint = blob[..CHECKPOINT_KEY_HINT_BYTES]
            .try_into()
            .map_err(|_| malformed("signature blob has a malformed key hint"))?;
        signatures.push(CheckpointSignature {
            name,
            key_hint,
            signature: blob[CHECKPOINT_KEY_HINT_BYTES..].to_vec(),
        });
    }
    if signatures.is_empty() {
        return Err(malformed("no signature lines found"));
    }

    Ok(ParsedCheckpoint {
        origin,
        tree_size,
        root_hash,
        note_body,
        signatures,
    })
}

pub(crate) fn checkpoint_key_hint(
    log: &TransparencyLog,
) -> Option<[u8; CHECKPOINT_KEY_HINT_BYTES]> {
    if let Some(checkpoint_key_id) = &log.checkpoint_key_id {
        return checkpoint_key_id
            .get(..CHECKPOINT_KEY_HINT_BYTES)?
            .try_into()
            .ok();
    }

    Sha256::digest(&log.public_key.raw_bytes)[..CHECKPOINT_KEY_HINT_BYTES]
        .try_into()
        .ok()
}

/// Whether a signed-note signature line's key name gets `log`'s priority.
///
/// `DESIGN.md` "Rekor-entry <-> bundle binding": a trusted `baseUrl`
/// carrying one leading `https://` counts as equivalent to its scheme-less
/// form, and everything else compares byte for byte. Deliberately no case
/// folding, trailing-slash trimming, percent-decoding, or DNS/Unicode
/// normalization: every one of those widens what an attacker-supplied
/// string receives priority, and none is needed for the real fixture
/// (`baseUrl` `https://rekor.sigstore.dev`, key name `rekor.sigstore.dev`).
/// This comparison is not an acceptance condition or authenticated binding.
fn checkpoint_key_name_matches(log: &TransparencyLog, name: &str) -> bool {
    let base_url = log.base_url.as_str();
    name == base_url || Some(name) == base_url.strip_prefix("https://")
}

/// Verifies a checkpoint envelope: its tree size and root hash must match
/// the inclusion proof it anchors, and at least one of its signature
/// lines carrying the trusted log's 4-byte key hint must verify against
/// `log_key` — the same trusted key already selected for the SET
/// (`DESIGN.md` "Rekor-entry <-> bundle binding": `logId == selected
/// trusted key`). Matching key names are tried first, but non-matching names
/// remain eligible as a fallback.
///
/// The key hint is public candidate metadata, while the name is a public
/// priority hint; both are outside the signed note body and unauthenticated.
/// A valid signature line with an arbitrary name therefore still verifies via
/// the fallback pass, and an attacker can label every decoy with the expected
/// name to defeat the ordering optimization. Deployment/origin binding is
/// performed by [`verify_tlog_entry`] using the caller's policy.
///
/// # Errors
///
/// Returns [`ParseError::Checkpoint`] if `envelope` does not match the
/// expected signed-note structure,
/// [`ResourceLimitError::TooManyCheckpointSignatures`] if it carries more
/// signature lines than [`limits::MAX_CHECKPOINT_SIGNATURES`],
/// [`TransparencyError::CheckpointTreeSizeMismatch`] /
/// [`TransparencyError::CheckpointRootHashMismatch`] on a mismatch
/// against `proof_tree_size` / `proof_root_hash`, and
/// [`TransparencyError::CheckpointSignatureInvalid`] if no signature line
/// verifies.  On success, returns the authenticated parsed checkpoint so the
/// caller can apply origin policy *after* this cryptographic check.
pub(crate) fn verify_checkpoint<'a>(
    envelope: &'a str,
    proof_tree_size: u64,
    proof_root_hash: &[u8],
    log: &TransparencyLog,
    log_key: &EcdsaVerifyingKey,
) -> Result<ParsedCheckpoint<'a>, Error> {
    let checkpoint = parse_checkpoint(envelope)?;
    if checkpoint.tree_size != proof_tree_size {
        return Err(Error::Transparency(
            TransparencyError::CheckpointTreeSizeMismatch,
        ));
    }
    if checkpoint.root_hash != proof_root_hash {
        return Err(Error::Transparency(
            TransparencyError::CheckpointRootHashMismatch,
        ));
    }
    let note_body_bytes = checkpoint.note_body.as_bytes();
    let Some(key_hint) = checkpoint_key_hint(log) else {
        return Err(Error::Transparency(
            TransparencyError::CheckpointSignatureInvalid,
        ));
    };
    let verified = checkpoint
        .signatures
        .iter()
        .filter(|signature| signature.key_hint == key_hint)
        .filter(|signature| checkpoint_key_name_matches(log, signature.name))
        .any(|signature| log_key.verify_der(note_body_bytes, &signature.signature))
        || checkpoint
            .signatures
            .iter()
            .filter(|signature| signature.key_hint == key_hint)
            .filter(|signature| !checkpoint_key_name_matches(log, signature.name))
            .any(|signature| log_key.verify_der(note_body_bytes, &signature.signature));
    if verified {
        Ok(checkpoint)
    } else {
        Err(Error::Transparency(
            TransparencyError::CheckpointSignatureInvalid,
        ))
    }
}

/// Runs [`verify_checkpoint`] with the envelope anchored to its *own*
/// declared tree size and root hash, so the two binding comparisons
/// always pass and the signature block is always reached.
///
/// Fuzzing-only. Feeding a fuzzer arbitrary `proof_tree_size` /
/// `proof_root_hash` would mean nearly every input died at the tree-size
/// comparison, leaving the key-hint filter, the signature-count bound,
/// and the ECDSA loop — the parts whose cost and control flow are driven
/// by attacker-chosen bytes — effectively unreachable. Those two
/// comparisons are covered by unit tests instead; neutralising them here
/// is what buys coverage of everything after them.
#[cfg(feature = "fuzzing")]
pub(crate) fn verify_checkpoint_self_anchored(
    envelope: &str,
    log: &TransparencyLog,
    log_key: &EcdsaVerifyingKey,
) -> Result<(), Error> {
    let checkpoint = parse_checkpoint(envelope)?;
    let (tree_size, root_hash) = (checkpoint.tree_size, checkpoint.root_hash.clone());
    verify_checkpoint(envelope, tree_size, &root_hash, log, log_key).map(|_| ())
}

// ---------------------------------------------------------------------
// 5. Time window
// ---------------------------------------------------------------------

/// Checks that `integrated_time` (unix seconds) falls within both the
/// selected trusted log key's `validFor` window and the leaf
/// certificate's `[notBefore, notAfter]` window.
///
/// The log key window is half-open (`end` exclusive, absent meaning
/// still valid, matching [`ValidityPeriod`]'s documented semantics); the
/// certificate window is the X.509/RFC 5280 convention of inclusive on
/// both ends.
///
/// # Errors
///
/// Returns [`TimestampError::IntegratedTimeOutsideLogKeyValidity`] or
/// [`TimestampError::IntegratedTimeOutsideCertificateValidity`],
/// whichever window rejects first.
pub(crate) fn check_time_window(
    integrated_time: u64,
    log_key_valid_for: &ValidityPeriod,
    leaf_not_before: i64,
    leaf_not_after: i64,
) -> Result<(), Error> {
    let outside_log_key = || Error::Timestamp(TimestampError::IntegratedTimeOutsideLogKeyValidity);

    let integrated_time = i64::try_from(integrated_time).map_err(|_| outside_log_key())?;

    let within_log_key_window = log_key_valid_for.start <= integrated_time
        && log_key_valid_for
            .end
            .is_none_or(|end| integrated_time < end);
    if !within_log_key_window {
        return Err(outside_log_key());
    }

    if !(leaf_not_before <= integrated_time && integrated_time <= leaf_not_after) {
        return Err(Error::Timestamp(
            TimestampError::IntegratedTimeOutsideCertificateValidity,
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------
// Composed verification
// ---------------------------------------------------------------------

/// The authenticated facts extracted from a verified Rekor v1 tlog entry:
/// integrated time and log index.
///
/// Internal to the crate — the public
/// [`crate::verifier::VerifiedTransparency`] is constructed only once the
/// full verification chain exists (this module is not wired into
/// [`crate::Verifier`] yet).
#[derive(Debug, Clone, Copy)]
pub(crate) struct VerifiedTimestamp {
    /// Unix seconds at which the entry was authenticated as integrated,
    /// per the verified inclusion promise (SET).
    pub(crate) integrated_time: u64,
    /// The entry's index in the log.
    pub(crate) log_index: u64,
}

/// Verifies `bundle`'s Rekor v1 transparency-log entry end-to-end:
/// canonicalized-body binding, SET, Merkle inclusion proof, checkpoint,
/// and time window, in that order (`DESIGN.md` "Time-evidence model",
/// steps 1-5; this module's own numbering above matches the order used
/// here).
///
/// # Errors
///
/// Returns [`TransparencyError::NoTlogEntries`] if the bundle carries no
/// transparency-log entries, [`UnsupportedError::MultipleTlogEntries`] if
/// it carries more than one (this crate verifies exactly one; selecting
/// among several candidate entries is unimplemented),
/// [`TransparencyError::InclusionProofMissing`] if the entry lacks one,
/// and otherwise whatever the first failing check among
/// [`parse_tlog_entry_body`], [`check_entry_binding`], [`select_log_key`],
/// [`verify_set`], [`verify_inclusion_proof`], [`verify_checkpoint`], and
/// [`check_time_window`] returns.
pub(crate) fn verify_tlog_entry(
    bundle: &Bundle,
    trust_store: &TrustStore,
    checkpoint_origin_policy: &CheckpointOriginPolicy,
) -> Result<VerifiedTimestamp, Error> {
    let tlog_entries = &bundle.verification_material.tlog_entries;
    let tlog_entry = match tlog_entries.as_slice() {
        [] => return Err(Error::Transparency(TransparencyError::NoTlogEntries)),
        [entry] => entry,
        entries => {
            return Err(Error::Unsupported(UnsupportedError::MultipleTlogEntries {
                count: entries.len(),
            }));
        }
    };
    let leaf_certificate = &bundle.verification_material.certificate;

    // 1. Canonicalized body model + binding.
    let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
    check_entry_binding(
        tlog_entry,
        &body,
        &bundle.dsse_envelope,
        &leaf_certificate.raw_bytes,
    )?;

    // 2. SET (inclusion promise).
    let (log, log_key) = select_log_key(trust_store, &tlog_entry.log_id_key_id)?;
    let selected_key_id: [u8; 32] = Sha256::digest(&log.public_key.raw_bytes).into();
    verify_set(tlog_entry, &selected_key_id, &log_key)?;

    // 3. Merkle inclusion proof.
    let Some(inclusion_proof) = &tlog_entry.inclusion_proof else {
        return Err(Error::Transparency(
            TransparencyError::InclusionProofMissing,
        ));
    };
    verify_inclusion_proof(&tlog_entry.canonicalized_body, inclusion_proof)?;

    // 4. Checkpoint (signed note), anchored to the same trusted log key.
    let checkpoint = verify_checkpoint(
        &inclusion_proof.checkpoint.envelope,
        inclusion_proof.tree_size,
        &inclusion_proof.root_hash,
        log,
        &log_key,
    )?;

    // The checkpoint's origin is compared only after its signed note has
    // authenticated under the selected key.  A mutated origin therefore
    // remains `CheckpointSignatureInvalid`, while an authenticated but
    // disallowed origin receives the deliberately opaque mismatch error.
    if !checkpoint_origin_policy.allows(&selected_key_id, checkpoint.origin) {
        return Err(Error::Transparency(
            TransparencyError::CheckpointOriginMismatch,
        ));
    }

    // 5. Time window: only now is integratedTime authenticated enough to
    // use for certificate/log-key validity.
    let leaf_info = LeafCertificateInfo::from_certificate(leaf_certificate)?;
    check_time_window(
        tlog_entry.integrated_time,
        &log.public_key.valid_for,
        leaf_info.not_before,
        leaf_info.not_after,
    )?;

    Ok(VerifiedTimestamp {
        integrated_time: tlog_entry.integrated_time,
        log_index: tlog_entry.log_index,
    })
}

// ---------------------------------------------------------------------
// Raw (untrusted-shape) mirror of the canonicalized body: plain serde
// derive, tolerant of unknown fields, strictness applied on conversion
// to `TlogEntryBody` above.
// ---------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTlogEntryBody {
    api_version: String,
    kind: String,
    spec: RawDsseSpec,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDsseSpec {
    envelope_hash: RawHash,
    payload_hash: RawHash,
    #[serde(default)]
    signatures: Vec<RawDsseSpecSignature>,
}

#[derive(serde::Deserialize)]
struct RawHash {
    // Kept only for faithful deserialization of the object shape; not
    // otherwise read (see module docs on why the algorithm string itself
    // isn't separately validated).
    #[allow(dead_code)]
    algorithm: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct RawDsseSpecSignature {
    signature: String,
    verifier: String,
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::{
        CHECKPOINT_KEY_HINT_BYTES, CHECKPOINT_SIGNATURE_PREFIX, VerifiedTimestamp,
        check_entry_binding, check_time_window, checkpoint_key_name_matches, parse_checkpoint,
        parse_tlog_entry_body, select_log_key, verify_checkpoint, verify_inclusion_proof,
        verify_set, verify_tlog_entry,
    };
    use crate::bundle::Bundle;
    use crate::dsse::{LeafCertificateInfo, reset_verify_der_call_count, verify_der_call_count};
    use crate::error::{
        ContentBindingError, Error, ResourceLimitError, TimestampError, TransparencyError,
        UnsupportedError,
    };
    use crate::limits;
    use crate::policy::{
        CheckpointOriginPolicy, GithubPolicy, RefPolicy, RepositoryIdentity, SignerPolicy,
        SourcePolicy, WorkflowPath, WorkflowRevisionPolicy,
    };
    use crate::subject::Subject;
    use crate::trust::{TrustStore, ValidityPeriod};
    use crate::verifier::Verifier;

    const GOLDEN_FIXTURE: &str = "github-cli/tarball-user-slsa-provenance.json";

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative)
    }

    fn read_fixture(relative: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(fixture_path(relative))?)
    }

    fn real_bundle() -> Result<Bundle, Box<dyn std::error::Error>> {
        Ok(Bundle::from_json(&read_fixture(GOLDEN_FIXTURE)?)?)
    }

    fn embedded_trust_store() -> Result<TrustStore, Box<dyn std::error::Error>> {
        Ok(TrustStore::embedded_public_good()?)
    }

    fn fixture_origin_policy(
        trust_store: &TrustStore,
    ) -> Result<CheckpointOriginPolicy, Box<dyn std::error::Error>> {
        let log = trust_store
            .tlogs
            .iter()
            .find(|log| log.base_url == "https://rekor.sigstore.dev")
            .ok_or("missing fixture Rekor v1 key")?;
        Ok(CheckpointOriginPolicy::builder()
            .allow_origin(log, "rekor.sigstore.dev - 1193050959916656506")?
            .build()?)
    }

    fn verifier_with_correct_policy() -> Result<Verifier, Box<dyn std::error::Error>> {
        let policy = GithubPolicy::builder()
            .source(SourcePolicy {
                repository: RepositoryIdentity::parse("cli/cli")?
                    .with_owner_id(59_704_711)
                    .with_repository_id(212_613_049),
                git_ref: RefPolicy::Exact("refs/heads/trunk".to_owned()),
                commit: None,
            })
            .signer(SignerPolicy {
                repository: RepositoryIdentity::parse("cli/cli")?,
                path: WorkflowPath::new(".github/workflows/deployment.yml")?,
                revision: WorkflowRevisionPolicy::Any,
            })
            .build()?;
        let trust_store = embedded_trust_store()?;
        Ok(Verifier::builder()
            .trust_store(trust_store.clone())
            .github_policy(policy)
            .checkpoint_origin_policy(fixture_origin_policy(&trust_store)?)
            .build()?)
    }

    /// Parses the golden fixture as JSON, lets `f` mutate it, then
    /// re-serializes and re-parses as a [`Bundle`].
    fn mutate_bundle_json(
        f: impl FnOnce(&mut serde_json::Value) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<Bundle, Box<dyn std::error::Error>> {
        let mut value: serde_json::Value = serde_json::from_slice(&read_fixture(GOLDEN_FIXTURE)?)?;
        f(&mut value)?;
        Ok(Bundle::from_json(&serde_json::to_vec(&value)?)?)
    }

    /// Decodes `bundle_value`'s `canonicalizedBody`, lets `f` mutate the
    /// decoded JSON body, then re-encodes it back into place.
    fn mutate_canonicalized_body_json(
        bundle_value: &mut serde_json::Value,
        f: impl FnOnce(&mut serde_json::Value) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let body_b64 = bundle_value["verificationMaterial"]["tlogEntries"][0]["canonicalizedBody"]
            .as_str()
            .ok_or("missing canonicalizedBody")?
            .to_owned();
        let mut body_value: serde_json::Value =
            serde_json::from_slice(&STANDARD.decode(&body_b64)?)?;
        f(&mut body_value)?;
        bundle_value["verificationMaterial"]["tlogEntries"][0]["canonicalizedBody"] =
            serde_json::Value::String(STANDARD.encode(serde_json::to_vec(&body_value)?));
        Ok(())
    }

    /// Returns `envelope` with `count` decoy signature lines inserted
    /// ahead of its genuine one, leaving the genuine signature last.
    ///
    /// Each decoy carries the smallest well-formed DER ECDSA signature,
    /// `SEQUENCE { INTEGER 1, INTEGER 1 }`, behind the 4-byte key hint:
    /// well-formed enough to reach the curve arithmetic, and the cheapest
    /// possible bytes-per-verification for an attacker.
    fn prepend_decoy_signature_lines(
        envelope: &str,
        count: usize,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let (body, genuine_signatures) = envelope
            .split_once("\n\n")
            .ok_or("missing blank line separating body from signatures")?;
        let decoy_blob = STANDARD.encode([0, 0, 0, 0, 0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 1]);
        let mut flooded = String::from(body);
        flooded.push_str("\n\n");
        for _ in 0..count {
            flooded.push_str(CHECKPOINT_SIGNATURE_PREFIX);
            flooded.push_str("decoy ");
            flooded.push_str(&decoy_blob);
            flooded.push('\n');
        }
        flooded.push_str(genuine_signatures);
        Ok(flooded)
    }

    /// Like [`prepend_decoy_signature_lines`], but each decoy copies the
    /// genuine line's key hint and carries `name`, allowing tests to control
    /// whether the name-priority hint matches or misses.
    ///
    /// The hint is public, so an attacker can always replay it; this is what
    /// isolates the name-priority behavior, which the all-zero hints in
    /// [`prepend_decoy_signature_lines`] would otherwise mask.
    fn prepend_decoys_with_genuine_key_hint(
        envelope: &str,
        count: usize,
        name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let (body, genuine_signatures) = envelope
            .split_once("\n\n")
            .ok_or("missing blank line separating body from signatures")?;
        let genuine_line = genuine_signatures
            .lines()
            .find(|line| line.starts_with(CHECKPOINT_SIGNATURE_PREFIX))
            .ok_or("no genuine signature line")?;
        let genuine_blob = STANDARD.decode(
            genuine_line
                .rsplit_once(' ')
                .ok_or("malformed genuine signature line")?
                .1,
        )?;
        let mut decoy = genuine_blob
            .get(..CHECKPOINT_KEY_HINT_BYTES)
            .ok_or("genuine signature has no key hint")?
            .to_vec();
        decoy.extend_from_slice(&[0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01]);
        let decoy_blob = STANDARD.encode(decoy);

        let mut flooded = String::from(body);
        flooded.push_str("\n\n");
        for _ in 0..count {
            flooded.push_str(CHECKPOINT_SIGNATURE_PREFIX);
            flooded.push_str(name);
            flooded.push(' ');
            flooded.push_str(&decoy_blob);
            flooded.push('\n');
        }
        flooded.push_str(genuine_signatures);
        Ok(flooded)
    }

    fn replace_checkpoint_line(
        envelope: &str,
        index: usize,
        new_line: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut lines: Vec<&str> = envelope.split('\n').collect();
        let slot = lines
            .get_mut(index)
            .ok_or("checkpoint line index out of range")?;
        *slot = new_line;
        Ok(lines.join("\n"))
    }

    // -------------------------------------------------------------
    // Positive: each check, then the composed function, against the
    // real fixture and the real embedded trust store.
    // -------------------------------------------------------------

    #[test]
    fn parses_real_canonicalized_body() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        if body.kind != "dsse" || body.api_version != "0.0.1" {
            return Err(format!(
                "unexpected kind/apiVersion: {}/{}",
                body.kind, body.api_version
            )
            .into());
        }
        Ok(())
    }

    #[test]
    fn real_entry_binding_holds() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        check_entry_binding(
            tlog_entry,
            &body,
            &bundle.dsse_envelope,
            &bundle.verification_material.certificate.raw_bytes,
        )?;
        Ok(())
    }

    #[test]
    fn real_set_verifies_with_real_log_key() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        verify_set(tlog_entry, &log.log_id_key_id, &log_key)?;
        Ok(())
    }

    #[test]
    fn real_inclusion_proof_reproduces_real_root() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        verify_inclusion_proof(&tlog_entry.canonicalized_body, proof)?;
        Ok(())
    }

    #[test]
    fn real_checkpoint_signature_verifies() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        verify_checkpoint(
            &proof.checkpoint.envelope,
            proof.tree_size,
            &proof.root_hash,
            log,
            &log_key,
        )?;
        Ok(())
    }

    #[test]
    fn real_integrated_time_is_within_all_windows() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, _log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        let leaf_info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        check_time_window(
            tlog_entry.integrated_time,
            &log.public_key.valid_for,
            leaf_info.not_before,
            leaf_info.not_after,
        )?;
        Ok(())
    }

    #[test]
    fn composed_verify_tlog_entry_succeeds_on_real_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let VerifiedTimestamp {
            integrated_time,
            log_index,
        } = verify_tlog_entry(&bundle, &trust_store, &fixture_origin_policy(&trust_store)?)?;
        if integrated_time != 1_783_027_755 {
            return Err(format!("unexpected integratedTime: {integrated_time}").into());
        }
        if log_index != 2_049_189_324 {
            return Err(format!("unexpected logIndex: {log_index}").into());
        }
        Ok(())
    }

    // -------------------------------------------------------------
    // Negative: binding
    // -------------------------------------------------------------

    #[test]
    fn flipped_dsse_payload_byte_fails_binding() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            let payload_b64 = v["dsseEnvelope"]["payload"]
                .as_str()
                .ok_or("missing payload")?
                .to_owned();
            let mut decoded = STANDARD.decode(payload_b64)?;
            decoded[0] ^= 0x01;
            v["dsseEnvelope"]["payload"] = serde_json::Value::String(STANDARD.encode(decoded));
            Ok(())
        })?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        match check_entry_binding(
            tlog_entry,
            &body,
            &bundle.dsse_envelope,
            &bundle.verification_material.certificate.raw_bytes,
        ) {
            Err(Error::ContentBinding(ContentBindingError::TlogEntryPayloadHashMismatch)) => Ok(()),
            other => Err(format!("expected TlogEntryPayloadHashMismatch, got {other:?}").into()),
        }
    }

    #[test]
    fn flipped_dsse_signature_byte_fails_binding() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            let sig_b64 = v["dsseEnvelope"]["signatures"][0]["sig"]
                .as_str()
                .ok_or("missing sig")?
                .to_owned();
            let mut decoded = STANDARD.decode(sig_b64)?;
            decoded[0] ^= 0x01;
            v["dsseEnvelope"]["signatures"][0]["sig"] =
                serde_json::Value::String(STANDARD.encode(decoded));
            Ok(())
        })?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        match check_entry_binding(
            tlog_entry,
            &body,
            &bundle.dsse_envelope,
            &bundle.verification_material.certificate.raw_bytes,
        ) {
            Err(Error::ContentBinding(ContentBindingError::TlogEntrySignatureMismatch)) => Ok(()),
            other => Err(format!("expected TlogEntrySignatureMismatch, got {other:?}").into()),
        }
    }

    #[test]
    fn altered_canonicalized_body_fails_binding_and_inclusion_proof()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut value: serde_json::Value = serde_json::from_slice(&read_fixture(GOLDEN_FIXTURE)?)?;
        mutate_canonicalized_body_json(&mut value, |body| {
            let original = body["spec"]["payloadHash"]["value"]
                .as_str()
                .ok_or("missing payloadHash.value")?
                .to_owned();
            let mut chars: Vec<char> = original.chars().collect();
            let first = *chars.first().ok_or("empty payloadHash.value")?;
            chars[0] = if first == 'a' { 'b' } else { 'a' };
            body["spec"]["payloadHash"]["value"] =
                serde_json::Value::String(chars.into_iter().collect());
            Ok(())
        })?;
        let bundle = Bundle::from_json(&serde_json::to_vec(&value)?)?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];

        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        match check_entry_binding(
            tlog_entry,
            &body,
            &bundle.dsse_envelope,
            &bundle.verification_material.certificate.raw_bytes,
        ) {
            Err(Error::ContentBinding(ContentBindingError::TlogEntryPayloadHashMismatch)) => {}
            other => {
                return Err(format!("expected TlogEntryPayloadHashMismatch, got {other:?}").into());
            }
        }

        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        match verify_inclusion_proof(&tlog_entry.canonicalized_body, proof) {
            Err(Error::Transparency(TransparencyError::InclusionProofInvalid)) => Ok(()),
            other => Err(format!("expected InclusionProofInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn unknown_body_kind_version_is_unsupported() -> Result<(), Box<dyn std::error::Error>> {
        let mut value: serde_json::Value = serde_json::from_slice(&read_fixture(GOLDEN_FIXTURE)?)?;
        mutate_canonicalized_body_json(&mut value, |body| {
            body["kind"] = serde_json::Value::String("intoto".to_owned());
            Ok(())
        })?;
        let bundle = Bundle::from_json(&serde_json::to_vec(&value)?)?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        match parse_tlog_entry_body(&tlog_entry.canonicalized_body) {
            Err(Error::Unsupported(UnsupportedError::TlogEntryKindVersion { kind, version })) => {
                if kind == "intoto" && version == "0.0.1" {
                    Ok(())
                } else {
                    Err(format!("unexpected kind/version: {kind}/{version}").into())
                }
            }
            other => Err(format!("expected TlogEntryKindVersion, got {other:?}").into()),
        }
    }

    #[test]
    fn entry_signature_mismatch_using_other_fixture_signature()
    -> Result<(), Box<dyn std::error::Error>> {
        let other_value: serde_json::Value =
            serde_json::from_slice(&read_fixture("github-cli/tarball-github-release-tsa.json")?)?;
        let other_sig = other_value["dsseEnvelope"]["signatures"][0]["sig"]
            .as_str()
            .ok_or("missing other fixture's dsse signature")?
            .to_owned();

        let mut value: serde_json::Value = serde_json::from_slice(&read_fixture(GOLDEN_FIXTURE)?)?;
        mutate_canonicalized_body_json(&mut value, |body| {
            body["spec"]["signatures"][0]["signature"] = serde_json::Value::String(other_sig);
            Ok(())
        })?;
        let bundle = Bundle::from_json(&serde_json::to_vec(&value)?)?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let body = parse_tlog_entry_body(&tlog_entry.canonicalized_body)?;
        match check_entry_binding(
            tlog_entry,
            &body,
            &bundle.dsse_envelope,
            &bundle.verification_material.certificate.raw_bytes,
        ) {
            Err(Error::ContentBinding(ContentBindingError::TlogEntrySignatureMismatch)) => Ok(()),
            other => Err(format!("expected TlogEntrySignatureMismatch, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: SET
    // -------------------------------------------------------------

    #[test]
    fn altered_integrated_time_fails_set() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"][0]["integratedTime"] =
                serde_json::Value::String("1783027756".to_owned());
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        match verify_set(tlog_entry, &log.log_id_key_id, &log_key) {
            Err(Error::Transparency(TransparencyError::SetInvalid)) => Ok(()),
            other => Err(format!("expected SetInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn altered_log_index_fails_set() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"][0]["logIndex"] =
                serde_json::Value::String("2049189325".to_owned());
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        match verify_set(tlog_entry, &log.log_id_key_id, &log_key) {
            Err(Error::Transparency(TransparencyError::SetInvalid)) => Ok(()),
            other => Err(format!("expected SetInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn set_absent_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"][0]
                .as_object_mut()
                .ok_or("expected tlogEntries[0] to be an object")?
                .remove("inclusionPromise");
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        match verify_set(tlog_entry, &log.log_id_key_id, &log_key) {
            Err(Error::Transparency(TransparencyError::SetMissing)) => Ok(()),
            other => Err(format!("expected SetMissing, got {other:?}").into()),
        }
    }

    #[test]
    fn unknown_log_id_fails_selection() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"][0]["logId"]["keyId"] =
                serde_json::Value::String(STANDARD.encode([0u8; 32]));
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        match select_log_key(&trust_store, &tlog_entry.log_id_key_id) {
            Err(Error::Transparency(TransparencyError::UnknownLogKey)) => Ok(()),
            other => Err(format!("expected UnknownLogKey, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: inclusion proof
    // -------------------------------------------------------------

    #[test]
    fn truncated_inclusion_proof_hashes_fails() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            let hashes = v["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["hashes"]
                .as_array_mut()
                .ok_or("missing inclusionProof.hashes")?;
            hashes.pop().ok_or("expected at least one hash")?;
            Ok(())
        })?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        match verify_inclusion_proof(&tlog_entry.canonicalized_body, proof) {
            Err(Error::Transparency(TransparencyError::InclusionProofInvalid)) => Ok(()),
            other => Err(format!("expected InclusionProofInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn extended_inclusion_proof_hashes_fails() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            let hashes = v["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["hashes"]
                .as_array_mut()
                .ok_or("missing inclusionProof.hashes")?;
            let last = hashes.last().cloned().ok_or("expected at least one hash")?;
            hashes.push(last);
            Ok(())
        })?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        match verify_inclusion_proof(&tlog_entry.canonicalized_body, proof) {
            Err(Error::Transparency(TransparencyError::InclusionProofInvalid)) => Ok(()),
            other => Err(format!("expected InclusionProofInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn altered_root_hash_fails_inclusion_proof() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            let root_b64 =
                v["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["rootHash"]
                    .as_str()
                    .ok_or("missing rootHash")?
                    .to_owned();
            let mut decoded = STANDARD.decode(root_b64)?;
            decoded[0] ^= 0x01;
            v["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["rootHash"] =
                serde_json::Value::String(STANDARD.encode(decoded));
            Ok(())
        })?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        match verify_inclusion_proof(&tlog_entry.canonicalized_body, proof) {
            Err(Error::Transparency(TransparencyError::InclusionProofInvalid)) => Ok(()),
            other => Err(format!("expected InclusionProofInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn inclusion_proof_absent_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"][0]
                .as_object_mut()
                .ok_or("expected tlogEntries[0] to be an object")?
                .remove("inclusionProof");
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        match verify_tlog_entry(&bundle, &trust_store, &fixture_origin_policy(&trust_store)?) {
            Err(Error::Transparency(TransparencyError::InclusionProofMissing)) => Ok(()),
            other => Err(format!("expected InclusionProofMissing, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: checkpoint
    // -------------------------------------------------------------

    #[test]
    fn checkpoint_tree_size_mismatch_fails() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        let mutated_envelope = replace_checkpoint_line(
            &proof.checkpoint.envelope,
            1,
            &(proof.tree_size + 1).to_string(),
        )?;
        match verify_checkpoint(
            &mutated_envelope,
            proof.tree_size,
            &proof.root_hash,
            log,
            &log_key,
        ) {
            Err(Error::Transparency(TransparencyError::CheckpointTreeSizeMismatch)) => Ok(()),
            other => Err(format!("expected CheckpointTreeSizeMismatch, got {other:?}").into()),
        }
    }

    #[test]
    fn checkpoint_root_hash_mismatch_fails() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        let mut decoded = STANDARD.decode(
            proof
                .checkpoint
                .envelope
                .split('\n')
                .nth(2)
                .ok_or("missing rootHash line")?,
        )?;
        decoded[0] ^= 0x01;
        let mutated_envelope =
            replace_checkpoint_line(&proof.checkpoint.envelope, 2, &STANDARD.encode(decoded))?;
        match verify_checkpoint(
            &mutated_envelope,
            proof.tree_size,
            &proof.root_hash,
            log,
            &log_key,
        ) {
            Err(Error::Transparency(TransparencyError::CheckpointRootHashMismatch)) => Ok(()),
            other => Err(format!("expected CheckpointRootHashMismatch, got {other:?}").into()),
        }
    }

    #[test]
    fn checkpoint_signature_bitflip_fails() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        let sig_line = proof
            .checkpoint
            .envelope
            .split('\n')
            .nth(4)
            .ok_or("missing signature line")?;
        let (prefix, blob_b64) = sig_line
            .rsplit_once(' ')
            .ok_or("malformed signature line")?;
        let mut decoded = STANDARD.decode(blob_b64)?;
        let last_index = decoded.len().checked_sub(1).ok_or("empty signature blob")?;
        decoded[last_index] ^= 0x01;
        let mutated_line = format!("{prefix} {}", STANDARD.encode(decoded));
        let mutated_envelope =
            replace_checkpoint_line(&proof.checkpoint.envelope, 4, &mutated_line)?;

        match verify_checkpoint(
            &mutated_envelope,
            proof.tree_size,
            &proof.root_hash,
            log,
            &log_key,
        ) {
            Err(Error::Transparency(TransparencyError::CheckpointSignatureInvalid)) => Ok(()),
            other => Err(format!("expected CheckpointSignatureInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn checkpoint_signature_flood_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        // The genuine signature is still present, and last. Without the
        // count check this envelope verifies *successfully*, having first
        // spent one ECDSA verification per decoy -- so a check that only
        // rejected invalid signatures would not close this. Nothing binds
        // these lines to the rest of the entry: the SET and the inclusion
        // proof both cover the note body, not its signature block, which
        // is what lets an attacker pad an otherwise genuine bundle.
        let flooded = prepend_decoy_signature_lines(
            &proof.checkpoint.envelope,
            limits::MAX_CHECKPOINT_SIGNATURES,
        )?;
        match verify_checkpoint(&flooded, proof.tree_size, &proof.root_hash, log, &log_key) {
            Err(Error::ResourceLimit(ResourceLimitError::TooManyCheckpointSignatures {
                actual,
                limit,
            })) if actual == limits::MAX_CHECKPOINT_SIGNATURES + 1
                && limit == limits::MAX_CHECKPOINT_SIGNATURES =>
            {
                Ok(())
            }
            other => Err(format!("expected TooManyCheckpointSignatures, got {other:?}").into()),
        }
    }

    #[test]
    fn checkpoint_at_the_signature_limit_still_verifies() -> Result<(), Box<dyn std::error::Error>>
    {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        // Exactly at the limit, with the genuine signature last: pins that
        // the bound is inclusive, and that a checkpoint whose own signature
        // is not the first line (a witness-cosigned log) still verifies.
        let padded = prepend_decoy_signature_lines(
            &proof.checkpoint.envelope,
            limits::MAX_CHECKPOINT_SIGNATURES - 1,
        )?;
        verify_checkpoint(&padded, proof.tree_size, &proof.root_hash, log, &log_key)?;
        Ok(())
    }

    #[test]
    fn checkpoint_signature_name_is_only_an_unauthenticated_priority_hint()
    -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        // Copy the genuine hint and DER signature verbatim, but give the
        // otherwise-valid line an arbitrary label. The label is outside the
        // signed note body, so it must not prevent cryptographic verification.
        let signature_line_index = proof
            .checkpoint
            .envelope
            .split('\n')
            .position(|line| line.starts_with(CHECKPOINT_SIGNATURE_PREFIX))
            .ok_or("missing signature line")?;
        let sig_line = proof
            .checkpoint
            .envelope
            .split('\n')
            .nth(signature_line_index)
            .ok_or("missing signature line")?;
        let blob_b64 = sig_line
            .strip_prefix(CHECKPOINT_SIGNATURE_PREFIX)
            .and_then(|line| line.rsplit_once(' '))
            .map(|(_, blob)| blob)
            .ok_or("malformed signature line")?;
        let other_label = format!("{CHECKPOINT_SIGNATURE_PREFIX}other.log.example {blob_b64}");
        let other_labeled_envelope = replace_checkpoint_line(
            &proof.checkpoint.envelope,
            signature_line_index,
            &other_label,
        )?;
        let other_checkpoint = parse_checkpoint(&other_labeled_envelope)?;
        let original_checkpoint = parse_checkpoint(&proof.checkpoint.envelope)?;
        assert_eq!(other_checkpoint.note_body, original_checkpoint.note_body);
        assert_eq!(other_checkpoint.signatures.len(), 1);
        assert_eq!(
            other_checkpoint.signatures[0].key_hint,
            original_checkpoint.signatures[0].key_hint
        );
        assert_eq!(
            other_checkpoint.signatures[0].signature,
            original_checkpoint.signatures[0].signature
        );

        // The unchanged valid signature still verifies despite the arbitrary
        // label, proving that name matching only affects ordering.
        verify_checkpoint(
            &other_labeled_envelope,
            proof.tree_size,
            &proof.root_hash,
            log,
            &log_key,
        )?;
        Ok(())
    }

    #[test]
    fn checkpoint_key_name_priority_is_exact_apart_from_one_https_prefix()
    -> Result<(), Box<dyn std::error::Error>> {
        let trust_store = embedded_trust_store()?;
        let log = trust_store
            .tlogs
            .iter()
            .find(|log| log.base_url == "https://rekor.sigstore.dev")
            .ok_or("embedded root has no rekor.sigstore.dev log")?;

        for accepted in ["rekor.sigstore.dev", "https://rekor.sigstore.dev"] {
            if !checkpoint_key_name_matches(log, accepted) {
                return Err(format!("expected {accepted:?} to match").into());
            }
        }
        // Each name below misses the priority predicate. It remains eligible
        // in the fallback pass; this test only pins the ordering distinction.
        for rejected in [
            "Rekor.Sigstore.Dev",
            "rekor.sigstore.dev/",
            "https://rekor.sigstore.dev/",
            "http://rekor.sigstore.dev",
            "https://https://rekor.sigstore.dev",
            "rekor.sigstore.dev.",
            " rekor.sigstore.dev",
            "",
        ] {
            if checkpoint_key_name_matches(log, rejected) {
                return Err(format!("expected {rejected:?} to be rejected").into());
            }
        }
        Ok(())
    }

    #[test]
    fn public_verifier_prioritizes_matching_checkpoint_signature_names()
    -> Result<(), Box<dyn std::error::Error>> {
        let verifier = verifier_with_correct_policy()?;
        let subject = Subject::from_digest_hex(
            "f146e594bdba65fdc499c43c86d6d07c0e0733257152067c4f9fe2acee4f4629",
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &real_bundle()?)?;
        let genuine_verify_calls = verify_der_call_count();

        // Decoys carrying the *genuine* key hint: the hint is public, so
        // replaying it is free. Their non-matching names should leave them
        // in the fallback pass, after the genuine matching line.
        let mut padded_bundle = real_bundle()?;
        let proof = padded_bundle.verification_material.tlog_entries[0]
            .inclusion_proof
            .as_mut()
            .ok_or("missing inclusion proof")?;
        proof.checkpoint.envelope = prepend_decoys_with_genuine_key_hint(
            &proof.checkpoint.envelope,
            limits::MAX_CHECKPOINT_SIGNATURES - 1,
            "other.log.example",
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &padded_bundle)?;
        let padded_verify_calls = verify_der_call_count();

        if padded_verify_calls != genuine_verify_calls {
            return Err(format!(
                "non-matching-name decoys ran before the genuine matching line: genuine={genuine_verify_calls}, padded={padded_verify_calls}"
            )
            .into());
        }
        Ok(())
    }

    #[test]
    fn attacker_can_defeat_checkpoint_name_priority() -> Result<(), Box<dyn std::error::Error>> {
        let verifier = verifier_with_correct_policy()?;
        let subject = Subject::from_digest_hex(
            "f146e594bdba65fdc499c43c86d6d07c0e0733257152067c4f9fe2acee4f4629",
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &real_bundle()?)?;
        let genuine_verify_calls = verify_der_call_count();

        // An attacker can copy the public hint and use the expected name on
        // every decoy. All of those candidates are then in the preferred
        // pass, so the name hint provides no work reduction.
        let mut padded_bundle = real_bundle()?;
        let proof = padded_bundle.verification_material.tlog_entries[0]
            .inclusion_proof
            .as_mut()
            .ok_or("missing inclusion proof")?;
        proof.checkpoint.envelope = prepend_decoys_with_genuine_key_hint(
            &proof.checkpoint.envelope,
            limits::MAX_CHECKPOINT_SIGNATURES - 1,
            "rekor.sigstore.dev",
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &padded_bundle)?;
        let padded_verify_calls = verify_der_call_count();
        let expected_calls = genuine_verify_calls + limits::MAX_CHECKPOINT_SIGNATURES - 1;
        if padded_verify_calls != expected_calls {
            return Err(format!(
                "expected every matching-name decoy to reach ECDSA: genuine={genuine_verify_calls}, padded={padded_verify_calls}, expected={expected_calls}"
            )
            .into());
        }
        Ok(())
    }

    #[test]
    fn public_verifier_skips_checkpoint_signatures_with_other_key_hints()
    -> Result<(), Box<dyn std::error::Error>> {
        let verifier = verifier_with_correct_policy()?;
        let genuine_bundle = real_bundle()?;
        let subject = Subject::from_digest_hex(
            "f146e594bdba65fdc499c43c86d6d07c0e0733257152067c4f9fe2acee4f4629",
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &genuine_bundle)?;
        let genuine_verify_calls = verify_der_call_count();
        if genuine_verify_calls == 0 {
            return Err("expected the public verifier to perform ECDSA verification".into());
        }

        let mut padded_bundle = real_bundle()?;
        let proof = padded_bundle.verification_material.tlog_entries[0]
            .inclusion_proof
            .as_mut()
            .ok_or("missing inclusion proof")?;
        proof.checkpoint.envelope = prepend_decoy_signature_lines(
            &proof.checkpoint.envelope,
            limits::MAX_CHECKPOINT_SIGNATURES - 1,
        )?;

        reset_verify_der_call_count();
        verifier.verify_digest(&subject, &padded_bundle)?;
        let padded_verify_calls = verify_der_call_count();

        if padded_verify_calls != genuine_verify_calls {
            return Err(format!(
                "non-matching checkpoint key hints caused extra ECDSA verification: genuine={genuine_verify_calls}, padded={padded_verify_calls}"
            )
            .into());
        }
        Ok(())
    }

    #[test]
    fn blank_signature_lines_do_not_count_toward_the_limit()
    -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let proof = tlog_entry
            .inclusion_proof
            .as_ref()
            .ok_or("missing inclusion proof")?;
        let (log, log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;

        // The limit bounds verifications, not bytes: blank lines are
        // skipped before any decode, so they must not trip it.
        let (body, genuine_signatures) = proof
            .checkpoint
            .envelope
            .split_once("\n\n")
            .ok_or("missing blank line separating body from signatures")?;
        let padded = format!(
            "{body}\n\n{}{genuine_signatures}",
            "\n".repeat(limits::MAX_CHECKPOINT_SIGNATURES * 4)
        );
        verify_checkpoint(&padded, proof.tree_size, &proof.root_hash, log, &log_key)?;
        Ok(())
    }

    // -------------------------------------------------------------
    // Negative: time window
    // -------------------------------------------------------------

    #[test]
    fn integrated_time_outside_synthetic_log_key_window_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let leaf_info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        let synthetic_window = ValidityPeriod {
            start: 0,
            end: Some(1),
        };
        match check_time_window(
            tlog_entry.integrated_time,
            &synthetic_window,
            leaf_info.not_before,
            leaf_info.not_after,
        ) {
            Err(Error::Timestamp(TimestampError::IntegratedTimeOutsideLogKeyValidity)) => Ok(()),
            other => {
                Err(format!("expected IntegratedTimeOutsideLogKeyValidity, got {other:?}").into())
            }
        }
    }

    #[test]
    fn integrated_time_outside_synthetic_certificate_window_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let trust_store = embedded_trust_store()?;
        let tlog_entry = &bundle.verification_material.tlog_entries[0];
        let (log, _log_key) = select_log_key(&trust_store, &tlog_entry.log_id_key_id)?;
        match check_time_window(tlog_entry.integrated_time, &log.public_key.valid_for, 0, 1) {
            Err(Error::Timestamp(TimestampError::IntegratedTimeOutsideCertificateValidity)) => {
                Ok(())
            }
            other => Err(format!(
                "expected IntegratedTimeOutsideCertificateValidity, got {other:?}"
            )
            .into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: composed function's own bundle-level checks
    // -------------------------------------------------------------

    #[test]
    fn no_tlog_entries_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = mutate_bundle_json(|v| {
            v["verificationMaterial"]["tlogEntries"] = serde_json::Value::Array(vec![]);
            Ok(())
        })?;
        let trust_store = embedded_trust_store()?;
        match verify_tlog_entry(&bundle, &trust_store, &fixture_origin_policy(&trust_store)?) {
            Err(Error::Transparency(TransparencyError::NoTlogEntries)) => Ok(()),
            other => Err(format!("expected NoTlogEntries, got {other:?}").into()),
        }
    }

    #[test]
    fn mislabeled_trust_root_key_id_still_verifies_via_recomputed_digest()
    -> Result<(), Box<dyn std::error::Error>> {
        // Parse the public-good trust root fixture as JSON.
        let mut trust_root_value: serde_json::Value =
            serde_json::from_slice(&read_fixture("trusted-roots/public-good.json")?)?;

        // Find the rekor.sigstore.dev v1 log (baseUrl == "https://rekor.sigstore.dev").
        let tlog_index = trust_root_value["tlogs"]
            .as_array()
            .ok_or("tlogs is not an array")?
            .iter()
            .position(|log| {
                log["baseUrl"]
                    .as_str()
                    .is_some_and(|url| url == "https://rekor.sigstore.dev")
            })
            .ok_or("rekor.sigstore.dev log not found")?;

        // Replace tlogs[tlog_index].logId.keyId with 32 bytes of 0xAA (base64-encoded).
        let mislabeled_key_id = STANDARD.encode([0xAAu8; 32]);
        trust_root_value["tlogs"][tlog_index]["logId"]["keyId"] =
            serde_json::Value::String(mislabeled_key_id);

        // Build a TrustStore from the mutated root.
        let mislabeled_trust_store =
            TrustStore::from_json(&serde_json::to_vec(&trust_root_value)?)?;

        // Verify that verify_tlog_entry still succeeds: select_log_key recomputes the
        // SHA-256 from the key material itself, not the mislabeled logId.keyId, so the
        // real golden bundle still verifies. The SET verification uses the recomputed
        // digest, not the trust store's label.
        let bundle = real_bundle()?;
        let result = verify_tlog_entry(
            &bundle,
            &mislabeled_trust_store,
            &fixture_origin_policy(&mislabeled_trust_store)?,
        )?;

        if result.integrated_time != 1_783_027_755 {
            return Err(format!("unexpected integratedTime: {}", result.integrated_time).into());
        }
        Ok(())
    }
}
