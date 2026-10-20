//! Fulcio identity-extension claims extraction.
//!
//! Extracts the signer-workflow identity Fulcio embeds in every leaf
//! certificate it issues (OIDs under 1.3.6.1.4.1.57264.1) plus the SAN
//! URI, from an already-[`crate::x509::validate_chain`]-validated leaf.
//! Claims are certificate-derived facts (authenticated by the X.509
//! chain), distinct from the workflow-controlled in-toto statement
//! content `DESIGN.md`'s "provenance-separated output" keeps apart.
//!
//! Only OIDs `.1` through `.6` (the deprecated, pre-2022 raw-string
//! encoding, still present alongside the current ones on real leaves) are
//! skipped; `.8` through `.21` -- the current DER `UTF8String`-wrapped
//! generation -- are modeled, matching what a real
//! `actions/attest-build-provenance` leaf actually carries (confirmed
//! against the golden fixture). OIDs `.22`-`.24` also appear on the real
//! leaf (repository visibility and a GitHub-specific environment-binding
//! extension) but are outside `DESIGN.md`'s modeled set and tolerated as
//! unknown, non-critical extensions.
//!
//! Run as step 9 of [`crate::Verifier::verify_digest`]'s chain; the
//! resulting [`FulcioClaims`] are matched against a caller's policy by
//! [`crate::policy_match`] (step 10).

use der::Decode;
use der::asn1::{ObjectIdentifier, Utf8StringRef};
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::SubjectAltName;
use x509_cert::ext::pkix::name::GeneralName;

use crate::error::{CertificateError, Error};
use crate::x509::{ValidatedLeaf, find_extension};

/// `subjectAltName` (RFC 5280 SS4.2.1.6) -- duplicated locally rather
/// than imported from [`crate::x509`] (which does not expose it) since
/// it is a single well-known constant.
const ID_CE_SUBJECT_ALT_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.17");

/// Builds a `const ObjectIdentifier` for `1.3.6.1.4.1.57264.1.$n`, the
/// Fulcio identity-extension OID arc.
macro_rules! fulcio_oid {
    ($n:literal) => {
        ObjectIdentifier::new_unwrap(concat!("1.3.6.1.4.1.57264.1.", $n))
    };
}

const OID_ISSUER: ObjectIdentifier = fulcio_oid!(8);
const OID_BUILD_SIGNER_URI: ObjectIdentifier = fulcio_oid!(9);
const OID_BUILD_SIGNER_DIGEST: ObjectIdentifier = fulcio_oid!(10);
const OID_RUNNER_ENVIRONMENT: ObjectIdentifier = fulcio_oid!(11);
const OID_SOURCE_REPOSITORY_URI: ObjectIdentifier = fulcio_oid!(12);
const OID_SOURCE_REPOSITORY_DIGEST: ObjectIdentifier = fulcio_oid!(13);
const OID_SOURCE_REPOSITORY_REF: ObjectIdentifier = fulcio_oid!(14);
const OID_SOURCE_REPOSITORY_ID: ObjectIdentifier = fulcio_oid!(15);
const OID_SOURCE_REPOSITORY_OWNER_URI: ObjectIdentifier = fulcio_oid!(16);
const OID_SOURCE_REPOSITORY_OWNER_ID: ObjectIdentifier = fulcio_oid!(17);
const OID_BUILD_CONFIG_URI: ObjectIdentifier = fulcio_oid!(18);
const OID_BUILD_CONFIG_DIGEST: ObjectIdentifier = fulcio_oid!(19);
const OID_BUILD_TRIGGER: ObjectIdentifier = fulcio_oid!(20);
const OID_RUN_INVOCATION_URI: ObjectIdentifier = fulcio_oid!(21);

