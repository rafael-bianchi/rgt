//! End-to-end verification-chain tests (DESIGN.md "Testing strategy"):
//! the flagship positive test asserts every [`VerificationReport`] field
//! against the real `cli/cli` v2.96.0 golden fixture, and each negative
//! targets one specific chain step, asserting the precise error category
//! (and, where the chain's fixed step order determines it, the exact
//! variant) it must fail with.
//!
//! Lighter smoke tests for the same fixtures live in
//! `tests/parse_fixtures.rs`; this file owns the exhaustive matrix.

use base64::{Engine as _, engine::general_purpose::STANDARD};

use attestation_verify::{
    Bundle, BundleSet, CheckpointOriginPolicy, ContentBindingError, Error, GithubPolicy,
    PolicyError, RefPolicy, RepositoryIdentity, SignerPolicy, SourcePolicy, Subject,
    TransparencyError, TrustStore, UnsupportedError, Verifier, WorkflowPath,
    WorkflowRevisionPolicy,
};

const GOLDEN_FIXTURE: &str = "github-cli/tarball-user-slsa-provenance.json";
const TARBALL_DIGEST_FIXTURE: &str = "github-cli/gh_2.96.0_linux_amd64.tar.gz.sha256";
const REKOR_V1_ORIGIN: &str = "rekor.sigstore.dev - 1193050959916656506";

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

fn read_fixture(relative: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(std::fs::read(fixture_path(relative))?)
}

fn read_fixture_string(relative: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(fixture_path(relative))?)
}

fn read_fixture_digest(relative: &str) -> Result<Subject, Box<dyn std::error::Error>> {
    let hex_digest = read_fixture_string(relative)?;
    Ok(Subject::from_digest_hex(hex_digest.trim())?)
}

fn real_bundle() -> Result<Bundle, Box<dyn std::error::Error>> {
    Ok(Bundle::from_json(&read_fixture(GOLDEN_FIXTURE)?)?)
}

fn tarball_digest() -> Result<Subject, Box<dyn std::error::Error>> {
    read_fixture_digest(TARBALL_DIGEST_FIXTURE)
}

// ---------------------------------------------------------------------
// Policy construction: a "correct" cli/cli source/signer pair (matching
// the golden fixture's real, empirically-confirmed identity -- see
// src/fulcio.rs's own tests), plus small per-test variations built by
// cloning and overriding one field.
// ---------------------------------------------------------------------

fn correct_source_policy() -> Result<SourcePolicy, Box<dyn std::error::Error>> {
    Ok(SourcePolicy {
        repository: RepositoryIdentity::parse("cli/cli")?
            .with_owner_id(59_704_711)
            .with_repository_id(212_613_049),
        git_ref: RefPolicy::Exact("refs/heads/trunk".to_owned()),
        commit: None,
    })
}

fn correct_signer_policy() -> Result<SignerPolicy, Box<dyn std::error::Error>> {
    Ok(SignerPolicy {
        repository: RepositoryIdentity::parse("cli/cli")?,
        path: WorkflowPath::new(".github/workflows/deployment.yml")?,
        revision: WorkflowRevisionPolicy::Any,
    })
}

fn build_policy(
    source: SourcePolicy,
    signer: SignerPolicy,
) -> Result<GithubPolicy, Box<dyn std::error::Error>> {
    Ok(GithubPolicy::builder()
        .source(source)
        .signer(signer)
        .build()?)
}

fn correct_policy() -> Result<GithubPolicy, Box<dyn std::error::Error>> {
    build_policy(correct_source_policy()?, correct_signer_policy()?)
}

fn verifier_with(policy: GithubPolicy) -> Result<Verifier, Box<dyn std::error::Error>> {
    let trust_store = TrustStore::embedded_public_good()?;
    let checkpoint_origin_policy = origin_policy(&trust_store, &[REKOR_V1_ORIGIN])?;
    verifier_with_origin_policy(policy, trust_store, checkpoint_origin_policy)
}

fn verifier_with_origin_policy(
    policy: GithubPolicy,
    trust_store: TrustStore,
    checkpoint_origin_policy: CheckpointOriginPolicy,
) -> Result<Verifier, Box<dyn std::error::Error>> {
    Ok(Verifier::builder()
        .trust_store(trust_store)
        .github_policy(policy)
        .checkpoint_origin_policy(checkpoint_origin_policy)
        .build()?)
}

