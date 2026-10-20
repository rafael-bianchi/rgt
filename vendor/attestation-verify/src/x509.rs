//! Fulcio X.509 certificate-chain validation.
//!
//! Implements `DESIGN.md`'s declared "X.509 / Fulcio validation profile":
//! local path validation against trust-store-supplied certificate
//! authorities only (bundle-supplied roots are never trusted, and this
//! crate does not even parse any), the allowed signature-algorithm/curve
//! pairing, name chaining, time windows, and the Fulcio leaf/CA
//! certificate profile. RFC 5280 SS4.2's "reject an unrecognized critical
//! extension" rule applies to *every* certificate validated here -- the
//! leaf and each trust-store chain certificate alike -- against a
//! per-role known-extension set. [`crate::sct`] (SCT verification) and
//! [`crate::fulcio`] (claims extraction) both consume this module's
//! [`ValidatedLeaf`]. Run as step 6 of
//! [`crate::Verifier::verify_digest`]'s chain, after the transparency-log
//! entry ([`crate::rekor`]) has already authenticated a signing time.

use der::asn1::ObjectIdentifier;
use der::{Decode, Encode};
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::{BasicConstraints, ExtendedKeyUsage, KeyUsage};
use x509_cert::{Certificate, TbsCertificate};

use crate::dsse::{EcdsaVerifyingKey, unix_seconds};
use crate::error::{CertificateError, Error};
use crate::trust::{CertificateAuthority, TrustStore, ValidityPeriod};

/// ECDSA-with-SHA-256 (1.2.840.10045.4.3.2): the only signature algorithm
/// this profile allows when the issuer's key is P-256.
const ECDSA_WITH_SHA_256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2");
/// ECDSA-with-SHA-384 (1.2.840.10045.4.3.3): the only signature algorithm
/// this profile allows when the issuer's key is P-384 (current Fulcio CA
/// signatures -- confirmed empirically against the real trust root).
const ECDSA_WITH_SHA_384: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.3");
/// `id-kp-codeSigning` (1.3.6.1.5.5.7.3.3), required in the leaf's
/// `extKeyUsage`.
const ID_KP_CODE_SIGNING: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.3");
/// The SCT-list extension OID (RFC 6962). Verifying its *content* is
/// [`crate::sct`]'s job; this module only needs the OID itself, to allow
/// it in the leaf's known-critical-extension set below.
pub(crate) const SCT_LIST_OID: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.11129.2.4.2");

const ID_CE_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.15");
const ID_CE_SUBJECT_ALT_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.17");
const ID_CE_BASIC_CONSTRAINTS: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.19");
const ID_CE_EXT_KEY_USAGE: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.37");

/// Extensions the leaf certificate may carry -- critical or not -- without
/// being rejected for an unrecognized critical extension.
///
/// Empirically, only `keyUsage` and `subjectAltName` are actually marked
/// critical on a real Fulcio leaf (confirmed against the golden fixture);
/// `extKeyUsage` and the SCT list are present but non-critical, and
/// `basicConstraints` is absent entirely. All five are listed here
/// regardless, per `DESIGN.md`: membership in the *known* set is what
/// matters, not each one's actual criticality on any one certificate.
const KNOWN_LEAF_EXTENSION_OIDS: [ObjectIdentifier; 5] = [
    ID_CE_KEY_USAGE,
    ID_CE_EXT_KEY_USAGE,
    ID_CE_BASIC_CONSTRAINTS,
    ID_CE_SUBJECT_ALT_NAME,
    SCT_LIST_OID,
];

/// Extensions a chain certificate authority may carry as *critical*
/// without being rejected for an unrecognized critical extension.
///
/// Deliberately narrower than [`KNOWN_LEAF_EXTENSION_OIDS`]: only the two
/// extensions [`verify_ca_profile`] actually enforces. Empirically, every
/// `certificateAuthorities[].certChain` certificate in both the embedded
/// public-good root and GitHub's trust root marks exactly `keyUsage` and
/// `basicConstraints` critical; `extKeyUsage`, `subjectKeyIdentifier` and
/// `authorityKeyIdentifier` are present on some of them but always
/// non-critical, so admitting them here would buy nothing and would mask
/// a constraint this crate does not enforce. `nameConstraints`
/// (2.5.29.30) and `policyConstraints` are likewise absent: unimplemented
/// means a caller-supplied intermediate carrying one critically must be
/// rejected, not silently unconstrained.
const KNOWN_CA_CRITICAL_EXTENSION_OIDS: [ObjectIdentifier; 2] =
    [ID_CE_KEY_USAGE, ID_CE_BASIC_CONSTRAINTS];

