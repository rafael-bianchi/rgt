//! Mechanical error taxonomy.
//!
//! Every error states what check failed, never what an attacker may have
//! intended. Each top-level variant wraps its own `#[non_exhaustive]` enum so
//! new failure modes can be added without a semver break. See DESIGN.md,
//! "Core decisions" item 8.

use thiserror::Error;

/// Top-level error type for this crate.
///
/// Each variant corresponds to one stage of parsing or (eventually)
/// verification. The taxonomy is intentionally mechanical: it says what
/// check failed, not what an attacker may have intended (no `Tampered`
/// variant, no inferred staleness).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A parser rejected malformed or non-conforming input.
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    /// The input uses a format, media type, or version this crate does not
    /// (yet) support.
    #[error("unsupported: {0}")]
    Unsupported(#[from] UnsupportedError),

    /// A trust-root or trust-material check failed.
    #[error("trust error: {0}")]
    Trust(#[from] TrustError),

    /// An X.509 certificate failed validation.
    #[error("certificate error: {0}")]
    Certificate(#[from] CertificateError),

    /// A transparency-log (Rekor) check failed.
    #[error("transparency error: {0}")]
    Transparency(#[from] TransparencyError),

    /// An RFC 3161 timestamp check failed.
    #[error("timestamp error: {0}")]
    Timestamp(#[from] TimestampError),

    /// The subject digest did not bind to the signed statement.
    #[error("content binding error: {0}")]
    ContentBinding(#[from] ContentBindingError),

    /// The caller's identity policy was not satisfied.
    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),

    /// An input exceeded a hard-coded resource limit.
    #[error("resource limit exceeded: {0}")]
    ResourceLimit(#[from] ResourceLimitError),
}

/// Failures while parsing a bundle, statement, or trusted-root document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    /// The input was not well-formed JSON, or did not match the expected
    /// shape once parsed.
    #[error("invalid JSON: {0}")]
    Json(String),

    /// A field that must be strict base64 (standard alphabet, padded) was
    /// not valid base64.
    #[error("invalid base64 in field `{field}`: {reason}")]
    Base64 {
        /// Name of the field that failed to decode.
        field: &'static str,
        /// Human-readable reason the decode failed.
        reason: String,
    },

    /// A field that must be strict hex was not valid hex, or had the wrong
    /// length.
    #[error("invalid hex in field `{field}`: {reason}")]
    Hex {
        /// Name of the field that failed to decode.
        field: &'static str,
        /// Human-readable reason the decode failed.
        reason: String,
    },

    /// A digest string was the wrong length, wrong case set, or contained
    /// non-hex characters.
    #[error("malformed digest: {0}")]
    MalformedDigest(String),

    /// An integer field that protojson encodes as a JSON string (`int64`,
    /// `uint64`) was not a valid decimal integer.
    #[error("field `{field}` is not a valid integer: {value}")]
    NotAnInteger {
        /// Name of the field that failed to parse.
        field: &'static str,
        /// The raw string value that failed to parse as an integer.
        value: String,
    },

    /// The DSSE envelope did not carry exactly one signature.
    #[error("dsse envelope has {count} signatures, expected exactly 1")]
    DsseSignatureCount {
        /// Number of signatures actually present.
        count: usize,
    },

    /// A required field was absent.
    #[error("missing required field `{0}`")]
    MissingField(&'static str),

    /// A container format (JSONL, API response) had no usable entries.
    #[error("empty container: {0}")]
    EmptyContainer(&'static str),

    /// An in-toto statement subject had an empty or otherwise unusable
    /// digest map.
    #[error("statement subject has no usable digest: {0}")]
    MalformedSubject(String),

    /// A field that must be an RFC 3339 timestamp did not parse as one.
    #[error("invalid RFC 3339 timestamp in field `{field}`: {reason}")]
    Rfc3339 {
        /// Name of the field that failed to parse.
        field: &'static str,
        /// Human-readable reason the timestamp was rejected.
        reason: String,
    },

    /// A Rekor checkpoint (signed note) envelope did not match the
    /// expected line structure: `<origin>\n<treeSize>\n<rootHash>\n`,
    /// optional extension lines, a blank line, then one or more `— <name>
    /// <base64>` signature lines.
    #[error("malformed checkpoint envelope: {0}")]
    Checkpoint(String),
}

/// The input is well-formed but uses a format, version, or shape this crate
/// does not implement.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum UnsupportedError {
    /// `mediaType` did not match any media type this crate understands.
    #[error("unsupported media type: {found}")]
    MediaType {
        /// The media-type string found in the input.
        found: String,
    },

    /// A container (e.g. the GitHub attestations API response) had entries
    /// but none carried an inline bundle.
    #[error("no inline bundles in response (bundles must be fetched out of band)")]
    BundlesNotInline,

    /// A Rekor transparency-log entry used an unsupported `kind`/`version`
    /// pair.
    #[error("unsupported tlog entry kind/version: {kind}/{version}")]
    TlogEntryKindVersion {
        /// The `kind` field found in the entry.
        kind: String,
        /// The `version` field found in the entry.
        version: String,
    },

    /// A DSSE envelope's `payloadType` was not the in-toto media type
    /// (`application/vnd.in-toto+json`). The payload type is covered by
    /// the PAE signature, so requiring it is what makes a signature over
    /// some other application's payload type unusable here: without the
    /// check, an envelope a policy-trusted identity signed under a
    /// different protocol could be replayed as an in-toto attestation.
    #[error("unsupported dsse payload type: {found}")]
    DssePayloadType {
        /// The `payloadType` string found in the envelope.
        found: String,
    },

    /// An in-toto statement's `_type` field did not match a statement
    /// version this crate understands.
    #[error("unsupported in-toto statement type: {found}")]
    StatementType {
        /// The `_type` string found in the input.
        found: String,
    },

    /// A signed statement's `predicateType` was not on this crate's
    /// supported allow-list. v0.1 verifies exactly one predicate type
    /// (`https://slsa.dev/provenance/v1`); GitHub's own release predicate
    /// (`https://in-toto.io/attestation/release/v0.2`) and any other
    /// predicate type are rejected here rather than verified and silently
    /// misinterpreted.
    #[error("unsupported predicate type: {found}")]
    PredicateType {
        /// The `predicateType` string found in the statement.
        found: String,
    },

    /// A bundle carried more than one transparency-log entry. This crate's
    /// verification chain checks exactly one; selecting among several
    /// candidate entries is unimplemented (no real-world fixture has been
    /// observed with more than one).
    #[error("bundle has {count} tlog entries, expected exactly 1")]
    MultipleTlogEntries {
        /// The actual number of tlog entries found.
        count: usize,
    },
}

/// Trust-root or trust-material failures.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TrustError {
    /// No key in the trust store matched the identifier the bundle
    /// referenced.
    #[error("unknown trust-store key id")]
    UnknownKeyId,

    /// A key existed but was not valid at the required authenticated time.
    #[error("no trusted key valid at the required time")]
    NoTrustedKeyValidAt,
}

/// X.509 certificate validation failures.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CertificateError {
    /// The certificate could not be parsed as DER.
    #[error("certificate is not valid DER: {0}")]
    InvalidDer(String),

    /// A `SubjectPublicKeyInfo` (from a leaf certificate or a trust-store
    /// key) used a key algorithm/curve this crate does not implement.
    /// Only P-256 and P-384 ECDSA are supported.
    #[error("unsupported public key algorithm")]
    UnsupportedKeyAlgorithm,

    /// A certificate's declared signature algorithm was not ECDSA with
    /// SHA-256 (P-256) or ECDSA with SHA-384 (P-384) -- the only
    /// algorithms DESIGN.md's "X.509 / Fulcio validation profile" allows.
    #[error("unsupported certificate signature algorithm")]
    UnsupportedSignatureAlgorithm,

    /// No certificate authority in the trust store produced a valid chain
    /// from the leaf to a self-signed root. Bundle-supplied roots are
    /// never trusted, so this is also the error for a leaf whose issuer
    /// matches no trust-store entry at all.
    #[error("no trusted certificate authority validated this certificate chain")]
    UntrustedCertificate,

    /// A certificate's signature did not cryptographically verify against
    /// its issuer's public key.
    #[error("certificate signature did not verify")]
    SignatureInvalid,

    /// A certificate's `issuer` did not DER-match its issuer certificate's
    /// `subject`.
    #[error("certificate issuer does not match issuing certificate's subject")]
    IssuerNameMismatch,

    /// The authenticated time fell outside a certificate's
    /// `[notBefore, notAfter]` validity window.
    #[error("authenticated time is outside the certificate's validity window")]
    OutsideCertificateValidity,

    /// The authenticated time fell outside the trust-store certificate
    /// authority entry's `validFor` window.
    #[error("authenticated time is outside the trusted certificate authority's validFor window")]
    OutsideCaValidity,

    /// A certificate's `basicConstraints` did not match what its role in
    /// the chain requires: a certificate authority must be `CA:TRUE`, and
    /// the leaf must have `basicConstraints` absent or `CA:FALSE`.
    #[error("certificate basicConstraints does not match its role in the chain")]
    InvalidBasicConstraints,

    /// A certificate authority's `pathLenConstraint` was violated by the
    /// number of certificate authorities subordinate to it in the chain.
    #[error("certificate authority pathLenConstraint exceeded")]
    PathLengthExceeded,

    /// A certificate's `keyUsage` extension was absent, or lacked the bit
    /// its role in the chain requires (`digitalSignature` on the leaf,
    /// `keyCertSign` on each certificate authority).
    #[error("certificate keyUsage is missing or lacks the required bit")]
    MissingKeyUsage,

    /// The leaf certificate's `extKeyUsage` extension was absent, or did
    /// not contain `codeSigning` (1.3.6.1.5.5.7.3.3).
    #[error("leaf certificate extKeyUsage does not contain codeSigning")]
    MissingCodeSigningEku,

    /// The leaf certificate carried a critical extension this crate does
    /// not recognize. RFC 5280 SS4.2 requires rejecting certificates with
    /// critical extensions a validator does not understand.
    #[error("leaf certificate has an unrecognized critical extension")]
    UnknownCriticalExtension,

    /// An extension this crate models by OID appeared more than once on a
    /// certificate.
    #[error("certificate has a duplicate extension")]
    DuplicateExtension,

    /// The leaf certificate carried no embedded SCT (Signed Certificate
    /// Timestamp) list extension, or the list was empty.
    #[error("leaf certificate has no embedded SCT")]
    SctMissing,

    /// No embedded SCT verified against any trusted CT log key.
    #[error("no embedded SCT verified against a trusted CT log key")]
    SctInvalid,

    /// An SCT's `logId` did not match any CT log in the trust store.
    #[error("SCT logId does not match any trusted CT log")]
    UnknownCtLog,

    /// An SCT's `timestamp` fell outside its matched CT log key's
    /// `validFor` window.
    #[error("SCT timestamp is outside the trusted CT log key's validFor window")]
    SctOutsideKeyValidity,
}

/// Transparency-log (Rekor) verification failures.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TransparencyError {
    /// The Merkle inclusion proof did not verify against the checkpoint,
    /// or the proof's `logIndex`/`treeSize`/hash count were inconsistent
    /// (including over- or under-length hash lists).
    #[error("inclusion proof did not verify")]
    InclusionProofInvalid,

    /// The bundle carried no transparency-log entries to verify.
    #[error("bundle has no transparency-log entries")]
    NoTlogEntries,

    /// No key in the trust store's transparency logs had a SHA-256 digest
    /// of its `SubjectPublicKeyInfo` matching the entry's `logId.keyId`.
    #[error("no trusted log key matches this entry's logId")]
    UnknownLogKey,

    /// The tlog entry carried no inclusion promise (SET). A SET is
    /// required: an inclusion proof alone does not authenticate
    /// `integratedTime` (DESIGN.md "Time-evidence model").
    #[error("tlog entry has no inclusion promise (SET)")]
    SetMissing,

    /// The inclusion promise (SET) did not verify against the selected
    /// trusted log key.
    #[error("inclusion promise (SET) did not verify")]
    SetInvalid,

    /// The tlog entry carried no Merkle inclusion proof.
    #[error("tlog entry has no inclusion proof")]
    InclusionProofMissing,

    /// The checkpoint's tree size did not match the inclusion proof's
    /// tree size.
    #[error("checkpoint treeSize does not match the inclusion proof's treeSize")]
    CheckpointTreeSizeMismatch,

    /// The checkpoint's root hash did not match the inclusion proof's
    /// root hash.
    #[error("checkpoint rootHash does not match the inclusion proof's rootHash")]
    CheckpointRootHashMismatch,

    /// No signature line in the checkpoint verified against the selected
    /// trusted log key.
    #[error("no checkpoint signature verified with the trusted log key")]
    CheckpointSignatureInvalid,

    /// The checkpoint signature authenticated successfully, but its opaque
    /// origin was not allowed for the selected trusted log-key identity.
    ///
    /// This intentionally carries no origin or key material so an attacker
    /// cannot use verification errors as an oracle for policy contents.
    #[error("checkpoint origin is not allowed for the trusted log key")]
    CheckpointOriginMismatch,
}

/// RFC 3161 timestamp verification failures.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TimestampError {
    /// The RFC 3161 timestamp token did not parse or verify.
    #[error("rfc3161 timestamp invalid")]
    Invalid,

    /// The authenticated `integratedTime` fell outside the selected
    /// trusted log key's `validFor` window.
    #[error("integratedTime is outside the trusted log key's validity window")]
    IntegratedTimeOutsideLogKeyValidity,

    /// The authenticated `integratedTime` fell outside the leaf
    /// certificate's `[notBefore, notAfter]` validity window.
    #[error("integratedTime is outside the leaf certificate's validity window")]
    IntegratedTimeOutsideCertificateValidity,
}

/// The signed content did not bind to the requested subject.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContentBindingError {
    /// The requested subject digest was not present among the statement's
    /// subjects.
    #[error("subject digest not found in statement")]
    SubjectNotFound,

    /// A DSSE envelope's signature did not verify against the leaf
    /// certificate's public key over the PAE-encoded payload.
    #[error("dsse envelope signature did not verify")]
    DsseSignatureInvalid,

    /// The Rekor entry's own `kind`/`apiVersion` (inside the
    /// canonicalized body) did not match the bundle's `kindVersion`
    /// metadata for the same tlog entry.
    #[error("tlog entry kindVersion does not match the canonicalized body's kind/apiVersion")]
    TlogEntryKindVersionMismatch,

    /// The Rekor entry's recorded signature did not match the bundle's
    /// DSSE signature.
    #[error("tlog entry signature does not match the bundle's DSSE signature")]
    TlogEntrySignatureMismatch,

    /// The Rekor entry's recorded verifier certificate did not match the
    /// bundle's leaf certificate.
    #[error("tlog entry verifier certificate does not match the bundle's leaf certificate")]
    TlogEntryCertificateMismatch,

    /// The Rekor entry's recorded payload hash did not match the SHA-256
    /// of the bundle's decoded DSSE payload.
    #[error("tlog entry payloadHash does not match the bundle's DSSE payload")]
    TlogEntryPayloadHashMismatch,

    /// The Rekor entry's recorded envelope hash did not match this
    /// crate's recomputation of it from the bundle's DSSE envelope.
    #[error("tlog entry envelopeHash does not match the bundle's DSSE envelope")]
    TlogEntryEnvelopeHashMismatch,
}