fn origin_policy(
    trust_store: &TrustStore,
    origins: &[&str],
) -> Result<CheckpointOriginPolicy, Box<dyn std::error::Error>> {
    let log = trust_store
        .tlogs
        .first()
        .ok_or("missing trusted Rekor log")?;
    origin_policy_for_log(log, origins)
}

fn origin_policy_for_log(
    log: &attestation_verify::TransparencyLog,
    origins: &[&str],
) -> Result<CheckpointOriginPolicy, Box<dyn std::error::Error>> {
    let mut builder = CheckpointOriginPolicy::builder();
    for origin in origins {
        builder = builder.allow_origin(log, *origin)?;
    }
    Ok(builder.build()?)
}

fn verifier_with_correct_policy() -> Result<Verifier, Box<dyn std::error::Error>> {
    verifier_with(correct_policy()?)
}

/// Parses the golden fixture as JSON, lets `f` mutate it, then
/// re-serializes and re-parses as a [`Bundle`] -- mirrors `rekor.rs`'s and
/// `x509.rs`'s own `mutate_*_json` test helpers.
fn bundle_from_mutated_json(
    f: impl FnOnce(&mut serde_json::Value) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<Bundle, Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_slice(&read_fixture(GOLDEN_FIXTURE)?)?;
    f(&mut value)?;
    Ok(Bundle::from_json(&serde_json::to_vec(&value)?)?)
}

// ---------------------------------------------------------------------
// Positive: exhaustive field-by-field assertion against the real
// cli/cli v2.96.0 fixture.
// ---------------------------------------------------------------------

#[test]
// Deliberately one long, flat list of assertions rather than several
// smaller test functions or helper-extracted checks: the point of this
// test is to be an exhaustive, at-a-glance checklist of every
// `VerificationReport` field's exact expected value.
#[allow(clippy::too_many_lines)]
fn verifies_real_cli_cli_bundle_with_every_report_field_exact()
-> Result<(), Box<dyn std::error::Error>> {
    let verifier = verifier_with_correct_policy()?;
    let bundle = real_bundle()?;
    let digest = tarball_digest()?;

    let report = verifier.verify_digest(&digest, &bundle)?;

    if report.subject.digest != digest {
        return Err("report.subject.digest did not round-trip the requested digest".into());
    }
    if report.subject.name.as_deref() != Some("gh_2.96.0_linux_amd64.tar.gz") {
        return Err(format!("unexpected subject.name: {:?}", report.subject.name).into());
    }

    if report.signer.issuer != "https://token.actions.githubusercontent.com" {
        return Err(format!("unexpected signer.issuer: {}", report.signer.issuer).into());
    }
    if report.signer.source_repository != "cli/cli" {
        return Err(format!(
            "unexpected signer.source_repository: {}",
            report.signer.source_repository
        )
        .into());
    }
    if report.signer.source_ref != "refs/heads/trunk" {
        return Err(format!("unexpected signer.source_ref: {}", report.signer.source_ref).into());
    }
    if report.signer.signer_repository != "cli/cli" {
        return Err(format!(
            "unexpected signer.signer_repository: {}",
            report.signer.signer_repository
        )
        .into());
    }
    if report.signer.signer_workflow_path != ".github/workflows/deployment.yml" {
        return Err(format!(
            "unexpected signer.signer_workflow_path: {}",
            report.signer.signer_workflow_path
        )
        .into());
    }

    if report.transparency.log_index != 2_049_189_324 {
        return Err(format!(
            "unexpected transparency.log_index: {}",
            report.transparency.log_index
        )
        .into());
    }
    if report.transparency.integrated_time != 1_783_027_755 {
        return Err(format!(
            "unexpected transparency.integrated_time: {}",
            report.transparency.integrated_time
        )
        .into());
    }

    if report.statement.predicate_type != "https://slsa.dev/provenance/v1" {
        return Err(format!(
            "unexpected statement.predicate_type: {}",
            report.statement.predicate_type
        )
        .into());
    }
    let expected_predicate = serde_json::json!({
        "buildDefinition": {
            "buildType": "https://actions.github.io/buildtypes/workflow/v1",
            "externalParameters": {
                "workflow": {
                    "ref": "refs/heads/trunk",
                    "repository": "https://github.com/cli/cli",
                    "path": ".github/workflows/deployment.yml"
                }
            },
            "internalParameters": {
                "github": {
                    "event_name": "workflow_dispatch",
                    "repository_id": "212613049",
                    "repository_owner_id": "59704711",
                    "runner_environment": "github-hosted"
                }
            },
            "resolvedDependencies": [
                {
                    "uri": "git+https://github.com/cli/cli@refs/heads/trunk",
                    "digest": {
                        "gitCommit": "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca0"
                    }
                }
            ]
        },
        "runDetails": {
            "builder": {
                "id": "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk"
            },
            "metadata": {
                "invocationId": "https://github.com/cli/cli/actions/runs/28622199740/attempts/1"
            }
        }
    });
    if report.statement.predicate != expected_predicate {
        return Err(format!(
            "unexpected statement.predicate: {}",
            report.statement.predicate
        )
        .into());
    }

    let expected_fingerprint =
        Subject::sha256_of(&read_fixture("trusted-roots/public-good.json")?).to_hex();
    if report.trust.fingerprint != expected_fingerprint {
        return Err(format!(
            "unexpected trust.fingerprint: {} (expected {expected_fingerprint})",
            report.trust.fingerprint
        )
        .into());
    }
    if report.trust.source != "embedded-public-good" {
        return Err(format!("unexpected trust.source: {}", report.trust.source).into());
    }

    Ok(())
}

