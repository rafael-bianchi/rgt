//! Verify GitHub Artifact Attestations (Sigstore bundles) offline, at
//! runtime, in Rust, with a minimal and auditable dependency footprint.
//!
//! See `DESIGN.md` in the repository root for the full design.
//!
//! # Status: functional, narrow scope
//!
//! This crate implements a complete offline verification chain — DSSE
//! signature, Rekor v1 transparency-log inclusion (SET + Merkle proof +
//! checkpoint), X.509/Fulcio chain validation, embedded SCT verification,
//! and GitHub identity-policy matching — for one bundle shape:
//! `actions/attest-build-provenance`-style bundles carrying a
//! `https://slsa.dev/provenance/v1` predicate, verified against the
//! embedded Sigstore public-good trust root or a caller-supplied one.
//! [`Verifier::verify_digest`] / [`Verifier::verify_bytes`] report success
//! only once every step of that chain has verified; the caller supplies an
//! identity policy and an exact checkpoint-origin policy (DESIGN.md "Core
//! decisions" item 2).
//!
//! Out of scope for now (typed [`UnsupportedError`], never silently
//! accepted): GitHub's own TSA-timestamped release-attestation flavor
//! (`initiator: github`, no tlog entries), Rekor v2 (Ed25519,
//! tiles/sharded logs), and any predicate type other than SLSA provenance
//! v1. This crate has not been independently audited; see `README.md` for
//! the exact verified scope and `DESIGN.md` for the full design and
//! roadmap.
//!
//! # Layout
//!
//! - [`Subject`]: the artifact digest being verified.
//! - [`Bundle`] / [`BundleSet`]: parsed Sigstore bundles, and the two
//!   container shapes GitHub serves them in.
//! - [`Statement`]: the in-toto statement inside a bundle's DSSE payload.
//! - [`TrustStore`]: a parsed trusted-root document.
//! - [`GithubPolicy`]: the identity policy a [`Verifier`] enforces.
//! - [`CheckpointOriginPolicy`]: exact signed checkpoint origins bound to
//!   trusted log-key SPKI digests.
//! - [`Verifier`]: the verification entry point;
//!   [`Verifier::verify_digest`] / [`Verifier::verify_bytes`] return a
//!   [`verifier::VerificationReport`] on success.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod error;
pub mod policy;
pub mod statement;
pub mod trust;
pub mod verifier;

mod dsse;
mod fulcio;
mod limits;
mod parse_util;
mod policy_match;
mod rekor;
mod sct;
mod strict_json;
mod subject;
mod time;
mod x509;

#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;

pub use bundle::{
    AttestationEntry, BUNDLE_MEDIA_TYPE, Bundle, BundleSet, Certificate, Checkpoint, DsseEnvelope,
    DsseSignature, InclusionPromise, InclusionProof, Rfc3161Timestamp, TimestampVerificationData,
    TlogEntry, VerificationMaterial,
};
pub use error::{
    CertificateError, ContentBindingError, Error, ParseError, PolicyError, ResourceLimitError,
    TimestampError, TransparencyError, TrustError, UnsupportedError,
};
pub use policy::{
    CheckpointOriginBinding, CheckpointOriginPolicy, CheckpointOriginPolicyBuilder, CommitSha,
    GithubPolicy, GithubPolicyBuilder, RefPolicy, RepositoryIdentity, SignerPolicy, SourcePolicy,
    WorkflowPath, WorkflowRevisionPolicy,
};
pub use statement::{STATEMENT_TYPE, Statement, StatementSubject};
pub use subject::Subject;
pub use trust::{
    CaSubject, CertificateAuthority, CtLog, PublicKey, TRUSTED_ROOT_MEDIA_TYPE, TransparencyLog,
    TrustStore, ValidityPeriod,
};
pub use verifier::{
    TrustSnapshotInfo, VerificationReport, VerifiedCertificateIdentity, VerifiedSignedStatement,
    VerifiedSubject, VerifiedTransparency, Verifier, VerifierBuilder,
};