/// The caller-supplied identity policy was not satisfied.
///
/// Matching (`crate::policy_match`) never infers what a mismatch might
/// mean; every variant below names exactly which comparison failed, and
/// (where applicable) carries the `expected` policy value alongside the
/// `found` authenticated certificate value (DESIGN.md "Core decisions"
/// item 8).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PolicyError {
    /// A policy builder was given an empty or malformed value.
    #[error("invalid policy configuration: {0}")]
    InvalidConfiguration(String),

    /// A Fulcio identity claim needed to evaluate the policy was absent
    /// from the certificate (its extension was not present).
    #[error("certificate is missing the `{claim}` identity claim required to evaluate policy")]
    MissingIdentityClaim {
        /// The name of the missing claim (e.g. `"source_repository_uri"`).
        claim: &'static str,
    },

    /// A Fulcio identity claim was present but not in the shape this crate
    /// requires to evaluate policy (e.g. the source repository URI or the
    /// SAN URI did not match the expected `https://github.com/...` shape).
    #[error("certificate's `{claim}` identity claim is malformed: {reason}")]
    MalformedIdentityClaim {
        /// The name of the malformed claim.
        claim: &'static str,
        /// Human-readable reason the claim was rejected.
        reason: String,
    },

    /// The certificate's authenticated OIDC issuer did not match the
    /// pinned GitHub Actions issuer.
    #[error("issuer mismatch: expected {expected}, found {found}")]
    IssuerMismatch {
        /// The pinned issuer this crate requires.
        expected: String,
        /// The issuer found in the certificate.
        found: String,
    },

    /// The certificate's authenticated source repository (`owner/name`)
    /// did not match the policy's required source repository.
    #[error("source repository mismatch: expected {expected}, found {found}")]
    SourceRepositoryMismatch {
        /// The `owner/name` the policy requires.
        expected: String,
        /// The `owner/name` found in the certificate.
        found: String,
    },

    /// The certificate's authenticated source repository owner id did not
    /// match the policy's pinned owner id.
    #[error("source repository owner id mismatch: expected {expected}, found {found}")]
    SourceOwnerIdMismatch {
        /// The decimal owner id the policy requires.
        expected: String,
        /// The decimal owner id found in the certificate.
        found: String,
    },

    /// The certificate's authenticated source repository id did not match
    /// the policy's pinned repository id.
    #[error("source repository id mismatch: expected {expected}, found {found}")]
    SourceRepositoryIdMismatch {
        /// The decimal repository id the policy requires.
        expected: String,
        /// The decimal repository id found in the certificate.
        found: String,
    },

    /// The certificate's authenticated source ref did not satisfy the
    /// policy's ref requirement (`Exact` or `Glob`).
    #[error("source ref mismatch: expected {expected}, found {found}")]
    SourceRefMismatch {
        /// The policy's required ref or ref-glob pattern.
        expected: String,
        /// The ref found in the certificate.
        found: String,
    },

    /// The certificate's authenticated source commit did not match the
    /// policy's required commit.
    #[error("source commit mismatch: expected {expected}, found {found}")]
    SourceCommitMismatch {
        /// The commit sha (lowercase hex) the policy requires.
        expected: String,
        /// The commit sha found in the certificate.
        found: String,
    },

    /// The certificate's authenticated signer-workflow repository
    /// (`owner/name`) did not match the policy's required repository.
    #[error("signer repository mismatch: expected {expected}, found {found}")]
    SignerRepositoryMismatch {
        /// The `owner/name` the policy requires.
        expected: String,
        /// The `owner/name` found in the certificate's SAN.
        found: String,
    },

    /// The certificate's authenticated signer-workflow path did not match
    /// the policy's required path.
    #[error("signer workflow path mismatch: expected {expected}, found {found}")]
    SignerWorkflowPathMismatch {
        /// The workflow path the policy requires.
        expected: String,
        /// The workflow path found in the certificate's SAN.
        found: String,
    },

    /// The certificate's authenticated signer-workflow revision did not
    /// satisfy the policy's revision requirement (`Ref` or `Sha`).
    #[error("signer workflow revision mismatch: expected {expected}, found {found}")]
    SignerRevisionMismatch {
        /// The ref or commit sha the policy requires.
        expected: String,
        /// The ref or digest found in the certificate.
        found: String,
    },
}

