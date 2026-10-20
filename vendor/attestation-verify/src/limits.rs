//! Hard resource limits enforced by every parser.
//!
//! These bound parsing cost independent of what an attacker-controlled input
//! shape looks like. They are intentionally generous relative to real-world
//! fixtures (the golden fixture carries 21 subjects and 1 tlog entry) but
//! tight enough to make unbounded-allocation attacks impossible.

/// Maximum accepted length, in bytes, for any single `from_json` /
/// `from_json_lines` / `from_github_response` input.
pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// Maximum number of `serde_json::Value` nodes in a single strict-parsed
/// JSON document (every scalar, every array element, every object member
/// value, and every container itself).
///
/// [`MAX_INPUT_BYTES`] alone does not bound the parsed tree: a `Value` node
/// costs on the order of 50-100 bytes, so a compact input of scalars (`0,`
/// is two bytes) expands by more than an order of magnitude. Unknown fields
/// are tolerated by design, which makes that expansion reachable on an
/// otherwise valid bundle. This cap bounds the tree to tens of megabytes.
/// The golden fixture parses to a few thousand nodes, so the headroom is as
/// generous as the other limits here.
pub const MAX_JSON_NODES: usize = 262_144;

/// Maximum number of transparency-log entries in a single bundle.
pub const MAX_TLOG_ENTRIES: usize = 32;

/// Maximum number of in-toto statement subjects.
pub const MAX_STATEMENT_SUBJECTS: usize = 1024;

/// Maximum number of bundles collected into a single `BundleSet`.
pub const MAX_BUNDLES_PER_SET: usize = 64;

/// Maximum number of certificates in a single certificate chain.
pub const MAX_CERTIFICATES_PER_CHAIN: usize = 128;

/// Maximum number of signature lines in a single checkpoint (signed note).
///
/// Tighter than the other limits on purpose: this one bounds *cryptographic*
/// work, not allocation. `verify_checkpoint` tries every signature line
/// against the selected log key, and the signature lines are covered by
/// neither the SET nor the inclusion proof, so their count is attacker-
/// controlled on an otherwise genuine bundle. Real checkpoints carry one
/// signature (the log's own); the headroom is for a log co-signed by a
/// witness quorum.
pub const MAX_CHECKPOINT_SIGNATURES: usize = 32;