/// A leaf certificate that has been fully validated against a trust
/// store's certificate authorities, per `DESIGN.md`'s "X.509 / Fulcio
/// validation profile".
///
/// Carries everything [`crate::sct`] and [`crate::fulcio`] need so
/// neither has to re-walk the chain.
#[derive(Debug)]
pub(crate) struct ValidatedLeaf {
    /// The parsed leaf certificate.
    pub(crate) leaf: Certificate,
    /// The leaf's own `SubjectPublicKeyInfo`, DER-encoded (the DSSE
    /// signing key -- see [`crate::dsse`]).
    pub(crate) leaf_spki_der: Vec<u8>,
    /// The DER-encoded `SubjectPublicKeyInfo` of the certificate that
    /// directly issued the leaf (an intermediate, or the root itself for
    /// a single-certificate chain). Needed to compute an SCT's
    /// `issuer_key_hash` ([`crate::sct`]).
    pub(crate) issuer_spki_der: Vec<u8>,
    /// Number of trust-store certificates walked from the leaf's direct
    /// issuer up to and including the root. Not currently read by
    /// [`crate::verifier`] (DESIGN.md's report shape does not call for
    /// it) or by anything else outside this module's own tests, but kept
    /// for diagnostics since it is already computed for free while
    /// validating.
    #[allow(dead_code)]
    pub(crate) chain_length: usize,
}

/// Validates `leaf_der` against `trust_store`'s certificate authorities at
/// `authenticated_time_unix`, per `DESIGN.md`'s "X.509 / Fulcio validation
/// profile".
///
/// Only trust-store-supplied certificate authorities are ever trusted.
/// Each candidate certificate authority's `certChain.certificates[]` is
/// ordered nearest-to-leaf first, self-signed root last (confirmed
/// empirically against the real public-good and GitHub trust roots: a
/// two-certificate entry is `[intermediate, root]`; GitHub's is
/// three-deep, `[intermediate-l2, intermediate-l1, root]`); this
/// function relies on that order rather than guessing it.
///
/// Tries every certificate authority whose chain's nearest-to-leaf
/// certificate has `subject` matching the leaf's `issuer`, in trust-store
/// order; the first fully-successful candidate wins. If every
/// name-matching candidate fails, the error from the *first* of them (in
/// trust-store order) is returned, rather than being masked as a generic
/// "untrusted": a candidate that named itself this leaf's issuer produced
/// a real, attributable reason, and reporting one such reason is more
/// useful than reporting none. [`CertificateError::UntrustedCertificate`]
/// is reserved for the case where no candidate's name matched at all.
///
/// # Errors
///
/// Returns [`CertificateError::InvalidDer`] if `leaf_der` (or a
/// trust-store certificate) does not parse,
/// [`CertificateError::UntrustedCertificate`] if no certificate
/// authority's chain issues this leaf at all, and otherwise whatever
/// specific [`CertificateError`] the first name-matching candidate failed
/// on (signature, name, time, or profile).
pub(crate) fn validate_chain(
    leaf_der: &[u8],
    trust_store: &TrustStore,
    authenticated_time_unix: i64,
) -> Result<ValidatedLeaf, Error> {
    let leaf = Certificate::from_der(leaf_der)
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;

    let mut name_matched_error: Option<Error> = None;
    for ca in &trust_store.certificate_authorities {
        match try_validate_against_ca(&leaf, ca, authenticated_time_unix) {
            Ok(Some(validated)) => return Ok(validated),
            Ok(None) => {}
            Err(e) => {
                if name_matched_error.is_none() {
                    name_matched_error = Some(e);
                }
            }
        }
    }
    Err(name_matched_error.unwrap_or(Error::Certificate(CertificateError::UntrustedCertificate)))
}

