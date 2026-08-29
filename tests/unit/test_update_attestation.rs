//! Attestation verification against a real, captured GitHub Actions
//! `attest-build-provenance` Sigstore bundle (from `cli/cli` v2.96.0 —
//! the crate's own golden fixture, public data).

use attestation_verify::{
    GithubPolicy, RefPolicy, RepositoryIdentity, SignerPolicy, SourcePolicy, WorkflowPath,
    WorkflowRevisionPolicy,
};
use rgt::updater::attestation::{decompress_bundle, verify_bundle};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/github-cli")
        .join(relative)
}

fn read_fixture_string(relative: &str) -> String {
    std::fs::read_to_string(fixture_path(relative)).unwrap()
}

/// The identity policy matching the fixture's real, empirically-confirmed
/// `cli/cli` identity (see the crate's own e2e tests).
fn cli_cli_policy() -> GithubPolicy {
    GithubPolicy::builder()
        .source(SourcePolicy {
            repository: RepositoryIdentity::parse("cli/cli")
                .unwrap()
                .with_owner_id(59_704_711)
                .with_repository_id(212_613_049),
            git_ref: RefPolicy::Exact("refs/heads/trunk".to_owned()),
            commit: None,
        })
        .signer(SignerPolicy {
            repository: RepositoryIdentity::parse("cli/cli").unwrap(),
            path: WorkflowPath::new(".github/workflows/deployment.yml").unwrap(),
            revision: WorkflowRevisionPolicy::Any,
        })
        .build()
        .unwrap()
}

fn tarball_digest_hex() -> String {
    read_fixture_string("gh_2.96.0_linux_amd64.tar.gz.sha256")
        .trim()
        .to_string()
}

#[test]
fn verifies_real_attest_build_provenance_bundle() {
    let bundle_json = std::fs::read(fixture_path("tarball-user-slsa-provenance.json")).unwrap();
    let digest = tarball_digest_hex();
    verify_bundle(&bundle_json, &digest, &cli_cli_policy()).expect("bundle must verify");
}

#[test]
fn tampered_digest_is_rejected() {
    let bundle_json = std::fs::read(fixture_path("tarball-user-slsa-provenance.json")).unwrap();
    // The checksums digest is NOT among this bundle's subjects.
    let wrong_digest = read_fixture_string("gh_2.96.0_checksums.txt.sha256")
        .trim()
        .to_string();
    assert!(
        verify_bundle(&bundle_json, &wrong_digest, &cli_cli_policy()).is_err(),
        "a digest not covered by the bundle must fail verification"
    );
}

#[test]
fn decompress_round_trips_snappy_payload() {
    // Encode something with snappy then confirm decompress_bundle recovers it.
    let payload = br#"{"hello":"world"}"#;
    let compressed = snap::raw::Encoder::new().compress_vec(payload).unwrap();
    let out = decompress_bundle(&compressed).unwrap();
    assert_eq!(out, payload);
}

#[test]
fn rgt_policy_targets_rgt_repo() {
    let policy = rgt::updater::attestation::rgt_policy().unwrap();
    let s = format!("{policy:?}");
    assert!(s.contains("rafael-bianchi") || s.contains("rgt"), "{s}");
}
