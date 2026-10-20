//! Fixture-based integration tests: parse real bundles, real trust roots,
//! and real digests captured from `cli/cli` v2.96.0 (see
//! `tests/fixtures/README.md`), plus targeted negative/mutation tests for
//! the hardening rules in DESIGN.md "Core decisions" item 2.

use base64::{Engine as _, engine::general_purpose::STANDARD};

use attestation_verify::{
    BUNDLE_MEDIA_TYPE, Bundle, BundleSet, CheckpointOriginPolicy, ContentBindingError, Error,
    GithubPolicy, ParseError, RefPolicy, RepositoryIdentity, ResourceLimitError, SignerPolicy,
    SourcePolicy, Subject, TrustStore, UnsupportedError, Verifier, WorkflowPath,
    WorkflowRevisionPolicy,
};

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

/// Reads a `.sha256` fixture (a bare 64-char hex digest, possibly with
/// trailing whitespace) as a [`Subject`].
fn read_fixture_digest(relative: &str) -> Result<Subject, Box<dyn std::error::Error>> {
    let hex_digest = read_fixture_string(relative)?;
    Ok(Subject::from_digest_hex(hex_digest.trim())?)
}

// ---------------------------------------------------------------------
// Golden fixtures
// ---------------------------------------------------------------------

#[test]
fn tarball_user_slsa_provenance_parses() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let bundle = Bundle::from_json(&bytes)?;

    if bundle.media_type != BUNDLE_MEDIA_TYPE {
        return Err(format!("unexpected media type: {}", bundle.media_type).into());
    }

    let entries = &bundle.verification_material.tlog_entries;
    if entries.len() != 1 {
        return Err(format!("expected 1 tlog entry, got {}", entries.len()).into());
    }
    let entry = &entries[0];
    if entry.kind != "dsse" || entry.version != "0.0.1" {
        return Err(format!("unexpected kind/version: {}/{}", entry.kind, entry.version).into());
    }
    if entry.inclusion_promise.is_none() {
        return Err("expected an inclusion promise (SET)".into());
    }
    let Some(inclusion_proof) = &entry.inclusion_proof else {
        return Err("expected an inclusion proof".into());
    };
    if inclusion_proof.checkpoint.envelope.is_empty() {
        return Err("expected a non-empty checkpoint envelope".into());
    }
    if entry.integrated_time != 1_783_027_755 {
        return Err(format!("unexpected integratedTime: {}", entry.integrated_time).into());
    }
    if entry.log_index != 2_049_189_324 {
        return Err(format!("unexpected logIndex: {}", entry.log_index).into());
    }

    let statement = bundle.statement()?;
    if statement.predicate_type != "https://slsa.dev/provenance/v1" {
        return Err(format!("unexpected predicateType: {}", statement.predicate_type).into());
    }
    if statement.subjects.len() != 21 {
        return Err(format!("expected 21 subjects, got {}", statement.subjects.len()).into());
    }

    let tarball_digest = read_fixture_digest("github-cli/gh_2.96.0_linux_amd64.tar.gz.sha256")?;
    if !statement.contains_subject(&tarball_digest) {
        return Err("expected the tarball digest to be found among subjects".into());
    }

    let checksums_digest = read_fixture_digest("github-cli/gh_2.96.0_checksums.txt.sha256")?;
    if statement.contains_subject(&checksums_digest) {
        return Err("expected the checksums.txt digest to NOT be found among subjects".into());
    }

    Ok(())
}

#[test]
fn tarball_github_release_tsa_parses() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-github-release-tsa.json")?;
    let bundle = Bundle::from_json(&bytes)?;

    if !bundle.verification_material.tlog_entries.is_empty() {
        return Err(format!(
            "expected 0 tlog entries, got {}",
            bundle.verification_material.tlog_entries.len()
        )
        .into());
    }
    let timestamps = &bundle
        .verification_material
        .timestamp_verification_data
        .rfc3161_timestamps;
    if timestamps.len() != 1 {
        return Err(format!("expected 1 rfc3161 timestamp, got {}", timestamps.len()).into());
    }

    let statement = bundle.statement()?;
    if statement.predicate_type != "https://in-toto.io/attestation/release/v0.2" {
        return Err(format!("unexpected predicateType: {}", statement.predicate_type).into());
    }

    Ok(())
}