/// Fulcio identity claims extracted from a validated leaf certificate.
///
/// Every field is `None` when its extension is absent -- tolerated, not
/// an error, since this extraction step is deliberately general-purpose;
/// it is a caller's identity *policy* (not modeled by this task) that
/// decides which fields it actually requires.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FulcioClaims {
    /// The signer-workflow URI from the leaf's `subjectAltName`
    /// (`GeneralName::UniformResourceIdentifier`), e.g.
    /// `"https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk"`.
    pub(crate) san_uri: Option<String>,
    /// OID .8: the OIDC issuer, e.g.
    /// `"https://token.actions.githubusercontent.com"`.
    pub(crate) issuer: Option<String>,
    /// OID .9: build signer URI.
    pub(crate) build_signer_uri: Option<String>,
    /// OID .10: build signer digest.
    pub(crate) build_signer_digest: Option<String>,
    /// OID .11: runner environment (e.g. `"github-hosted"`).
    pub(crate) runner_environment: Option<String>,
    /// OID .12: source repository URI, e.g.
    /// `"https://github.com/cli/cli"`.
    pub(crate) source_repository_uri: Option<String>,
    /// OID .13: source repository digest.
    pub(crate) source_repository_digest: Option<String>,
    /// OID .14: source repository ref, e.g. `"refs/heads/trunk"`.
    pub(crate) source_repository_ref: Option<String>,
    /// OID .15: source repository numeric id, as a decimal string.
    pub(crate) source_repository_id: Option<String>,
    /// OID .16: source repository owner URI, e.g.
    /// `"https://github.com/cli"`.
    pub(crate) source_repository_owner_uri: Option<String>,
    /// OID .17: source repository owner numeric id, as a decimal string.
    pub(crate) source_repository_owner_id: Option<String>,
    /// OID .18: build config URI.
    pub(crate) build_config_uri: Option<String>,
    /// OID .19: build config digest.
    pub(crate) build_config_digest: Option<String>,
    /// OID .20: build trigger (e.g. `"workflow_dispatch"`, `"push"`).
    pub(crate) build_trigger: Option<String>,
    /// OID .21: run invocation URI.
    pub(crate) run_invocation_uri: Option<String>,
}

/// Extracts [`FulcioClaims`] from `validated`'s leaf certificate.
///
/// # Errors
///
/// Returns [`CertificateError::DuplicateExtension`] if any modeled
/// extension (the SAN, or any of OIDs `.8`-`.21`) appears more than once,
/// and [`CertificateError::InvalidDer`] if a present extension's value is
/// not the DER type this crate expects for it (`GeneralNames` for the
/// SAN, a DER `UTF8String` for every Fulcio OID).
pub(crate) fn extract_claims(validated: &ValidatedLeaf) -> Result<FulcioClaims, Error> {
    let tbs = validated.leaf.tbs_certificate();
    let extensions = tbs.extensions().map_or(&[][..], |exts| exts.as_slice());

    Ok(FulcioClaims {
        san_uri: extract_san_uri(extensions)?,
        issuer: extract_utf8_claim(extensions, OID_ISSUER)?,
        build_signer_uri: extract_utf8_claim(extensions, OID_BUILD_SIGNER_URI)?,
        build_signer_digest: extract_utf8_claim(extensions, OID_BUILD_SIGNER_DIGEST)?,
        runner_environment: extract_utf8_claim(extensions, OID_RUNNER_ENVIRONMENT)?,
        source_repository_uri: extract_utf8_claim(extensions, OID_SOURCE_REPOSITORY_URI)?,
        source_repository_digest: extract_utf8_claim(extensions, OID_SOURCE_REPOSITORY_DIGEST)?,
        source_repository_ref: extract_utf8_claim(extensions, OID_SOURCE_REPOSITORY_REF)?,
        source_repository_id: extract_utf8_claim(extensions, OID_SOURCE_REPOSITORY_ID)?,
        source_repository_owner_uri: extract_utf8_claim(
            extensions,
            OID_SOURCE_REPOSITORY_OWNER_URI,
        )?,
        source_repository_owner_id: extract_utf8_claim(extensions, OID_SOURCE_REPOSITORY_OWNER_ID)?,
        build_config_uri: extract_utf8_claim(extensions, OID_BUILD_CONFIG_URI)?,
        build_config_digest: extract_utf8_claim(extensions, OID_BUILD_CONFIG_DIGEST)?,
        build_trigger: extract_utf8_claim(extensions, OID_BUILD_TRIGGER)?,
        run_invocation_uri: extract_utf8_claim(extensions, OID_RUN_INVOCATION_URI)?,
    })
}

