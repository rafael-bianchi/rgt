//! DSSE (Dead Simple Signing Envelope) signature verification.
//!
//! Pure verification functions: PAE (pre-authentication encoding), leaf
//! `SubjectPublicKeyInfo` extraction from the bundle's leaf certificate,
//! and ECDSA envelope-signature verification — run as step 7 of
//! [`crate::Verifier::verify_digest`]'s chain, after the leaf's X.509
//! chain ([`crate::x509`]) has already validated.
//!
//! [`LeafCertificateInfo`] only extracts a leaf's `SubjectPublicKeyInfo`
//! and `[notBefore, notAfter]` window directly from DER (used by
//! [`crate::rekor`]'s time-window check, which runs before full chain
//! validation); [`crate::x509::validate_chain`] is the actual
//! certificate-chain / Fulcio-profile validation (`DESIGN.md` "X.509 /
//! Fulcio validation profile").

use der::{Decode, Encode};
use signature::Verifier as _;
use spki::DecodePublicKey;

use crate::bundle::{Certificate, DsseEnvelope};
use crate::error::{CertificateError, ContentBindingError, Error};

#[cfg(test)]
std::thread_local! {
    // Deterministic, per-test-thread work counter for public-path performance
    // regressions. Thread-local state keeps parallel unit tests independent.
    static VERIFY_DER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_verify_der_call_count() {
    VERIFY_DER_CALLS.set(0);
}

#[cfg(test)]
pub(crate) fn verify_der_call_count() -> usize {
    VERIFY_DER_CALLS.get()
}

/// DSSE v1 pre-authentication encoding (PAE):
/// `"DSSEv1" SP len(payloadType) SP payloadType SP len(payload) SP payload`,
/// where both lengths are ASCII decimal encodings of a *byte* length and
/// `payload` is the raw decoded payload (not base64).
pub(crate) fn pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let payload_type = payload_type.as_bytes();
    let mut out = Vec::with_capacity(payload_type.len() + payload.len() + 16);
    out.extend_from_slice(b"DSSEv1 ");
    out.extend_from_slice(payload_type.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload_type);
    out.push(b' ');
    out.extend_from_slice(payload.len().to_string().as_bytes());
    out.push(b' ');
    out.extend_from_slice(payload);
    out
}

/// An ECDSA verifying key decoded from a `SubjectPublicKeyInfo`, for one
/// of the curves this crate supports.
///
/// Used both for a bundle's leaf certificate (DSSE envelope verification,
/// this module) and a trust store's transparency-log key (SET/checkpoint
/// verification, [`crate::rekor`]) — the same SPKI-decode and
/// DER-signature-verify logic applies to both, so it lives here once.
#[derive(Debug, Clone)]
pub(crate) enum EcdsaVerifyingKey {
    /// NIST P-256, verified over SHA-256 (Fulcio leaf certificates and
    /// the Rekor v1 log key).
    P256(p256::ecdsa::VerifyingKey),
    /// NIST P-384, verified over SHA-384 (current Fulcio intermediate/
    /// root CA signatures; `DESIGN.md` "X.509 / Fulcio validation
    /// profile").
    P384(p384::ecdsa::VerifyingKey),
}

impl EcdsaVerifyingKey {
    /// Decodes `spki_der` (a DER-encoded `SubjectPublicKeyInfo`) as a
    /// P-256 or P-384 ECDSA verifying key, whichever the SPKI identifies.
    ///
    /// Each candidate decode independently checks both the outer
    /// algorithm OID (id-ecPublicKey) and the curve parameter OID before
    /// accepting any key material, so there is no risk of a P-256 point
    /// being misinterpreted as a P-384 one or vice versa.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::UnsupportedKeyAlgorithm`] if `spki_der`
    /// is not a valid P-256 or P-384 `SubjectPublicKeyInfo`.
    pub(crate) fn from_spki_der(spki_der: &[u8]) -> Result<Self, Error> {
        if let Ok(key) = p256::ecdsa::VerifyingKey::from_public_key_der(spki_der) {
            return Ok(EcdsaVerifyingKey::P256(key));
        }
        if let Ok(key) = p384::ecdsa::VerifyingKey::from_public_key_der(spki_der) {
            return Ok(EcdsaVerifyingKey::P384(key));
        }
        Err(Error::Certificate(
            CertificateError::UnsupportedKeyAlgorithm,
        ))
    }