#[test]
fn checksums_gh_download_jsonl_has_one_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/checksums-gh-download.jsonl")?;
    let set = BundleSet::from_json_lines(&bytes)?;
    if set.bundles.len() != 1 {
        return Err(format!("expected 1 bundle, got {}", set.bundles.len()).into());
    }
    if !set.entries.is_empty() {
        return Err("JSONL bundles should carry no acquisition metadata entries".into());
    }
    Ok(())
}

#[test]
fn attestations_api_response_reports_bundles_not_inline() -> Result<(), Box<dyn std::error::Error>>
{
    let bytes = read_fixture("github-cli/attestations-api-response.redacted.json")?;
    match BundleSet::from_github_response(&bytes) {
        Err(Error::Unsupported(UnsupportedError::BundlesNotInline)) => Ok(()),
        other => Err(format!("expected BundlesNotInline error, got {other:?}").into()),
    }
}

#[test]
fn public_good_trust_root_parses() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("trusted-roots/public-good.json")?;
    let store = TrustStore::from_json(&bytes)?;

    if store.certificate_authorities.len() != 2 {
        return Err(format!(
            "expected 2 CAs, got {}",
            store.certificate_authorities.len()
        )
        .into());
    }
    if store.tlogs.len() != 2 {
        return Err(format!("expected 2 tlogs, got {}", store.tlogs.len()).into());
    }
    let has_ed25519 = store
        .tlogs
        .iter()
        .any(|tlog| tlog.public_key.key_details.contains("ED25519"));
    if !has_ed25519 {
        return Err("expected one tlog with keyDetails containing ED25519".into());
    }
    if store.ctlogs.len() != 2 {
        return Err(format!("expected 2 ctlogs, got {}", store.ctlogs.len()).into());
    }

    Ok(())
}

#[test]
fn github_trust_root_parses() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("trusted-roots/github.json")?;
    let store = TrustStore::from_json(&bytes)?;

    if store.certificate_authorities.len() != 6 {
        return Err(format!(
            "expected 6 CAs, got {}",
            store.certificate_authorities.len()
        )
        .into());
    }
    if !store.tlogs.is_empty() {
        return Err(format!("expected 0 tlogs, got {}", store.tlogs.len()).into());
    }
    if store.timestamp_authorities.len() != 6 {
        return Err(format!(
            "expected 6 timestamp authorities, got {}",
            store.timestamp_authorities.len()
        )
        .into());
    }
    if !store.ctlogs.is_empty() {
        return Err(format!("expected 0 ctlogs, got {}", store.ctlogs.len()).into());
    }

    Ok(())
}