/// Attempts to validate `leaf` against one candidate certificate
/// authority entry.
///
/// Returns `Ok(None)` if this candidate's chain does not even claim to
/// issue `leaf` (its nearest-to-leaf certificate's `subject` does not
/// match `leaf`'s `issuer`) -- the caller should try the next candidate,
/// not treat this as a failure. Once past that name-match gate, any
/// further problem is a real, attributable failure for this candidate.
fn try_validate_against_ca(
    leaf: &Certificate,
    ca: &CertificateAuthority,
    authenticated_time: i64,
) -> Result<Option<ValidatedLeaf>, Error> {
    let chain = ca
        .certificates
        .iter()
        .map(|c| decode_certificate(&c.raw_bytes))
        .collect::<Result<Vec<Certificate>, Error>>()?;

    let Some(direct_issuer) = chain.first() else {
        return Ok(None);
    };
    if direct_issuer.tbs_certificate().subject() != leaf.tbs_certificate().issuer() {
        return Ok(None);
    }

    if !within_valid_for(authenticated_time, &ca.valid_for) {
        return Err(Error::Certificate(CertificateError::OutsideCaValidity));
    }
    verify_time_window(leaf.tbs_certificate(), authenticated_time)?;
    verify_leaf_profile(leaf)?;

    // Walk leaf -> chain[0] -> chain[1] -> ... -> chain[last] (the root).
    // At i == chain.len() - 1 this verifies "the root signs the previous
    // certificate," never "the root signs itself" -- that self-signature
    // is deliberately never checked (the root is the trust anchor by
    // being listed here, not by its own signature).
    let mut signee = leaf;
    for (i, issuer) in chain.iter().enumerate() {
        if issuer.tbs_certificate().subject() != signee.tbs_certificate().issuer() {
            return Err(Error::Certificate(CertificateError::IssuerNameMismatch));
        }
        verify_time_window(issuer.tbs_certificate(), authenticated_time)?;
        verify_ca_profile(issuer, i)?;
        verify_signature(signee, issuer)?;
        signee = issuer;
    }

    let leaf_spki_der = encode_certificate_error(leaf.tbs_certificate().subject_public_key_info())?;
    let issuer_spki_der =
        encode_certificate_error(direct_issuer.tbs_certificate().subject_public_key_info())?;

    Ok(Some(ValidatedLeaf {
        leaf: leaf.clone(),
        leaf_spki_der,
        issuer_spki_der,
        chain_length: chain.len(),
    }))
}

fn decode_certificate(der_bytes: &[u8]) -> Result<Certificate, Error> {
    Certificate::from_der(der_bytes)
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))
}

fn encode_certificate_error(value: &impl Encode) -> Result<Vec<u8>, Error> {
    value
        .to_der()
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))
}

/// `[start, end)` half-open window check, matching [`ValidityPeriod`]'s
/// documented semantics (`end` exclusive, absent meaning still valid) --
/// the same convention [`crate::rekor::check_time_window`] uses for a
/// trusted log key's `validFor`.
fn within_valid_for(t: i64, window: &ValidityPeriod) -> bool {
    window.start <= t && window.end.is_none_or(|end| t < end)
}

/// X.509/RFC 5280 certificate validity is inclusive on both ends, unlike
/// [`within_valid_for`]'s half-open trust-store windows.
fn verify_time_window(tbs: &TbsCertificate, authenticated_time: i64) -> Result<(), Error> {
    let validity = tbs.validity();
    let not_before = unix_seconds(validity.not_before.to_unix_duration());
    let not_after = unix_seconds(validity.not_after.to_unix_duration());
    if authenticated_time < not_before || authenticated_time > not_after {
        return Err(Error::Certificate(
            CertificateError::OutsideCertificateValidity,
        ));
    }
    Ok(())
}

/// Verifies that `child`'s signature was produced by `issuer`'s private
/// key over `child`'s `TBSCertificate` bytes.
///
/// The signature algorithm allowlist is enforced by curve, not by trust:
/// `issuer`'s own key determines which single OID `child`'s declared
/// `signatureAlgorithm` must equal (P-256 issuer -> ECDSA-with-SHA-256;
/// P-384 issuer -> ECDSA-with-SHA-384); anything else is rejected before
/// any cryptographic verification is attempted.
fn verify_signature(child: &Certificate, issuer: &Certificate) -> Result<(), Error> {
    let issuer_spki_der =
        encode_certificate_error(issuer.tbs_certificate().subject_public_key_info())?;
    let issuer_key = EcdsaVerifyingKey::from_spki_der(&issuer_spki_der)?;

    let expected_oid = match issuer_key {
        EcdsaVerifyingKey::P256(_) => ECDSA_WITH_SHA_256,
        EcdsaVerifyingKey::P384(_) => ECDSA_WITH_SHA_384,
    };
    if child.signature_algorithm().oid != expected_oid {
        return Err(Error::Certificate(
            CertificateError::UnsupportedSignatureAlgorithm,
        ));
    }

    let tbs_der = encode_certificate_error(child.tbs_certificate())?;
    let signature_bytes = child.signature().raw_bytes();
    if issuer_key.verify_der(&tbs_der, signature_bytes) {
        Ok(())
    } else {
        Err(Error::Certificate(CertificateError::SignatureInvalid))
    }
}