/// Extracts and DER-decodes the extension at `oid` as a `UTF8String`, per
/// the current (OID `.8` and up) Fulcio identity-extension encoding: the
/// extension's `extnValue` OCTET STRING directly contains a DER
/// `UTF8String` TLV (unlike the SCT list, this is a single level of
/// wrapping -- see [`crate::sct`]'s module docs for the extension that
/// is double-wrapped).
///
/// # Errors
///
/// Returns [`CertificateError::DuplicateExtension`] if `oid` appears more
/// than once, and [`CertificateError::InvalidDer`] if it appears once but
/// its value is not a well-formed DER `UTF8String`.
fn extract_utf8_claim(
    extensions: &[Extension],
    oid: ObjectIdentifier,
) -> Result<Option<String>, Error> {
    let Some(ext) = find_extension(extensions, oid)? else {
        return Ok(None);
    };
    let s = Utf8StringRef::from_der(ext.extn_value.as_bytes())
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    Ok(Some(s.as_str().to_owned()))
}

/// Extracts the URI from the leaf's `subjectAltName`, if it has a
/// `uniformResourceIdentifier` `GeneralName` entry.
///
/// A Fulcio leaf's SAN carries exactly this one name in every fixture
/// this crate has seen; if more than one URI entry were ever present,
/// the *first* is returned rather than erroring -- `DuplicateExtension`
/// is reserved for the SAN extension itself appearing twice (a malformed
/// certificate), not for a `GeneralNames` sequence with multiple entries
/// (valid X.509 shape this crate simply does not expect Fulcio to use).
///
/// # Errors
///
/// Returns [`CertificateError::DuplicateExtension`] if the SAN extension
/// itself appears more than once, and [`CertificateError::InvalidDer`] if
/// it appears once but does not decode as `GeneralNames`.
fn extract_san_uri(extensions: &[Extension]) -> Result<Option<String>, Error> {
    let Some(ext) = find_extension(extensions, ID_CE_SUBJECT_ALT_NAME)? else {
        return Ok(None);
    };
    let SubjectAltName(names) = SubjectAltName::from_der(ext.extn_value.as_bytes())
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    Ok(names.into_iter().find_map(|name| match name {
        GeneralName::UniformResourceIdentifier(uri) => Some(uri.as_str().to_owned()),
        _ => None,
    }))
}

#[cfg(test)]
mod tests {
    use der::asn1::AnyRef;
    use der::{Decode, Encode};

    use super::{FulcioClaims, extract_claims};
    use crate::bundle::Bundle;
    use crate::error::{CertificateError, Error};
    use crate::sct::verify_embedded_scts;
    use crate::trust::TrustStore;
    use crate::x509::{ValidatedLeaf, find_extension, validate_chain};

