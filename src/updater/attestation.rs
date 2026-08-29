//! GitHub Artifact Attestation fetch + in-binary verification.
//!
//! The updater verifies a release's Sigstore bundle — an independent trust
//! anchor from the release's own assets — before installing (FR-003). The
//! bundle is fetched from the GitHub Attestations API, snappy-decompressed,
//! and verified in-binary via `attestation-verify` against the embedded
//! Sigstore public-good trust root: DSSE signature, Fulcio certificate chain,
//! embedded SCT, Rekor v1 SET + Merkle inclusion + checkpoint, and the
//! `rafael-bianchi/rgt` workflow policy.

use serde::Deserialize;

use attestation_verify::{
    Bundle, CheckpointOriginPolicy, GithubPolicy, RefPolicy, RepositoryIdentity, SignerPolicy,
    SourcePolicy, Subject, TrustStore, Verifier, WorkflowPath, WorkflowRevisionPolicy,
};

/// The repo whose attestations `rgt update` accepts.
pub const RGT_OWNER: &str = "rafael-bianchi";
pub const RGT_REPO: &str = "rgt";
pub const RGT_OWNER_ID: u64 = 19_198_157;
pub const RGT_REPO_ID: u64 = 1_318_560_598;
/// The workflow that signs RGT release artifacts.
pub const RGT_SIGNER_WORKFLOW: &str = ".github/workflows/release.yml";
/// The checkpoint origin of the Sigstore public-good Rekor v1 log.
const REKOR_V1_ORIGIN: &str = "rekor.sigstore.dev - 1193050959916656506";

/// The `GET /repos/{owner}/{repo}/attestations/sha256:<digest>` response shape.
#[derive(Debug, Deserialize)]
struct AttestationsResponse {
    #[serde(default)]
    attestations: Vec<AttestationEntry>,
}

#[derive(Debug, Deserialize)]
struct AttestationEntry {
    bundle_url: String,
}

/// Builds the identity policy RGT enforces: the artifact must be attested by a
/// `rafael-bianchi/rgt` workflow (release workflow), on any ref. The source
/// repository is pinned by numeric owner/repo id; the signer repository is
/// name-matched only (a Fulcio certificate carries no signer id claim).
pub fn rgt_policy() -> Result<GithubPolicy, String> {
    let source_repository = RepositoryIdentity::parse(&format!("{RGT_OWNER}/{RGT_REPO}"))
        .map_err(|e| format!("invalid repository identity: {e}"))?
        .with_owner_id(RGT_OWNER_ID)
        .with_repository_id(RGT_REPO_ID);
    let signer_repository = RepositoryIdentity::parse(&format!("{RGT_OWNER}/{RGT_REPO}"))
        .map_err(|e| format!("invalid repository identity: {e}"))?;
    GithubPolicy::builder()
        .source(SourcePolicy {
            repository: source_repository,
            git_ref: RefPolicy::Glob("refs/*".to_owned()),
            commit: None,
        })
        .signer(SignerPolicy {
            repository: signer_repository,
            path: WorkflowPath::new(RGT_SIGNER_WORKFLOW)
                .map_err(|e| format!("invalid workflow path: {e}"))?,
            revision: WorkflowRevisionPolicy::Any,
        })
        .build()
        .map_err(|e| format!("failed to build attestation policy: {e}"))
}

/// Fetches the snappy-compressed Sigstore bundle for an artifact digest.
///
/// `subject_digest` is the `sha256:<hex>` form used by the GitHub API. Returns
/// the raw (compressed) `bundle_url` body, or a refusal-worthy error when no
/// attestation exists (FR-003: missing attestation).
pub fn fetch_bundle(subject_digest: &str, token: Option<&str>) -> Result<Vec<u8>, String> {
    let url = format!(
        "https://api.github.com/repos/{RGT_OWNER}/{RGT_REPO}/attestations/{subject_digest}"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(30))
        .timeout_write(std::time::Duration::from_secs(30))
        .build();
    let mut req = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", "rgt-updater/1.0");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {}", t));
    }
    let response = req
        .call()
        .map_err(|e| format!("failed to fetch attestation list: {e}"))?;
    let body = response
        .into_string()
        .map_err(|e| format!("failed to read attestation list: {e}"))?;
    let parsed: AttestationsResponse = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse attestation list: {e}"))?;
    let entry = parsed
        .attestations
        .into_iter()
        .next()
        .ok_or_else(|| "no attestation found for this release artifact".to_string())?;

    let bundle_resp = agent
        .get(&entry.bundle_url)
        .set("User-Agent", "rgt-updater/1.0")
        .call()
        .map_err(|e| format!("failed to fetch attestation bundle: {e}"))?;
    let mut reader = bundle_resp.into_reader();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)
        .map_err(|e| format!("failed to read attestation bundle: {e}"))?;
    Ok(bytes)
}

/// Snappy-decompresses a `bundle_url` body into the Sigstore bundle JSON.
pub fn decompress_bundle(compressed: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = snap::raw::Decoder::new();
    decoder
        .decompress_vec(compressed)
        .map_err(|e| format!("failed to decompress attestation bundle: {e}"))
}

/// Verifies a Sigstore bundle against `digest` (hex SHA-256, no `sha256:`
/// prefix) under `policy`, using the embedded public-good trust root.
///
/// Returns `Ok(())` only when the full chain verifies: DSSE signature, Fulcio
/// chain, SCT, Rekor v1 SET + inclusion + checkpoint, and the identity policy.
pub fn verify_bundle(
    bundle_json: &[u8],
    digest: &str,
    policy: &GithubPolicy,
) -> Result<(), String> {
    let bundle = Bundle::from_json(bundle_json)
        .map_err(|e| format!("failed to parse attestation bundle: {e}"))?;
    let subject =
        Subject::from_digest_hex(digest).map_err(|e| format!("invalid artifact digest: {e}"))?;
    let trust_store = TrustStore::embedded_public_good()
        .map_err(|e| format!("failed to load trusted root: {e}"))?;
    let log = trust_store
        .tlogs
        .first()
        .ok_or_else(|| "trusted root has no Rekor log".to_string())?;
    let origin_policy = CheckpointOriginPolicy::builder()
        .allow_origin(log, REKOR_V1_ORIGIN)
        .map_err(|e| format!("invalid checkpoint origin policy: {e}"))?
        .build()
        .map_err(|e| format!("invalid checkpoint origin policy: {e}"))?;
    let verifier = Verifier::builder()
        .trust_store(trust_store)
        .github_policy(policy.clone())
        .checkpoint_origin_policy(origin_policy)
        .build()
        .map_err(|e| format!("failed to build attestation verifier: {e}"))?;
    verifier
        .verify_digest(&subject, &bundle)
        .map(|_| ())
        .map_err(|e| format!("attestation verification failed: {e}"))
}

/// Hex SHA-256 of a file on disk (the digest the attestation must cover).
pub fn file_sha256_hex(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        use std::io::Read;
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