#[test]
fn wrong_full_checkpoint_origin_is_an_authenticated_policy_mismatch()
-> Result<(), Box<dyn std::error::Error>> {
    let trust_store = TrustStore::embedded_public_good()?;
    let origin_policy = origin_policy(&trust_store, &["rekor.sigstore.dev - 1"])?;
    let verifier = verifier_with_origin_policy(correct_policy()?, trust_store, origin_policy)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Transparency(TransparencyError::CheckpointOriginMismatch)) => Ok(()),
        other => Err(format!("expected CheckpointOriginMismatch, got {other:?}").into()),
    }
}

#[test]
fn host_only_checkpoint_origin_is_not_normalized_or_accepted()
-> Result<(), Box<dyn std::error::Error>> {
    let trust_store = TrustStore::embedded_public_good()?;
    let origin_policy = origin_policy(&trust_store, &["rekor.sigstore.dev"])?;
    let verifier = verifier_with_origin_policy(correct_policy()?, trust_store, origin_policy)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Transparency(TransparencyError::CheckpointOriginMismatch)) => Ok(()),
        other => Err(format!("expected CheckpointOriginMismatch, got {other:?}").into()),
    }
}

#[test]
fn multiple_allowed_origins_for_one_key_are_supported() -> Result<(), Box<dyn std::error::Error>> {
    let trust_store = TrustStore::embedded_public_good()?;
    let origin_policy = origin_policy(&trust_store, &["unused alternate origin", REKOR_V1_ORIGIN])?;
    let verifier = verifier_with_origin_policy(correct_policy()?, trust_store, origin_policy)?;
    verifier.verify_digest(&tarball_digest()?, &real_bundle()?)?;
    Ok(())
}

#[test]
fn base_url_mutation_does_not_change_authenticated_origin_acceptance()
-> Result<(), Box<dyn std::error::Error>> {
    let mut trust_root: serde_json::Value =
        serde_json::from_slice(&read_fixture("trusted-roots/public-good.json")?)?;
    trust_root["tlogs"][0]["baseUrl"] =
        serde_json::Value::String("https://attacker-controlled.invalid/not-normative".to_owned());
    let trust_store = TrustStore::from_json(&serde_json::to_vec(&trust_root)?)?;
    let checkpoint_origin_policy = origin_policy(&trust_store, &[REKOR_V1_ORIGIN])?;
    let verifier =
        verifier_with_origin_policy(correct_policy()?, trust_store, checkpoint_origin_policy)?;
    verifier.verify_digest(&tarball_digest()?, &real_bundle()?)?;
    Ok(())
}

#[test]
fn authenticated_origin_bound_to_wrong_known_log_key_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let trust_store = TrustStore::embedded_public_good()?;
    let checkpoint_origin_policy =
        origin_policy_for_log(&trust_store.tlogs[1], &[REKOR_V1_ORIGIN])?;
    let verifier =
        verifier_with_origin_policy(correct_policy()?, trust_store, checkpoint_origin_policy)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Transparency(TransparencyError::CheckpointOriginMismatch)) => Ok(()),
        other => Err(format!("expected CheckpointOriginMismatch, got {other:?}").into()),
    }
}