/// The leaf-specific profile: `basicConstraints` absent-or-`CA:FALSE`,
/// `keyUsage` `digitalSignature`, `extKeyUsage` containing `codeSigning`,
/// and no unrecognized critical extension.
fn verify_leaf_profile(leaf: &Certificate) -> Result<(), Error> {
    let extensions = extensions_slice(leaf.tbs_certificate());

    reject_unknown_critical_extensions(extensions, &KNOWN_LEAF_EXTENSION_OIDS)?;

    if let Some(bc) =
        find_extension_decoded::<BasicConstraints>(extensions, ID_CE_BASIC_CONSTRAINTS)?
        && bc.ca
    {
        return Err(Error::Certificate(
            CertificateError::InvalidBasicConstraints,
        ));
    }

    let key_usage = find_extension_decoded::<KeyUsage>(extensions, ID_CE_KEY_USAGE)?
        .ok_or(Error::Certificate(CertificateError::MissingKeyUsage))?;
    if !key_usage.digital_signature() {
        return Err(Error::Certificate(CertificateError::MissingKeyUsage));
    }

    let eku = find_extension_decoded::<ExtendedKeyUsage>(extensions, ID_CE_EXT_KEY_USAGE)?
        .ok_or(Error::Certificate(CertificateError::MissingCodeSigningEku))?;
    if !eku.0.contains(&ID_KP_CODE_SIGNING) {
        return Err(Error::Certificate(CertificateError::MissingCodeSigningEku));
    }

    Ok(())
}

/// The certificate-authority profile applied to every certificate in a
/// chain (intermediate and root alike -- confirmed empirically that both
/// carry the same shape on the real trust root): no unrecognized critical
/// extension, `basicConstraints` `CA:TRUE` with any `pathLenConstraint`
/// respected, and `keyUsage` `keyCertSign`.
///
/// The critical-extension gate uses [`KNOWN_CA_CRITICAL_EXTENSION_OIDS`],
/// not the leaf's set, so a caller-supplied intermediate restricted by a
/// critical constraint this crate does not implement (`nameConstraints`
/// above all) fails closed instead of having that restriction silently
/// ignored, per RFC 5280 SS4.2/SS6.1.
///
/// `subordinate_ca_count` is the number of certificate authorities
/// strictly between `cert` and the leaf (0 for the certificate that
/// directly issues the leaf), which is what a `pathLenConstraint` bounds
/// per RFC 5280 SS4.2.1.9.
fn verify_ca_profile(cert: &Certificate, subordinate_ca_count: usize) -> Result<(), Error> {
    let extensions = extensions_slice(cert.tbs_certificate());

    reject_unknown_critical_extensions(extensions, &KNOWN_CA_CRITICAL_EXTENSION_OIDS)?;

    let bc = find_extension_decoded::<BasicConstraints>(extensions, ID_CE_BASIC_CONSTRAINTS)?
        .ok_or(Error::Certificate(
            CertificateError::InvalidBasicConstraints,
        ))?;
    if !bc.ca {
        return Err(Error::Certificate(
            CertificateError::InvalidBasicConstraints,
        ));
    }
    if let Some(max) = bc.path_len_constraint
        && subordinate_ca_count > usize::from(max)
    {
        return Err(Error::Certificate(CertificateError::PathLengthExceeded));
    }

    let key_usage = find_extension_decoded::<KeyUsage>(extensions, ID_CE_KEY_USAGE)?
        .ok_or(Error::Certificate(CertificateError::MissingKeyUsage))?;
    if !key_usage.key_cert_sign() {
        return Err(Error::Certificate(CertificateError::MissingKeyUsage));
    }

    Ok(())
}