#[test]
fn embedded_public_good_equals_fixture_file() -> Result<(), Box<dyn std::error::Error>> {
    let embedded = TrustStore::embedded_public_good()?;
    let bytes = read_fixture("trusted-roots/public-good.json")?;
    let from_fixture = TrustStore::from_json(&bytes)?;
    if embedded != from_fixture {
        return Err(
            "embedded_public_good() does not match parsing the fixture file directly".into(),
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Verifier: wiring sanity (the exhaustive positive/negative chain matrix
// lives in tests/verify_e2e.rs; these two are a light smoke test that the
// Verifier's public API and this fixture set actually plug together).
// ---------------------------------------------------------------------

/// A policy that actually matches `github-cli/tarball-user-slsa-provenance.json`
/// (real `cli/cli` v2.96.0 workflow-provenance fixture).
fn matching_cli_cli_policy() -> Result<GithubPolicy, Box<dyn std::error::Error>> {
    let source = SourcePolicy {
        repository: RepositoryIdentity::parse("cli/cli")?
            .with_owner_id(59_704_711)
            .with_repository_id(212_613_049),
        git_ref: RefPolicy::Exact("refs/heads/trunk".to_owned()),
        commit: None,
    };
    let signer = SignerPolicy {
        repository: RepositoryIdentity::parse("cli/cli")?,
        path: WorkflowPath::new(".github/workflows/deployment.yml")?,
        revision: WorkflowRevisionPolicy::Any,
    };
    Ok(GithubPolicy::builder()
        .source(source)
        .signer(signer)
        .build()?)
}

#[test]
fn verifier_succeeds_on_real_bundle_with_matching_policy() -> Result<(), Box<dyn std::error::Error>>
{
    let trust_store = TrustStore::embedded_public_good()?;
    let checkpoint_origin_policy = CheckpointOriginPolicy::builder()
        .allow_origin(
            trust_store
                .tlogs
                .first()
                .ok_or("missing trusted Rekor log")?,
            REKOR_V1_ORIGIN,
        )?
        .build()?;
    let verifier = Verifier::builder()
        .trust_store(trust_store)
        .github_policy(matching_cli_cli_policy()?)
        .checkpoint_origin_policy(checkpoint_origin_policy)
        .build()?;

    let bundle = Bundle::from_json(&read_fixture(
        "github-cli/tarball-user-slsa-provenance.json",
    )?)?;
    let digest = read_fixture_digest("github-cli/gh_2.96.0_linux_amd64.tar.gz.sha256")?;

    let report = verifier.verify_digest(&digest, &bundle)?;
    if report.signer.source_repository != "cli/cli" {
        return Err(format!(
            "unexpected source_repository: {}",
            report.signer.source_repository
        )
        .into());
    }
    // Exhaustive field-by-field assertions live in tests/verify_e2e.rs.
    Ok(())
}

#[test]
fn verify_bytes_fails_closed_when_hashed_artifact_is_not_a_subject()
-> Result<(), Box<dyn std::error::Error>> {
    // `gh_2.96.0_checksums.txt` is not among
    // `tarball-user-slsa-provenance.json`'s 21 subjects (confirmed by
    // `tarball_user_slsa_provenance_parses` above), regardless of policy:
    // `verify_bytes` must still fail closed on subject binding.
    let trust_store = TrustStore::embedded_public_good()?;
    let checkpoint_origin_policy = CheckpointOriginPolicy::builder()
        .allow_origin(
            trust_store
                .tlogs
                .first()
                .ok_or("missing trusted Rekor log")?,
            REKOR_V1_ORIGIN,
        )?
        .build()?;
    let verifier = Verifier::builder()
        .trust_store(trust_store)
        .github_policy(matching_cli_cli_policy()?)
        .checkpoint_origin_policy(checkpoint_origin_policy)
        .build()?;

    let bundle = Bundle::from_json(&read_fixture(
        "github-cli/tarball-user-slsa-provenance.json",
    )?)?;
    let checksums_bytes = read_fixture("github-cli/gh_2.96.0_checksums.txt")?;

    match verifier.verify_bytes(&checksums_bytes, &bundle) {
        Err(Error::ContentBinding(ContentBindingError::SubjectNotFound)) => Ok(()),
        other => Err(format!("expected ContentBinding(SubjectNotFound), got {other:?}").into()),
    }
}

// ---------------------------------------------------------------------
// Negatives
// ---------------------------------------------------------------------

#[test]
fn rejects_truncated_json() -> Result<(), Box<dyn std::error::Error>> {
    let truncated = br#"{"mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json", "verificationMaterial": {"#;
    match Bundle::from_json(truncated) {
        Err(Error::Parse(ParseError::Json(_))) => Ok(()),
        other => Err(format!("expected ParseError::Json, got {other:?}").into()),
    }
}

#[test]
fn rejects_wrong_bundle_media_type() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    value["mediaType"] = serde_json::Value::String("application/x-bogus+json".to_owned());
    let mutated = serde_json::to_vec(&value)?;

    match Bundle::from_json(&mutated) {
        Err(Error::Unsupported(UnsupportedError::MediaType { found })) => {
            if found == "application/x-bogus+json" {
                Ok(())
            } else {
                Err(format!("unexpected `found` value: {found}").into())
            }
        }
        other => Err(format!("expected MediaType error, got {other:?}").into()),
    }
}

#[test]
fn rejects_dsse_with_zero_signatures() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    value["dsseEnvelope"]["signatures"] = serde_json::Value::Array(vec![]);
    let mutated = serde_json::to_vec(&value)?;

    match Bundle::from_json(&mutated) {
        Err(Error::Parse(ParseError::DsseSignatureCount { count: 0 })) => Ok(()),
        other => Err(format!("expected DsseSignatureCount {{ count: 0 }}, got {other:?}").into()),
    }
}

#[test]
fn rejects_dsse_with_two_signatures() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let first_signature = value["dsseEnvelope"]["signatures"][0].clone();
    value["dsseEnvelope"]["signatures"] =
        serde_json::Value::Array(vec![first_signature.clone(), first_signature]);
    let mutated = serde_json::to_vec(&value)?;

    match Bundle::from_json(&mutated) {
        Err(Error::Parse(ParseError::DsseSignatureCount { count: 2 })) => Ok(()),
        other => Err(format!("expected DsseSignatureCount {{ count: 2 }}, got {other:?}").into()),
    }
}

#[test]
fn rejects_malformed_subject_digest_63_chars() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut bundle_value: serde_json::Value = serde_json::from_slice(&bytes)?;

    let payload_b64 = bundle_value["dsseEnvelope"]["payload"]
        .as_str()
        .ok_or("dsseEnvelope.payload missing or not a string")?
        .to_owned();
    let payload_bytes = STANDARD.decode(payload_b64)?;
    let mut statement_value: serde_json::Value = serde_json::from_slice(&payload_bytes)?;
    statement_value["subject"][0]["digest"]["sha256"] = serde_json::Value::String("a".repeat(63));
    let mutated_payload = serde_json::to_vec(&statement_value)?;
    bundle_value["dsseEnvelope"]["payload"] =
        serde_json::Value::String(STANDARD.encode(mutated_payload));

    let mutated_bytes = serde_json::to_vec(&bundle_value)?;
    let bundle = Bundle::from_json(&mutated_bytes)?;

    match bundle.statement() {
        Err(Error::Parse(ParseError::MalformedSubject(_))) => Ok(()),
        other => Err(format!("expected MalformedSubject error, got {other:?}").into()),
    }
}