#[test]
fn mutating_signed_checkpoint_origin_stays_signature_invalid()
-> Result<(), Box<dyn std::error::Error>> {
    let bundle = bundle_from_mutated_json(|value| {
        let envelope = value["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]
            ["checkpoint"]["envelope"]
            .as_str()
            .ok_or("missing checkpoint envelope")?;
        let (_origin, remainder) = envelope.split_once('\n').ok_or("missing origin line")?;
        value["verificationMaterial"]["tlogEntries"][0]["inclusionProof"]["checkpoint"]["envelope"] =
            serde_json::Value::String(format!("tampered-origin\n{remainder}"));
        Ok(())
    })?;
    let verifier = verifier_with_correct_policy()?;

    match verifier.verify_digest(&tarball_digest()?, &bundle) {
        Err(Error::Transparency(TransparencyError::CheckpointSignatureInvalid)) => Ok(()),
        other => Err(format!("expected CheckpointSignatureInvalid, got {other:?}").into()),
    }
}

#[test]
fn missing_checkpoint_origin_policy_fails_verifier_build() -> Result<(), Box<dyn std::error::Error>>
{
    match Verifier::builder()
        .trust_store(TrustStore::embedded_public_good()?)
        .github_policy(correct_policy()?)
        .build()
    {
        Err(Error::Policy(PolicyError::InvalidConfiguration(_))) => Ok(()),
        other => Err(format!("expected missing-origin-policy build error, got {other:?}").into()),
    }
}

#[test]
fn unknown_checkpoint_origin_policy_key_fails_verifier_build()
-> Result<(), Box<dyn std::error::Error>> {
    let mut trust_store = TrustStore::embedded_public_good()?;
    let unknown_log = trust_store.tlogs.remove(0);
    let checkpoint_origin_policy = origin_policy_for_log(&unknown_log, &[REKOR_V1_ORIGIN])?;
    match Verifier::builder()
        .trust_store(trust_store)
        .github_policy(correct_policy()?)
        .checkpoint_origin_policy(checkpoint_origin_policy)
        .build()
    {
        Err(Error::Policy(PolicyError::InvalidConfiguration(_))) => Ok(()),
        other => Err(format!("expected unknown-origin-key build error, got {other:?}").into()),
    }
}

// ---------------------------------------------------------------------
// The v0.2 boundary: GitHub's own TSA release-attestation flavor is
// deterministically rejected, not silently skipped or misverified.
// ---------------------------------------------------------------------

#[test]
fn github_tsa_release_bundle_is_unsupported_predicate_type()
-> Result<(), Box<dyn std::error::Error>> {
    // `gh_2.96.0_checksums.txt`'s only available bundle is GitHub's own
    // release attestation (`initiator: github`): predicate
    // `https://in-toto.io/attestation/release/v0.2`, zero tlog entries,
    // RFC 3161 TSA timestamp instead -- v0.2 scope, not verified by this
    // crate. Verifying it must fail deterministically at the
    // predicate-type allow-list (chain step 3), before subject binding
    // or the (absent) tlog entry are ever examined -- regardless of
    // which policy is used, since policy is checked last (step 10).
    let verifier = verifier_with_correct_policy()?;
    let set = BundleSet::from_json_lines(&read_fixture("github-cli/checksums-gh-download.jsonl")?)?;
    let bundle = set
        .bundles
        .into_iter()
        .next()
        .ok_or("expected at least one bundle in the JSONL fixture")?;
    let checksums_bytes = read_fixture("github-cli/gh_2.96.0_checksums.txt")?;

    match verifier.verify_bytes(&checksums_bytes, &bundle) {
        Err(Error::Unsupported(UnsupportedError::PredicateType { found })) => {
            if found == "https://in-toto.io/attestation/release/v0.2" {
                Ok(())
            } else {
                Err(format!("unexpected predicateType: {found}").into())
            }
        }
        other => Err(format!("expected Unsupported(PredicateType), got {other:?}").into()),
    }
}

// ---------------------------------------------------------------------
// Negatives: each targets one specific chain step.
// ---------------------------------------------------------------------

#[test]
fn non_in_toto_dsse_payload_type_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // The payload type is covered by the DSSE PAE signature, so an
    // envelope a trusted identity signed under a different application's
    // payload type must not be accepted here just because its payload
    // parses as an in-toto statement (cross-protocol replay).
    let bundle = bundle_from_mutated_json(|v| {
        v["dsseEnvelope"]["payloadType"] =
            serde_json::Value::String("application/vnd.example.other+json".to_owned());
        Ok(())
    })?;
    let verifier = verifier_with_correct_policy()?;

    // Chain step 1 runs before any signature verification, so the
    // mutation's now-stale DSSE signature cannot be what this fails on.
    match verifier.verify_digest(&tarball_digest()?, &bundle) {
        Err(Error::Unsupported(UnsupportedError::DssePayloadType { found })) => {
            if found == "application/vnd.example.other+json" {
                Ok(())
            } else {
                Err(format!("unexpected payloadType: {found}").into())
            }
        }
        other => Err(format!("expected Unsupported(DssePayloadType), got {other:?}").into()),
    }
}