fn extensions_slice(tbs: &TbsCertificate) -> &[Extension] {
    tbs.extensions().map_or(&[], |exts| exts.as_slice())
}

/// RFC 5280 SS4.2/SS6.1: a certificate carrying a critical extension the
/// verifier does not recognize must be rejected. `known` is the caller's
/// role-specific recognized set ([`KNOWN_LEAF_EXTENSION_OIDS`] for a leaf,
/// [`KNOWN_CA_CRITICAL_EXTENSION_OIDS`] for a chain certificate
/// authority).
fn reject_unknown_critical_extensions(
    extensions: &[Extension],
    known: &[ObjectIdentifier],
) -> Result<(), Error> {
    for ext in extensions {
        if ext.critical && !known.contains(&ext.extn_id) {
            return Err(Error::Certificate(
                CertificateError::UnknownCriticalExtension,
            ));
        }
    }
    Ok(())
}

/// Finds at most one extension with the given `oid` among `extensions`.
///
/// Shared by [`crate::sct`] and [`crate::fulcio`] as well as this
/// module's own profile checks, so "more than one" is defined and
/// rejected identically everywhere this crate looks up an extension by
/// OID.
///
/// # Errors
///
/// Returns [`CertificateError::DuplicateExtension`] if more than one
/// extension with `oid` is present.
pub(crate) fn find_extension(
    extensions: &[Extension],
    oid: ObjectIdentifier,
) -> Result<Option<&Extension>, Error> {
    let mut matches = extensions.iter().filter(|e| e.extn_id == oid);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::Certificate(CertificateError::DuplicateExtension));
    }
    Ok(first)
}

/// Like [`find_extension`], but also DER-decodes the match as `T`.
fn find_extension_decoded<'a, T: Decode<'a>>(
    extensions: &'a [Extension],
    oid: ObjectIdentifier,
) -> Result<Option<T>, Error> {
    let Some(ext) = find_extension(extensions, oid)? else {
        return Ok(None);
    };
    let decoded = T::from_der(ext.extn_value.as_bytes()).map_err(|_| {
        Error::Certificate(CertificateError::InvalidDer(
            "certificate extension value".to_owned(),
        ))
    })?;
    Ok(Some(decoded))
}

#[cfg(test)]
mod tests {
    use der::{Decode, Encode};

    use super::{ValidatedLeaf, validate_chain};
    use crate::bundle::Bundle;
    use crate::dsse::EcdsaVerifyingKey;
    use crate::error::{CertificateError, Error};
    use crate::trust::TrustStore;