#[test]
fn rejects_non_numeric_log_index() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    value["verificationMaterial"]["tlogEntries"][0]["logIndex"] =
        serde_json::Value::String("not-a-number".to_owned());
    let mutated = serde_json::to_vec(&value)?;

    match Bundle::from_json(&mutated) {
        Err(Error::Parse(ParseError::NotAnInteger { .. })) => Ok(()),
        other => Err(format!("expected NotAnInteger error, got {other:?}").into()),
    }
}

#[test]
fn rejects_oversized_bundle_input() -> Result<(), Box<dyn std::error::Error>> {
    let oversized = vec![b'a'; 8 * 1024 * 1024 + 1];
    match Bundle::from_json(&oversized) {
        Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge { .. })) => Ok(()),
        other => Err(format!("expected InputTooLarge error, got {other:?}").into()),
    }
}

#[test]
fn rejects_oversized_bundle_set_jsonl_input() -> Result<(), Box<dyn std::error::Error>> {
    let oversized = vec![b'a'; 8 * 1024 * 1024 + 1];
    match BundleSet::from_json_lines(&oversized) {
        Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge { .. })) => Ok(()),
        other => Err(format!("expected InputTooLarge error, got {other:?}").into()),
    }
}

#[test]
fn rejects_oversized_github_response_input() -> Result<(), Box<dyn std::error::Error>> {
    let oversized = vec![b'a'; 8 * 1024 * 1024 + 1];
    match BundleSet::from_github_response(&oversized) {
        Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge { .. })) => Ok(()),
        other => Err(format!("expected InputTooLarge error, got {other:?}").into()),
    }
}