    const REAL_INTEGRATED_TIME: i64 = 1_783_027_755;

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative)
    }

    fn read_fixture(relative: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(fixture_path(relative))?)
    }

    fn real_leaf_der() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bundle = Bundle::from_json(&read_fixture(
            "github-cli/tarball-user-slsa-provenance.json",
        )?)?;
        Ok(bundle.verification_material.certificate.raw_bytes)
    }

    fn embedded_trust_store() -> Result<TrustStore, Box<dyn std::error::Error>> {
        Ok(TrustStore::embedded_public_good()?)
    }

    // -------------------------------------------------------------
    // Positive: composed helper -- leaf DER -> ValidatedLeaf -> SCT ok ->
    // FulcioClaims, all on the real golden fixture.
    // -------------------------------------------------------------

    /// leaf DER -> [`validate_chain`] -> [`verify_embedded_scts`] ->
    /// [`extract_claims`], exactly the composition order `DESIGN.md`'s
    /// X.509/Fulcio profile implies: claims are only ever read from a
    /// leaf whose chain *and* CT-log evidence both already checked out.
    fn verify_and_extract_claims(
        leaf_der: &[u8],
        trust_store: &TrustStore,
        authenticated_time: i64,
    ) -> Result<FulcioClaims, Error> {
        let validated = validate_chain(leaf_der, trust_store, authenticated_time)?;
        verify_embedded_scts(&validated, trust_store)?;
        extract_claims(&validated)
    }

    #[test]
    fn composed_chain_to_sct_to_claims_succeeds_on_real_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = embedded_trust_store()?;
        let claims = verify_and_extract_claims(&leaf_der, &trust_store, REAL_INTEGRATED_TIME)?;

        // Known-truth assertions (DESIGN.md task): matches
        // `attestations-api-response.redacted.json`'s `repository_id`
        // and the pinned GitHub Actions OIDC issuer.
        assert_claim(
            claims.source_repository_uri.as_deref(),
            "https://github.com/cli/cli",
        )?;
        assert_claim(claims.source_repository_id.as_deref(), "212613049")?;
        assert_claim(
            claims.issuer.as_deref(),
            "https://token.actions.githubusercontent.com",
        )?;
        assert_claim(
            claims.source_repository_owner_uri.as_deref(),
            "https://github.com/cli",
        )?;
        assert_claim(claims.source_repository_owner_id.as_deref(), "59704711")?;

        // Empirically discovered (printed and cross-checked against the
        // real leaf's DER before writing this test) rather than assumed:
        // this fixture's workflow ran via `workflow_dispatch` on
        // `refs/heads/trunk`, *not* a `v2.96.0` tag push -- see this
        // crate's task report for the full spec-vs-reality note. Asserted
        // here as ground truth precisely so a future fixture refresh that
        // silently changes this is caught.
        assert_claim(claims.source_repository_ref.as_deref(), "refs/heads/trunk")?;
        assert_claim(claims.build_trigger.as_deref(), "workflow_dispatch")?;
        assert_claim(claims.runner_environment.as_deref(), "github-hosted")?;
        assert_claim(
            claims.san_uri.as_deref(),
            "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk",
        )?;
        assert_claim(
            claims.build_signer_uri.as_deref(),
            "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk",
        )?;
        assert_claim(
            claims.build_config_uri.as_deref(),
            "https://github.com/cli/cli/.github/workflows/deployment.yml@refs/heads/trunk",
        )?;
        assert_claim(
            claims.run_invocation_uri.as_deref(),
            "https://github.com/cli/cli/actions/runs/28622199740/attempts/1",
        )?;
        assert_claim(
            claims.source_repository_digest.as_deref(),
            "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca0",
        )?;
        assert_claim(
            claims.build_signer_digest.as_deref(),
            "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca0",
        )?;
        assert_claim(
            claims.build_config_digest.as_deref(),
            "b300f2ec7ec9dc9addc39b2ad88c54097ded7ca0",
        )?;

        Ok(())
    }

    fn assert_claim(
        actual: Option<&str>,
        expected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match actual {
            Some(s) if s == expected => Ok(()),
            other => Err(format!("expected {expected:?}, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: duplicate extension
    // -------------------------------------------------------------

    #[test]
    fn find_extension_rejects_synthetic_duplicate() -> Result<(), Box<dyn std::error::Error>> {
        use x509_cert::ext::Extension;

        let oid = der::asn1::ObjectIdentifier::new_unwrap("1.2.3.4");
        let value = der::asn1::OctetString::new(vec![0x0c, 0x01, b'x'])
            .map_err(|e| format!("failed to build OctetString: {e}"))?;
        let one = Extension {
            extn_id: oid,
            critical: false,
            extn_value: value,
        };
        let two = one.clone();
        match find_extension(&[one, two], oid) {
            Err(Error::Certificate(CertificateError::DuplicateExtension)) => Ok(()),
            other => Err(format!("expected DuplicateExtension, got {other:?}").into()),
        }
    }

    /// Duplicates the extension at `oid` within `leaf_der`'s
    /// `TBSCertificate`, leaving everything else (including the now-stale
    /// outer signature) untouched. The result is not a certificate any
    /// chain validation would accept, but `extract_claims` never checks
    /// signatures -- it only reads `validated.leaf`, which this test
    /// constructs directly rather than by going through
    /// [`validate_chain`] (mirroring `sct.rs`'s
    /// `leaf_without_sct_extension_is_sct_missing` test).
    fn duplicate_extension_in_leaf_der(
        leaf_der: &[u8],
        oid: der::asn1::ObjectIdentifier,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use der::{Tag, TagNumber, Tagged};
        use x509_cert::ext::Extensions;

        let extensions_context_tag = Tag::ContextSpecific {
            constructed: true,
            number: TagNumber(3),
        };

        let outer = Vec::<AnyRef<'_>>::from_der(leaf_der)?;
        let [cert_tbs, sig_alg, signature] = <[AnyRef<'_>; 3]>::try_from(outer)
            .map_err(|_| "expected exactly 3 top-level Certificate fields")?;

        // `cert_tbs.value()` is already stripped of the TBSCertificate's
        // own outer SEQUENCE tag+length; re-encode `cert_tbs` back to a
        // complete TLV first so `Vec::<AnyRef>::from_der` (which expects
        // a whole SEQUENCE, not pre-stripped content) has the wrapper it
        // needs to strip itself.
        let cert_tbs_der = cert_tbs.to_der()?;
        let mut tbs_fields = Vec::<AnyRef<'_>>::from_der(&cert_tbs_der)?;
        let ext_field = tbs_fields.pop().ok_or("TBSCertificate has no fields")?;
        if ext_field.tag() != extensions_context_tag {
            return Err("TBSCertificate's last field is not [3] EXPLICIT extensions".into());
        }
        let mut extensions = Extensions::from_der(ext_field.value())?;
        let duplicate = extensions
            .iter()
            .find(|e| e.extn_id == oid)
            .cloned()
            .ok_or("extension to duplicate was not present")?;
        extensions.push(duplicate);

        let extensions_der = extensions.to_der()?;
        let new_ext_field = AnyRef::new(extensions_context_tag, &extensions_der)?;
        tbs_fields.push(new_ext_field);
        let new_tbs_der = tbs_fields.to_der()?;
        let new_tbs = AnyRef::from_der(&new_tbs_der)?;

        Ok(vec![new_tbs, sig_alg, signature].to_der()?)
    }

    #[test]
    fn duplicate_fulcio_extension_on_real_leaf_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let source_repository_uri_oid =
            der::asn1::ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.12");
        let tampered_der = duplicate_extension_in_leaf_der(&leaf_der, source_repository_uri_oid)?;

        let leaf = x509_cert::Certificate::from_der(&tampered_der)
            .map_err(|e| format!("failed to parse tampered leaf: {e}"))?;
        let validated = ValidatedLeaf {
            leaf,
            leaf_spki_der: Vec::new(),
            issuer_spki_der: Vec::new(),
            chain_length: 1,
        };

        match extract_claims(&validated) {
            Err(Error::Certificate(CertificateError::DuplicateExtension)) => Ok(()),
            other => Err(format!("expected DuplicateExtension, got {other:?}").into()),
        }
    }
}