#[test]
fn wrong_digest_fails_subject_binding() -> Result<(), Box<dyn std::error::Error>> {
    let verifier = verifier_with_correct_policy()?;
    let bundle = real_bundle()?;
    // `gh_2.96.0_checksums.txt`'s digest is not among this bundle's 21
    // subjects (confirmed by `parse_fixtures.rs`'s own
    // `tarball_user_slsa_provenance_parses` test).
    let wrong_digest = read_fixture_digest("github-cli/gh_2.96.0_checksums.txt.sha256")?;

    match verifier.verify_digest(&wrong_digest, &bundle) {
        Err(Error::ContentBinding(ContentBindingError::SubjectNotFound)) => Ok(()),
        other => Err(format!("expected ContentBinding(SubjectNotFound), got {other:?}").into()),
    }
}

#[test]
fn policy_owner_id_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourcePolicy {
        repository: RepositoryIdentity::parse("cli/cli")?
            .with_owner_id(1) // real owner id is 59_704_711
            .with_repository_id(212_613_049),
        ..correct_source_policy()?
    };
    let verifier = verifier_with(build_policy(source, correct_signer_policy()?)?)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Policy(PolicyError::SourceOwnerIdMismatch { expected, found })) => {
            if expected == "1" && found == "59704711" {
                Ok(())
            } else {
                Err(format!("unexpected expected/found: {expected}/{found}").into())
            }
        }
        other => Err(format!("expected Policy(SourceOwnerIdMismatch), got {other:?}").into()),
    }
}

#[test]
fn policy_repository_name_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourcePolicy {
        repository: RepositoryIdentity::parse("cli/wrong-name")?
            .with_owner_id(59_704_711)
            .with_repository_id(212_613_049),
        ..correct_source_policy()?
    };
    let verifier = verifier_with(build_policy(source, correct_signer_policy()?)?)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Policy(PolicyError::SourceRepositoryMismatch { .. })) => Ok(()),
        other => Err(format!("expected Policy(SourceRepositoryMismatch), got {other:?}").into()),
    }
}

#[test]
fn policy_source_ref_exact_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // The real ref is `refs/heads/trunk` (this fixture's workflow ran via
    // `workflow_dispatch`, not a `v2.96.0` tag push -- see src/fulcio.rs's
    // tests), so an exact-tag policy must be rejected, not silently
    // accepted by name/id matching alone.
    let source = SourcePolicy {
        git_ref: RefPolicy::Exact("refs/tags/v2.96.0".to_owned()),
        ..correct_source_policy()?
    };
    let verifier = verifier_with(build_policy(source, correct_signer_policy()?)?)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Policy(PolicyError::SourceRefMismatch { expected, found })) => {
            if expected == "refs/tags/v2.96.0" && found == "refs/heads/trunk" {
                Ok(())
            } else {
                Err(format!("unexpected expected/found: {expected}/{found}").into())
            }
        }
        other => Err(format!("expected Policy(SourceRefMismatch), got {other:?}").into()),
    }
}

#[test]
fn policy_source_ref_glob_matching_pattern_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourcePolicy {
        git_ref: RefPolicy::Glob("refs/heads/*".to_owned()),
        ..correct_source_policy()?
    };
    let verifier = verifier_with(build_policy(source, correct_signer_policy()?)?)?;
    verifier.verify_digest(&tarball_digest()?, &real_bundle()?)?;
    Ok(())
}

#[test]
fn policy_source_ref_glob_nonmatching_pattern_fails() -> Result<(), Box<dyn std::error::Error>> {
    let source = SourcePolicy {
        git_ref: RefPolicy::Glob("refs/tags/*".to_owned()),
        ..correct_source_policy()?
    };
    let verifier = verifier_with(build_policy(source, correct_signer_policy()?)?)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Policy(PolicyError::SourceRefMismatch { .. })) => Ok(()),
        other => Err(format!("expected Policy(SourceRefMismatch), got {other:?}").into()),
    }
}