#[test]
fn rejects_bundle_with_oversized_ignored_json_array() -> Result<(), Box<dyn std::error::Error>> {
    // Unknown fields are tolerated by design, so an ignored array is enough
    // to blow up the parsed `Value` tree on an otherwise valid bundle while
    // staying far under the input-byte limit.
    let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let Some(object) = value.as_object_mut() else {
        return Err("expected the fixture to be a JSON object".into());
    };
    object.insert(
        "junk".to_owned(),
        serde_json::Value::Array(vec![serde_json::Value::from(0); 300_000]),
    );
    let mutated = serde_json::to_vec(&value)?;
    if mutated.len() >= 8 * 1024 * 1024 {
        return Err("mutated bundle should stay under the input-byte limit".into());
    }

    match Bundle::from_json(&mutated) {
        Err(Error::ResourceLimit(ResourceLimitError::TooManyJsonNodes { .. })) => Ok(()),
        other => Err(format!("expected TooManyJsonNodes error, got {other:?}").into()),
    }
}

#[test]
fn subject_from_digest_hex_rejects_63_65_and_non_hex() -> Result<(), Box<dyn std::error::Error>> {
    for candidate in ["a".repeat(63), "a".repeat(65), "z".repeat(64)] {
        match Subject::from_digest_hex(&candidate) {
            Err(Error::Parse(ParseError::MalformedDigest(_))) => {}
            other => {
                return Err(format!(
                    "expected MalformedDigest error for {candidate:?}, got {other:?}"
                )
                .into());
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Fixture facts that DESIGN.md's normative sections argue *from*.
//
// Both sections explain why a check is deliberately absent, and both
// arguments rest on what the real `cli/cli` bundle happens to contain. A
// fixture refresh that changed either would leave the document asserting
// something false, and the natural next step -- "implement the missing
// check" -- would then start rejecting genuine bundles. These tests fail
// at that moment instead.
// ---------------------------------------------------------------------

/// Reads the golden fixture as raw JSON.
///
/// Deliberately not through [`Bundle`]: the claims below are about what
/// the file contains, so reading them through the crate's own model would
/// let a modelling change mask a fixture change.
fn golden_fixture_json() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(&read_fixture(
        "github-cli/tarball-user-slsa-provenance.json",
    )?)?)
}

fn json_u64(value: &serde_json::Value, path: &str) -> Result<u64, Box<dyn std::error::Error>> {
    // protojson encodes int64/uint64 as JSON strings.
    let raw = value
        .as_str()
        .ok_or_else(|| format!("{path} is not a JSON string"))?;
    Ok(raw.parse::<u64>()?)
}

#[test]
fn golden_log_index_differs_from_its_inclusion_proof_index()
-> Result<(), Box<dyn std::error::Error>> {
    let golden = golden_fixture_json()?;
    let entry = &golden["verificationMaterial"]["tlogEntries"][0];
    let proof = &entry["inclusionProof"];

    let entry_index = json_u64(&entry["logIndex"], "logIndex")?;
    let proof_index = json_u64(&proof["logIndex"], "inclusionProof.logIndex")?;
    let tree_size = json_u64(&proof["treeSize"], "inclusionProof.treeSize")?;

    if entry_index == proof_index {
        return Err(format!(
            "DESIGN.md \"Rekor-entry <-> bundle binding\" states these are distinct quantities and \
             that comparing them would reject genuine bundles; this fixture now has both at \
             {entry_index}, so that passage no longer describes real data"
        )
        .into());
    }
    // The proof's index is a position within the tree the proof was issued
    // against, so it must be inside that tree. The entry's own index is a
    // position in the log as a whole and carries no such relationship --
    // here it is larger than the tree it was proved against.
    if proof_index >= tree_size {
        return Err(format!(
            "inclusion-proof index {proof_index} is not inside its own tree of size {tree_size}"
        )
        .into());
    }
    Ok(())
}

/// Half the order of the P-256 base point, big-endian. A signature is
/// "high-S" when its `s` exceeds this.
const P256_HALF_ORDER: [u8; 32] = [
    0x7f, 0xff, 0xff, 0xff, 0x80, 0x00, 0x00, 0x00, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xde, 0x73, 0x7d, 0x56, 0xd3, 0x8b, 0xcf, 0x42, 0x79, 0xdc, 0xe5, 0x61, 0x7e, 0x31, 0x92, 0xa8,
];

/// Returns `s` from a DER `SEQUENCE { INTEGER r, INTEGER s }`, left-padded
/// to 32 bytes.
fn ecdsa_der_s(signature: &[u8]) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    let mut offset = 0;
    let mut take = |expected_tag: u8| -> Result<&[u8], Box<dyn std::error::Error>> {
        let tag = *signature.get(offset).ok_or("truncated DER tag")?;
        if tag != expected_tag {
            return Err(format!("expected DER tag {expected_tag:#04x}, found {tag:#04x}").into());
        }
        let length = *signature.get(offset + 1).ok_or("truncated DER length")? as usize;
        if length & 0x80 != 0 {
            return Err("unexpected long-form DER length in an ECDSA signature".into());
        }
        let start = offset + 2;
        let end = start + length;
        let body = signature.get(start..end).ok_or("truncated DER value")?;
        offset = end;
        Ok(body)
    };

    let sequence = take(0x30)?.to_vec();
    let mut inner = 0;
    let mut integer = |data: &[u8]| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let tag = *data.get(inner).ok_or("truncated DER integer tag")?;
        if tag != 0x02 {
            return Err(format!("expected DER INTEGER, found {tag:#04x}").into());
        }
        let length = *data.get(inner + 1).ok_or("truncated DER integer length")? as usize;
        let start = inner + 2;
        let end = start + length;
        let body = data
            .get(start..end)
            .ok_or("truncated DER integer")?
            .to_vec();
        inner = end;
        Ok(body)
    };
    let _r = integer(&sequence)?;
    let s = integer(&sequence)?;

    // DER integers are signed, so a leading zero byte may be present.
    let trimmed = s.strip_prefix(&[0]).unwrap_or(&s);
    if trimmed.len() > 32 {
        return Err(format!("s is {} bytes, too large for P-256", trimmed.len()).into());
    }
    let mut out = [0u8; 32];
    out[32 - trimmed.len()..].copy_from_slice(trimmed);
    Ok(out)
}