/// A hard resource limit (size, count, or depth) was exceeded. These limits
/// exist to keep parsing cost bounded and independent of attacker-controlled
/// input shape.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResourceLimitError {
    /// The raw input exceeded the maximum accepted byte length.
    #[error("input of {actual} bytes exceeds the {limit}-byte limit")]
    InputTooLarge {
        /// The actual input length in bytes.
        actual: usize,
        /// The maximum accepted length in bytes.
        limit: usize,
    },

    /// A JSON document parsed to more `serde_json::Value` nodes than the
    /// limit. Carries no actual count: parsing stops at the limit rather
    /// than walking the rest of the input to total it up.
    #[error("JSON document exceeds the limit of {limit} value nodes")]
    TooManyJsonNodes {
        /// The maximum accepted number of value nodes.
        limit: usize,
    },

    /// A bundle carried more transparency-log entries than the limit.
    #[error("{actual} tlog entries exceeds the limit of {limit}")]
    TooManyTlogEntries {
        /// The actual number of entries found.
        actual: usize,
        /// The maximum accepted number of entries.
        limit: usize,
    },

    /// An in-toto statement carried more subjects than the limit.
    #[error("{actual} statement subjects exceeds the limit of {limit}")]
    TooManySubjects {
        /// The actual number of subjects found.
        actual: usize,
        /// The maximum accepted number of subjects.
        limit: usize,
    },

    /// A `BundleSet` container carried more bundles than the limit.
    #[error("{actual} bundles exceeds the limit of {limit} per BundleSet")]
    TooManyBundles {
        /// The actual number of bundles found.
        actual: usize,
        /// The maximum accepted number of bundles.
        limit: usize,
    },

    /// A certificate chain carried more certificates than the limit.
    #[error("{actual} certificates exceeds the limit of {limit} per chain")]
    TooManyCertificates {
        /// The actual number of certificates found.
        actual: usize,
        /// The maximum accepted number of certificates.
        limit: usize,
    },

    /// A checkpoint carried more signature lines than the limit.
    #[error("{actual} checkpoint signature lines exceeds the limit of {limit}")]
    TooManyCheckpointSignatures {
        /// The actual number of signature lines found.
        actual: usize,
        /// The maximum accepted number of signature lines.
        limit: usize,
    },
}