    /// Verifies `signature` (ASN.1 DER-encoded ECDSA) over `message`,
    /// hashing `message` with this key's curve-appropriate digest
    /// (SHA-256 for P-256, SHA-384 for P-384).
    ///
    /// Returns `false` both for a malformed DER signature and for a
    /// well-formed but cryptographically invalid one: both mean "this
    /// does not verify," and callers only ever need to distinguish
    /// "verifies" from "does not," picking their own specific error
    /// variant for the latter.
    pub(crate) fn verify_der(&self, message: &[u8], signature: &[u8]) -> bool {
        #[cfg(test)]
        VERIFY_DER_CALLS.set(VERIFY_DER_CALLS.get() + 1);

        match self {
            EcdsaVerifyingKey::P256(key) => p256::ecdsa::DerSignature::from_bytes(signature)
                .is_ok_and(|sig| key.verify(message, &sig).is_ok()),
            EcdsaVerifyingKey::P384(key) => p384::ecdsa::DerSignature::from_bytes(signature)
                .is_ok_and(|sig| key.verify(message, &sig).is_ok()),
        }
    }
}

/// A leaf certificate's public key and validity window, extracted from
/// its DER bytes, independent of chain validation.
///
/// [`crate::rekor`]'s time-window check (chain step 5) reads
/// `not_before`/`not_after` directly from the leaf's own DER, since it
/// runs *before* the chain is validated (chain step 6,
/// [`crate::x509::validate_chain`]) — indeed its output (an authenticated
/// integrated time) is what step 6 validates the chain *at*. Once the
/// chain has validated, [`crate::verifier::Verifier::verify_digest`] uses
/// the chain-validated [`crate::x509::ValidatedLeaf::leaf_spki_der`]
/// instead of `key` below for DSSE verification, so `key` is exercised
/// only by this module's own tests.
#[derive(Debug, Clone)]
pub(crate) struct LeafCertificateInfo {
    /// The certificate's subject public key.
    #[allow(dead_code)] // exercised by this module's tests; see doc above.
    pub(crate) key: EcdsaVerifyingKey,
    /// `notBefore`, unix seconds.
    pub(crate) not_before: i64,
    /// `notAfter`, unix seconds.
    pub(crate) not_after: i64,
}

impl LeafCertificateInfo {
    /// Parses `certificate` (DER bytes) far enough to extract its
    /// `SubjectPublicKeyInfo` and `[notBefore, notAfter]` validity
    /// window.
    ///
    /// # Errors
    ///
    /// Returns [`CertificateError::InvalidDer`] if the bytes are not a
    /// well-formed X.509 certificate, and
    /// [`CertificateError::UnsupportedKeyAlgorithm`] if its public key is
    /// not P-256 or P-384.
    pub(crate) fn from_certificate(certificate: &Certificate) -> Result<Self, Error> {
        let cert = x509_cert::Certificate::from_der(&certificate.raw_bytes)
            .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
        let tbs = cert.tbs_certificate();
        let spki_der = tbs
            .subject_public_key_info()
            .to_der()
            .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
        let key = EcdsaVerifyingKey::from_spki_der(&spki_der)?;
        let validity = tbs.validity();
        Ok(LeafCertificateInfo {
            key,
            not_before: unix_seconds(validity.not_before.to_unix_duration()),
            not_after: unix_seconds(validity.not_after.to_unix_duration()),
        })
    }
}

/// Converts a [`std::time::Duration`] since the unix epoch (as produced
/// by X.509 `Time::to_unix_duration`, always non-negative) to signed
/// unix seconds. Saturates rather than panicking in the practically
/// unreachable case of a duration past `i64::MAX` seconds — X.509 dates
/// are bounded to the year 9999 or so, far below that.
///
/// `pub(crate)`: also used by [`crate::x509`] for chain-validity checks
/// on every certificate in a chain, not just the leaf.
pub(crate) fn unix_seconds(d: std::time::Duration) -> i64 {
    i64::try_from(d.as_secs()).unwrap_or(i64::MAX)
}