#[test]
fn every_golden_signature_is_high_s() -> Result<(), Box<dyn std::error::Error>> {
    let golden = golden_fixture_json()?;
    let entry = &golden["verificationMaterial"]["tlogEntries"][0];

    let checkpoint_envelope = entry["inclusionProof"]["checkpoint"]["envelope"]
        .as_str()
        .ok_or("checkpoint envelope is not a string")?;
    let checkpoint_line = checkpoint_envelope
        .lines()
        .rfind(|line| line.starts_with('\u{2014}'))
        .ok_or("checkpoint has no signature line")?;
    let checkpoint_blob = STANDARD.decode(
        checkpoint_line
            .rsplit_once(' ')
            .ok_or("malformed checkpoint signature line")?
            .1,
    )?;

    let signatures = [
        // The 4-byte signed-note key hint precedes the DER signature.
        ("checkpoint", checkpoint_blob[4..].to_vec()),
        (
            "SET",
            STANDARD.decode(
                entry["inclusionPromise"]["signedEntryTimestamp"]
                    .as_str()
                    .ok_or("SET is not a string")?,
            )?,
        ),
        (
            "DSSE",
            STANDARD.decode(
                golden["dsseEnvelope"]["signatures"][0]["sig"]
                    .as_str()
                    .ok_or("DSSE signature is not a string")?,
            )?,
        ),
    ];

    for (label, signature) in signatures {
        if ecdsa_der_s(&signature)? <= P256_HALF_ORDER {
            return Err(format!(
                "the {label} signature is low-S. DESIGN.md \"Testing strategy\" states that every \
                 signature in this fixture is high-S, and concludes from that low-S must never be \
                 enforced; that conclusion no longer follows from this fixture"
            )
            .into());
        }
    }
    Ok(())
}