#[test]
fn policy_signer_workflow_path_mismatch_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let signer = SignerPolicy {
        path: WorkflowPath::new(".github/workflows/other.yml")?,
        ..correct_signer_policy()?
    };
    let verifier = verifier_with(build_policy(correct_source_policy()?, signer)?)?;

    match verifier.verify_digest(&tarball_digest()?, &real_bundle()?) {
        Err(Error::Policy(PolicyError::SignerWorkflowPathMismatch { expected, found })) => {
            if expected == ".github/workflows/other.yml"
                && found == ".github/workflows/deployment.yml"
            {
                Ok(())
            } else {
                Err(format!("unexpected expected/found: {expected}/{found}").into())
            }
        }
        other => Err(format!("expected Policy(SignerWorkflowPathMismatch), got {other:?}").into()),
    }
}

#[test]
fn tampered_dsse_payload_byte_fails_at_tlog_entry_binding_before_dsse_verify()
-> Result<(), Box<dyn std::error::Error>> {
    // Flip a character inside a subject *name* (not the digest under
    // test, and not JSON structure) so the payload decodes to different
    // bytes while remaining well-formed JSON with the same predicate type
    // and the same tarball digest among its subjects -- isolating the
    // tamper to exactly "the payload bytes changed" rather than also
    // breaking JSON parsing or subject lookup.
    let bundle = bundle_from_mutated_json(|v| {
        let payload_b64 = v["dsseEnvelope"]["payload"]
            .as_str()
            .ok_or("missing dsseEnvelope.payload")?
            .to_owned();
        let mut statement: serde_json::Value =
            serde_json::from_slice(&STANDARD.decode(payload_b64)?)?;
        let name = statement["subject"][0]["name"]
            .as_str()
            .ok_or("missing subject[0].name")?
            .to_owned();
        statement["subject"][0]["name"] = serde_json::Value::String(format!("{name}-tampered"));
        let mutated_payload = serde_json::to_vec(&statement)?;
        v["dsseEnvelope"]["payload"] = serde_json::Value::String(STANDARD.encode(mutated_payload));
        Ok(())
    })?;
    let verifier = verifier_with_correct_policy()?;

    // This crate's chain order checks the tlog entry's recorded payload
    // hash (step 5, `check_entry_binding`) before the DSSE signature
    // itself (step 7) -- both are `ContentBindingError`, but the specific
    // variant that fires is `TlogEntryPayloadHashMismatch`, not
    // `DsseSignatureInvalid`: the tampered payload's hash no longer
    // matches the Rekor entry's original, untouched `payloadHash`, and
    // that mismatch is caught before the DSSE-signature step is ever
    // reached.
    match verifier.verify_digest(&tarball_digest()?, &bundle) {
        Err(Error::ContentBinding(ContentBindingError::TlogEntryPayloadHashMismatch)) => Ok(()),
        other => Err(format!(
            "expected ContentBinding(TlogEntryPayloadHashMismatch), got {other:?}"
        )
        .into()),
    }
}

#[test]
fn altered_integrated_time_fails_set_verification() -> Result<(), Box<dyn std::error::Error>> {
    let bundle = bundle_from_mutated_json(|v| {
        v["verificationMaterial"]["tlogEntries"][0]["integratedTime"] =
            serde_json::Value::String("1783027756".to_owned());
        Ok(())
    })?;
    let verifier = verifier_with_correct_policy()?;

    match verifier.verify_digest(&tarball_digest()?, &bundle) {
        Err(Error::Transparency(TransparencyError::SetInvalid)) => Ok(()),
        other => Err(format!("expected Transparency(SetInvalid), got {other:?}").into()),
    }
}

#[test]
fn zero_tlog_trust_store_fails_origin_policy_build() -> Result<(), Box<dyn std::error::Error>> {
    // `trusted-roots/github.json` carries zero transparency logs. A
    // non-empty checkpoint-origin policy cannot name a key in that root, so
    // the new fail-fast builder contract rejects this configuration before
    // any bundle verification starts.
    let public_good = TrustStore::embedded_public_good()?;
    let checkpoint_origin_policy =
        origin_policy_for_log(&public_good.tlogs[0], &[REKOR_V1_ORIGIN])?;
    let verifier = Verifier::builder()
        .trust_store(TrustStore::from_json(&read_fixture(
            "trusted-roots/github.json",
        )?)?)
        .github_policy(correct_policy()?)
        .checkpoint_origin_policy(checkpoint_origin_policy)
        .build();

    match verifier {
        Err(Error::Policy(PolicyError::InvalidConfiguration(_))) => Ok(()),
        other => Err(format!("expected policy build failure, got {other:?}").into()),
    }
}