/// Verifies a DSSE envelope's signature against a leaf certificate's
/// public key.
///
/// Dispatches on the leaf key's curve (P-256 verified over SHA-256,
/// P-384 over SHA-384); the signature must be ASN.1 DER-encoded ECDSA.
/// The bundle parser already enforces exactly one DSSE signature
/// ([`crate::error::ParseError::DsseSignatureCount`]), so there is
/// nothing to iterate here.
///
/// # Errors
///
/// Returns [`ContentBindingError::DsseSignatureInvalid`] if the
/// signature does not verify.
pub(crate) fn verify_envelope(
    envelope: &DsseEnvelope,
    leaf_key: &EcdsaVerifyingKey,
) -> Result<(), Error> {
    let message = pae(&envelope.payload_type, &envelope.payload);
    if leaf_key.verify_der(&message, &envelope.signature.sig) {
        Ok(())
    } else {
        Err(Error::ContentBinding(
            ContentBindingError::DsseSignatureInvalid,
        ))
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::{EcdsaVerifyingKey, LeafCertificateInfo, pae, verify_envelope};
    use crate::bundle::Bundle;
    use crate::error::{CertificateError, ContentBindingError, Error};

    fn fixture_path(relative: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(relative)
    }

    fn read_fixture(relative: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(std::fs::read(fixture_path(relative))?)
    }

    fn real_bundle() -> Result<Bundle, Box<dyn std::error::Error>> {
        let bytes = read_fixture("github-cli/tarball-user-slsa-provenance.json")?;
        Ok(Bundle::from_json(&bytes)?)
    }

    #[test]
    fn pae_matches_hand_computed_vector() -> Result<(), Box<dyn std::error::Error>> {
        let encoded = pae("application/vnd.in-toto+json", b"hello");
        if encoded != b"DSSEv1 28 application/vnd.in-toto+json 5 hello" {
            return Err(format!("unexpected PAE encoding: {encoded:?}").into());
        }
        Ok(())
    }

    #[test]
    fn pae_uses_byte_length_not_char_count() -> Result<(), Box<dyn std::error::Error>> {
        // "héllo" is 5 chars but 6 bytes in UTF-8 (é is 2 bytes); the PAE
        // length prefix must reflect the byte length.
        let encoded = pae("x", "héllo".as_bytes());
        if encoded != "DSSEv1 1 x 6 héllo".as_bytes() {
            return Err(format!("unexpected PAE encoding: {encoded:?}").into());
        }
        Ok(())
    }

    #[test]
    fn extracts_leaf_key_and_validity_from_real_fixture() -> Result<(), Box<dyn std::error::Error>>
    {
        let bundle = real_bundle()?;
        let info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        if !matches!(info.key, EcdsaVerifyingKey::P256(_)) {
            return Err("expected a P-256 leaf key".into());
        }
        if info.not_before != 1_783_027_754 {
            return Err(format!("unexpected notBefore: {}", info.not_before).into());
        }
        if info.not_after != 1_783_028_354 {
            return Err(format!("unexpected notAfter: {}", info.not_after).into());
        }
        Ok(())
    }

    #[test]
    fn verifies_real_dsse_envelope_with_real_leaf_key() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        verify_envelope(&bundle.dsse_envelope, &info.key)?;
        Ok(())
    }

    #[test]
    fn rejects_flipped_byte_in_dsse_payload() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        let mut envelope = bundle.dsse_envelope.clone();
        envelope.payload[0] ^= 0x01;
        match verify_envelope(&envelope, &info.key) {
            Err(Error::ContentBinding(ContentBindingError::DsseSignatureInvalid)) => Ok(()),
            other => Err(format!("expected DsseSignatureInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_flipped_byte_in_dsse_signature() -> Result<(), Box<dyn std::error::Error>> {
        let bundle = real_bundle()?;
        let info =
            LeafCertificateInfo::from_certificate(&bundle.verification_material.certificate)?;
        let mut envelope = bundle.dsse_envelope.clone();
        envelope.signature.sig[0] ^= 0x01;
        match verify_envelope(&envelope, &info.key) {
            Err(Error::ContentBinding(ContentBindingError::DsseSignatureInvalid)) => Ok(()),
            other => Err(format!("expected DsseSignatureInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_invalid_der_certificate() -> Result<(), Box<dyn std::error::Error>> {
        let bogus = crate::bundle::Certificate {
            raw_bytes: b"not a certificate".to_vec(),
        };
        match LeafCertificateInfo::from_certificate(&bogus) {
            Err(Error::Certificate(CertificateError::InvalidDer(_))) => Ok(()),
            other => Err(format!("expected InvalidDer, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_unsupported_key_algorithm() -> Result<(), Box<dyn std::error::Error>> {
        // The public-good trust root's second tlog (Rekor v2) uses
        // Ed25519 — a real SPKI this crate deliberately does not support
        // yet (DESIGN.md "Rekor v1 / v2 scope").
        let bytes = read_fixture("trusted-roots/public-good.json")?;
        let root: serde_json::Value = serde_json::from_slice(&bytes)?;
        let raw_bytes = root["tlogs"][1]["publicKey"]["rawBytes"]
            .as_str()
            .ok_or("expected tlogs[1].publicKey.rawBytes")?;
        if !root["tlogs"][1]["publicKey"]["keyDetails"]
            .as_str()
            .is_some_and(|s| s.contains("ED25519"))
        {
            return Err("expected tlogs[1] to be the ED25519 log".into());
        }
        let spki_der = STANDARD.decode(raw_bytes)?;
        match EcdsaVerifyingKey::from_spki_der(&spki_der) {
            Err(Error::Certificate(CertificateError::UnsupportedKeyAlgorithm)) => Ok(()),
            other => Err(format!("expected UnsupportedKeyAlgorithm, got {other:?}").into()),
        }
    }
}