    /// The real golden fixture's authenticated Rekor `integratedTime`
    /// (also used throughout `rekor.rs`'s tests): the only time this
    /// crate has independent evidence the leaf was actually used at.
    const REAL_INTEGRATED_TIME: i64 = 1_783_027_755;

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative)
    }

    fn read_fixture(relative: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(fixture_path(relative))?)
    }

    fn leaf_der_from(bundle_fixture: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bundle = Bundle::from_json(&read_fixture(bundle_fixture)?)?;
        Ok(bundle.verification_material.certificate.raw_bytes)
    }

    fn real_leaf_der() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        leaf_der_from("github-cli/tarball-user-slsa-provenance.json")
    }

    fn embedded_trust_store() -> Result<TrustStore, Box<dyn std::error::Error>> {
        Ok(TrustStore::embedded_public_good()?)
    }

    fn github_trust_store() -> Result<TrustStore, Box<dyn std::error::Error>> {
        Ok(TrustStore::from_json(&read_fixture(
            "trusted-roots/github.json",
        )?)?)
    }

    /// Parses `trusted-roots/public-good.json` as JSON, lets `f` mutate
    /// it, then re-serializes and re-parses as a [`TrustStore`] --
    /// mirrors `rekor.rs`'s `mutate_bundle_json` test helper, applied to
    /// the trust-root document instead of a bundle.
    fn mutate_public_good_json(
        f: impl FnOnce(&mut serde_json::Value) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<TrustStore, Box<dyn std::error::Error>> {
        let mut value: serde_json::Value =
            serde_json::from_slice(&read_fixture("trusted-roots/public-good.json")?)?;
        f(&mut value)?;
        Ok(TrustStore::from_json(&serde_json::to_vec(&value)?)?)
    }

    // -------------------------------------------------------------
    // Positive
    // -------------------------------------------------------------

    #[test]
    fn real_chain_validates_at_real_integrated_time() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = embedded_trust_store()?;
        let ValidatedLeaf {
            leaf_spki_der,
            issuer_spki_der,
            chain_length,
            ..
        } = validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME)?;

        // The matched candidate is `certificateAuthorities[1]`
        // (intermediate + root); confirmed empirically against the raw
        // fixture JSON before writing this test.
        if chain_length != 2 {
            return Err(format!("expected chain_length 2, got {chain_length}").into());
        }
        // The leaf's own key is P-256 (also confirmed in `dsse.rs`'s
        // tests); the issuing intermediate's key is P-384.
        if !matches!(
            EcdsaVerifyingKey::from_spki_der(&leaf_spki_der)?,
            EcdsaVerifyingKey::P256(_)
        ) {
            return Err("expected leaf SPKI to decode as P-256".into());
        }
        if !matches!(
            EcdsaVerifyingKey::from_spki_der(&issuer_spki_der)?,
            EcdsaVerifyingKey::P384(_)
        ) {
            return Err("expected issuer SPKI to decode as P-384".into());
        }
        Ok(())
    }

    // -------------------------------------------------------------
    // Negative: time
    // -------------------------------------------------------------

    #[test]
    fn time_before_leaf_not_before_fails() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = embedded_trust_store()?;
        // Real leaf notBefore is 1_783_027_754 (confirmed in `dsse.rs`'s
        // tests); one second earlier is outside the window.
        match validate_chain(&leaf_der, &trust_store, 1_783_027_753) {
            Err(Error::Certificate(CertificateError::OutsideCertificateValidity)) => Ok(()),
            other => Err(format!("expected OutsideCertificateValidity, got {other:?}").into()),
        }
    }

    #[test]
    fn time_after_leaf_not_after_fails() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = embedded_trust_store()?;
        // Real leaf notAfter is 1_783_028_354; one second later is
        // outside the window.
        match validate_chain(&leaf_der, &trust_store, 1_783_028_355) {
            Err(Error::Certificate(CertificateError::OutsideCertificateValidity)) => Ok(()),
            other => Err(format!("expected OutsideCertificateValidity, got {other:?}").into()),
        }
    }

    #[test]
    fn time_outside_synthetic_ca_valid_for_fails() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        // Shrink `certificateAuthorities[1]`'s (the one matching this
        // leaf) `validFor.end` to before the real integrated time,
        // leaving the certificates themselves untouched so name-chaining
        // and signatures still succeed.
        let trust_store = mutate_public_good_json(|v| {
            v["certificateAuthorities"][1]["validFor"]["end"] =
                serde_json::Value::String("2023-01-01T00:00:00Z".to_owned());
            Ok(())
        })?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::OutsideCaValidity)) => Ok(()),
            other => Err(format!("expected OutsideCaValidity, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: untrusted root / wrong root
    // -------------------------------------------------------------

    #[test]
    fn real_leaf_against_github_root_only_is_untrusted() -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = github_trust_store()?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::UntrustedCertificate)) => Ok(()),
            other => Err(format!("expected UntrustedCertificate, got {other:?}").into()),
        }
    }

    #[test]
    fn tsa_flavor_leaf_against_public_good_root_fails() -> Result<(), Box<dyn std::error::Error>> {
        // `tarball-github-release-tsa.json`'s leaf is issued by GitHub's
        // own Fulcio ("Fulcio Intermediate l1,O=GitHub, Inc."), not
        // sigstore.dev -- confirmed empirically -- so no public-good
        // certificate authority names it as an issuer.
        let leaf_der = leaf_der_from("github-cli/tarball-github-release-tsa.json")?;
        let trust_store = embedded_trust_store()?;
        // Use the TSA leaf's own notBefore so this test fails for the
        // *right* reason (untrusted issuer), not merely because the
        // real integrated time falls outside this unrelated leaf's
        // validity window.
        let bundle =
            Bundle::from_json(&read_fixture("github-cli/tarball-github-release-tsa.json")?)?;
        let leaf_info = crate::dsse::LeafCertificateInfo::from_certificate(
            &bundle.verification_material.certificate,
        )?;
        match validate_chain(&leaf_der, &trust_store, leaf_info.not_before) {
            Err(Error::Certificate(CertificateError::UntrustedCertificate)) => Ok(()),
            other => Err(format!("expected UntrustedCertificate, got {other:?}").into()),
        }
    }

    // -------------------------------------------------------------
    // Negative: signature / chain shape
    // -------------------------------------------------------------

    #[test]
    fn flipped_leaf_signature_byte_fails() -> Result<(), Box<dyn std::error::Error>> {
        let mut leaf_der = real_leaf_der()?;
        // The outer `Certificate ::= SEQUENCE { tbs, sigAlg, signature }`
        // BIT STRING is the last field, so flipping the very last byte
        // corrupts the signature while leaving the DER structurally
        // parseable (confirmed in Python before writing this test).
        let last = leaf_der.len() - 1;
        leaf_der[last] ^= 0x01;
        let trust_store = embedded_trust_store()?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::SignatureInvalid)) => Ok(()),
            other => Err(format!("expected SignatureInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn intermediate_removed_from_synthetic_trust_store_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        // Drop `certificateAuthorities[1]`'s intermediate
        // (`certChain.certificates[0]`), leaving only the root. No
        // remaining certificate anywhere in the trust store has
        // `subject == "CN=sigstore-intermediate,O=sigstore.dev"`
        // (`certificateAuthorities[0]` never had an intermediate either),
        // so this is untrusted, not merely a broken chain.
        let trust_store = mutate_public_good_json(|v| {
            let certs = v["certificateAuthorities"][1]["certChain"]["certificates"]
                .as_array_mut()
                .ok_or("expected certificates array")?;
            certs.remove(0);
            Ok(())
        })?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::UntrustedCertificate)) => Ok(()),
            other => Err(format!("expected UntrustedCertificate, got {other:?}").into()),
        }
    }

    #[test]
    fn mismatched_signature_algorithm_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        // The leaf's own key is P-256, but it declares
        // ecdsa-with-SHA384 (correctly -- its *actual* issuer is the
        // P-384 intermediate). Feeding the leaf to `verify_signature` as
        // its own "issuer" exercises the OID/curve allowlist directly:
        // a P-256 issuer key requires ecdsa-with-SHA256, so this must be
        // rejected before any signature math runs at all.
        let leaf_der = real_leaf_der()?;
        let leaf = der::Decode::from_der(leaf_der.as_slice())
            .map_err(|e: der::Error| format!("failed to parse leaf: {e}"))?;
        match super::verify_signature(&leaf, &leaf) {
            Err(Error::Certificate(CertificateError::UnsupportedSignatureAlgorithm)) => Ok(()),
            other => Err(format!("expected UnsupportedSignatureAlgorithm, got {other:?}").into()),
        }
    }

    /// Re-marks the extension at `oid` critical within `cert_der`'s
    /// `TBSCertificate`, leaving everything else -- including the now
    /// cryptographically-stale outer signature -- untouched.
    ///
    /// Safe for a "does the profile check fire" test specifically
    /// because both profile checks (which is where
    /// `reject_unknown_critical_extensions` is called) run *before* the
    /// signature over the certificate being tampered with is verified in
    /// [`try_validate_against_ca`]: the leaf's profile is checked before
    /// the chain walk starts, and each chain certificate's profile is
    /// checked at its own step, one step before its issuer verifies its
    /// signature. So the tampered certificate is rejected for the reason
    /// under test before the stale signature would ever matter.
    fn mark_extension_critical(
        cert_der: &[u8],
        oid: der::asn1::ObjectIdentifier,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use der::{Tag, TagNumber, Tagged};
        use x509_cert::ext::Extensions;

        let extensions_context_tag = Tag::ContextSpecific {
            constructed: true,
            number: TagNumber(3),
        };

        let outer = Vec::<der::asn1::AnyRef<'_>>::from_der(cert_der)?;
        let [cert_tbs, sig_alg, signature] = <[der::asn1::AnyRef<'_>; 3]>::try_from(outer)
            .map_err(|_| "expected exactly 3 top-level Certificate fields")?;

        let cert_tbs_der = cert_tbs.to_der()?;
        let mut tbs_fields = Vec::<der::asn1::AnyRef<'_>>::from_der(&cert_tbs_der)?;
        let ext_field = tbs_fields.pop().ok_or("TBSCertificate has no fields")?;
        if ext_field.tag() != extensions_context_tag {
            return Err("TBSCertificate's last field is not [3] EXPLICIT extensions".into());
        }
        let mut extensions = Extensions::from_der(ext_field.value())?;
        let target = extensions
            .iter_mut()
            .find(|e| e.extn_id == oid)
            .ok_or("extension to mark critical was not present")?;
        target.critical = true;

        let extensions_der = extensions.to_der()?;
        let new_ext_field = der::asn1::AnyRef::new(extensions_context_tag, &extensions_der)?;
        tbs_fields.push(new_ext_field);
        let new_tbs_der = tbs_fields.to_der()?;
        let new_tbs = der::asn1::AnyRef::from_der(&new_tbs_der)?;

        Ok(vec![new_tbs, sig_alg, signature].to_der()?)
    }

    #[test]
    fn unknown_critical_extension_on_leaf_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        // OID .12 (source repository URI) is outside the known set
        // (`keyUsage`/`extKeyUsage`/`basicConstraints`/SAN/SCT-list) and
        // non-critical on the real leaf; marking it critical must be
        // rejected per RFC 5280 SS4.2.
        let leaf_der = real_leaf_der()?;
        let source_repository_uri_oid =
            der::asn1::ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1.12");
        let tampered_der = mark_extension_critical(&leaf_der, source_repository_uri_oid)?;
        let trust_store = embedded_trust_store()?;
        match validate_chain(&tampered_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::UnknownCriticalExtension)) => Ok(()),
            other => Err(format!("expected UnknownCriticalExtension, got {other:?}").into()),
        }
    }

    /// Builds a trust store whose `certificateAuthorities[1]`
    /// intermediate -- the one that actually issues the real leaf -- has
    /// the extension at `oid` re-marked critical, via the same DER
    /// surgery [`mark_extension_critical`] performs on a leaf.
    fn public_good_with_critical_intermediate_extension(
        oid: der::asn1::ObjectIdentifier,
    ) -> Result<TrustStore, Box<dyn std::error::Error>> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        mutate_public_good_json(|v| {
            let slot =
                &mut v["certificateAuthorities"][1]["certChain"]["certificates"][0]["rawBytes"];
            let der = STANDARD.decode(slot.as_str().ok_or("expected rawBytes string")?)?;
            *slot = serde_json::Value::String(STANDARD.encode(mark_extension_critical(&der, oid)?));
            Ok(())
        })
    }

    #[test]
    fn unknown_critical_extension_on_chain_ca_is_rejected() -> Result<(), Box<dyn std::error::Error>>
    {
        // `authorityKeyIdentifier` is present and non-critical on the
        // real sigstore intermediate, and is outside the certificate-
        // authority known set (which holds only `keyUsage` and
        // `basicConstraints`), so marking it critical must be rejected
        // per RFC 5280 SS4.2 -- the same rule the leaf has always been
        // held to.
        let authority_key_identifier_oid = der::asn1::ObjectIdentifier::new_unwrap("2.5.29.35");
        let trust_store =
            public_good_with_critical_intermediate_extension(authority_key_identifier_oid)?;
        let leaf_der = real_leaf_der()?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::UnknownCriticalExtension)) => Ok(()),
            other => Err(format!("expected UnknownCriticalExtension, got {other:?}").into()),
        }
    }

    #[test]
    fn critical_extended_key_usage_on_chain_ca_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        // Pins that the certificate-authority known set is genuinely
        // narrower than the leaf's: `extKeyUsage` is a known *leaf*
        // extension, but nothing in `verify_ca_profile` enforces it, so
        // a critical one on a chain certificate must still fail closed.
        let ext_key_usage_oid = der::asn1::ObjectIdentifier::new_unwrap("2.5.29.37");
        let trust_store = public_good_with_critical_intermediate_extension(ext_key_usage_oid)?;
        let leaf_der = real_leaf_der()?;
        match validate_chain(&leaf_der, &trust_store, REAL_INTEGRATED_TIME) {
            Err(Error::Certificate(CertificateError::UnknownCriticalExtension)) => Ok(()),
            other => Err(format!("expected UnknownCriticalExtension, got {other:?}").into()),
        }
    }
}
