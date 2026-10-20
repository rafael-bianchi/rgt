//! Embedded SCT (Signed Certificate Timestamp) verification.
//!
//! Implements `DESIGN.md`'s "SCT verification is day one" requirement:
//! extraction of the embedded SCT list extension (RFC 6962), exact TLS
//! re-serialization of the precertificate signing input, and signature
//! verification against a trust-store Certificate Transparency log key.
//! An SCT is evidence about certificate *issuance*, not artifact-signing
//! time -- it never substitutes for the Rekor SET/checkpoint
//! ([`crate::rekor`]).
//!
//! ## The SCT list extension
//!
//! The leaf's SCT-list extension (OID 1.3.6.1.4.1.11129.2.4.2) is
//! double-OCTET-STRING-wrapped: the general X.509 `extnValue` OCTET
//! STRING (already unwrapped by `x509-cert`'s extension parsing) itself
//! contains the DER encoding of *another* OCTET STRING (RFC 6962's own
//! `SignedCertificateTimestampList ::= OCTET STRING`), and only *that*
//! one's content is the raw TLS-encoded list. Confirmed byte-for-byte
//! against the real golden fixture before writing this (both by hand and
//! cross-checked against `cryptography`'s `tbs_precertificate_bytes` in
//! Python) -- easy to miss since every *other* extension this crate reads
//! (`KeyUsage`, `BasicConstraints`, ...) is single-wrapped.
//!
//! ## The precertificate signing input
//!
//! Each SCT signs a `digitally-signed` struct (RFC 6962 SS3.2) over a
//! "precertificate" `TBSCertificate`: the real, issued `TBSCertificate`
//! with the SCT-list extension itself removed (it cannot very well sign
//! over its own bytes) and every other field, including the real
//! `subjectPublicKeyInfo`, `extensions` order, and even the real Fulcio
//! identity extensions, byte-for-byte unchanged. This module reconstructs
//! that `TBSCertificate` by DER surgery: decode the original TBS as a flat
//! sequence of opaque `AnyRef` TLVs, decode just the trailing `[3]`
//! extensions wrapper as typed `Extension`s, drop the one matching the
//! SCT-list OID, re-encode the rest, and re-assemble. This is exactly
//! what `cryptography`'s `Certificate.tbs_precertificate_bytes` does;
//! this module's positive test confirms this crate's reconstruction is
//! byte-identical to that reference implementation's output.
//!
//! Run as step 8 of [`crate::Verifier::verify_digest`]'s chain, after the
//! leaf's X.509 chain ([`crate::x509`]) has already validated.

use der::asn1::{AnyRef, OctetString};
use der::{Decode, Encode, Tag, TagNumber, Tagged};
use sha2::{Digest, Sha256};
use x509_cert::TbsCertificate;
use x509_cert::ext::Extensions;

use crate::dsse::EcdsaVerifyingKey;
use crate::error::{CertificateError, Error};
use crate::trust::TrustStore;
use crate::x509::{SCT_LIST_OID, ValidatedLeaf, find_extension};

/// RFC 6962 SS3.2 `Version.v1`.
const SCT_VERSION_V1: u8 = 0;
/// RFC 6962 SS3.2 `HashAlgorithm.sha256` (borrowed from TLS 1.2's
/// `SignatureAndHashAlgorithm`, RFC 5246 SS7.4.1.4.1).
const HASH_ALGORITHM_SHA256: u8 = 4;
/// RFC 5246 SS7.4.1.4.1 `SignatureAlgorithm.ecdsa`.
const SIGNATURE_ALGORITHM_ECDSA: u8 = 3;
/// RFC 6962 SS3.2 `SignatureType.certificate_timestamp`.
const SIGNATURE_TYPE_CERTIFICATE_TIMESTAMP: u8 = 0;
/// RFC 6962 SS3.2 `LogEntryType.precert_entry`.
const LOG_ENTRY_TYPE_PRECERT: u16 = 1;
/// The `[3] EXPLICIT extensions` context tag every v3 `TBSCertificate`
/// uses -- shared shape knowledge with [`crate::x509`], duplicated here
/// as a constant rather than imported since it is `der`'s own `Tag` type,
/// not this crate's.
const EXTENSIONS_CONTEXT_TAG: Tag = Tag::ContextSpecific {
    constructed: true,
    number: TagNumber(3),
};

/// A single parsed SCT (RFC 6962 SS3.2), before any verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSct {
    pub(crate) version: u8,
    pub(crate) log_id: [u8; 32],
    pub(crate) timestamp_ms: u64,
    pub(crate) extensions: Vec<u8>,
    pub(crate) hash_algorithm: u8,
    pub(crate) signature_algorithm: u8,
    pub(crate) signature: Vec<u8>,
}

/// Verifies `validated`'s embedded SCTs against `trust_store`'s CT logs.
///
/// Threshold is "at least one SCT verifies against a trusted CT log"
/// (`DESIGN.md`): every embedded SCT is tried, in the order the
/// certificate lists them, and the first one that verifies wins. If none
/// do, the first SCT's specific failure reason is returned (mirroring
/// [`crate::x509::validate_chain`]'s "attribute the sole reason when
/// there is one" behavior), except when the extension itself is absent
/// or the list is empty, which is unconditionally
/// [`CertificateError::SctMissing`].
///
/// # Errors
///
/// Returns [`CertificateError::SctMissing`] if the leaf has no embedded
/// SCT list (or an empty one), and otherwise whatever specific
/// [`CertificateError`] (`SctInvalid` / `UnknownCtLog` /
/// `SctOutsideKeyValidity`) the SCTs failed on.
pub(crate) fn verify_embedded_scts(
    validated: &ValidatedLeaf,
    trust_store: &TrustStore,
) -> Result<(), Error> {
    let tbs = validated.leaf.tbs_certificate();
    let sct_list_bytes = extract_sct_list_bytes(tbs)?;
    let scts = parse_sct_list(&sct_list_bytes)?;
    if scts.is_empty() {
        return Err(Error::Certificate(CertificateError::SctMissing));
    }

    let tbs_der = tbs
        .to_der()
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    let precert_tbs = precert_tbs_der(&tbs_der)?;
    let issuer_key_hash: [u8; 32] = Sha256::digest(&validated.issuer_spki_der).into();

    let mut first_error: Option<Error> = None;
    for sct in &scts {
        match verify_one_sct(sct, &precert_tbs, issuer_key_hash, trust_store) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    Err(first_error.unwrap_or(Error::Certificate(CertificateError::SctInvalid)))
}

/// Extracts the raw TLS-encoded `SignedCertificateTimestampList` bytes
/// from `tbs`'s SCT-list extension (see this module's doc comment for
/// why two layers of OCTET STRING unwrapping are needed).
///
/// # Errors
///
/// Returns [`CertificateError::SctMissing`] if the extension is absent,
/// [`CertificateError::DuplicateExtension`] if it appears more than once,
/// and [`CertificateError::InvalidDer`] if its value is not a
/// well-formed inner OCTET STRING.
fn extract_sct_list_bytes(tbs: &TbsCertificate) -> Result<Vec<u8>, Error> {
    let extensions = tbs.extensions().map_or(&[][..], |exts| exts.as_slice());
    let ext = find_extension(extensions, SCT_LIST_OID)?
        .ok_or(Error::Certificate(CertificateError::SctMissing))?;
    let inner = OctetString::from_der(ext.extn_value.as_bytes())
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    Ok(inner.as_bytes().to_vec())
}

/// Parses a TLS-encoded `SignedCertificateTimestampList` (RFC 6962
/// SS3.3): a 2-byte total-length prefix followed by one or more SCTs,
/// each itself 2-byte-length-prefixed.
///
/// # Errors
///
/// Returns [`CertificateError::SctInvalid`] if `bytes` is truncated, the
/// length prefix does not match the remaining byte count, or any
/// individual SCT is malformed.
pub(crate) fn parse_sct_list(bytes: &[u8]) -> Result<Vec<ParsedSct>, Error> {
    let mut cursor = bytes;
    let list_len = usize::from(take_u16(&mut cursor)?);
    if list_len != cursor.len() {
        return Err(sct_invalid());
    }
    let mut scts = Vec::new();
    while !cursor.is_empty() {
        let sct_len = usize::from(take_u16(&mut cursor)?);
        let sct_bytes = take(&mut cursor, sct_len)?;
        scts.push(parse_single_sct(sct_bytes)?);
    }
    Ok(scts)
}

/// Parses one SCT (RFC 6962 SS3.2): `version(1) || logId(32) ||
/// timestamp(8) || extensions(2-byte len + bytes) || hash_alg(1) ||
/// sig_alg(1) || signature(2-byte len + DER ECDSA bytes)`, with no
/// trailing bytes.
fn parse_single_sct(bytes: &[u8]) -> Result<ParsedSct, Error> {
    let mut cursor = bytes;
    let version = take_u8(&mut cursor)?;
    let log_id = take_array32(&mut cursor)?;
    let timestamp_ms = take_u64(&mut cursor)?;
    let ext_len = usize::from(take_u16(&mut cursor)?);
    let extensions = take(&mut cursor, ext_len)?.to_vec();
    let hash_algorithm = take_u8(&mut cursor)?;
    let signature_algorithm = take_u8(&mut cursor)?;
    let sig_len = usize::from(take_u16(&mut cursor)?);
    let signature = take(&mut cursor, sig_len)?.to_vec();
    if !cursor.is_empty() {
        return Err(sct_invalid());
    }
    Ok(ParsedSct {
        version,
        log_id,
        timestamp_ms,
        extensions,
        hash_algorithm,
        signature_algorithm,
        signature,
    })
}

fn sct_invalid() -> Error {
    Error::Certificate(CertificateError::SctInvalid)
}

fn take<'a>(cursor: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if cursor.len() < n {
        return Err(sct_invalid());
    }
    let (head, tail) = cursor.split_at(n);
    *cursor = tail;
    Ok(head)
}

fn take_u8(cursor: &mut &[u8]) -> Result<u8, Error> {
    Ok(take(cursor, 1)?[0])
}

fn take_u16(cursor: &mut &[u8]) -> Result<u16, Error> {
    let b = take(cursor, 2)?;
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

fn take_u64(cursor: &mut &[u8]) -> Result<u64, Error> {
    let b = take(cursor, 8)?;
    let array: [u8; 8] = b.try_into().map_err(|_| sct_invalid())?;
    Ok(u64::from_be_bytes(array))
}

fn take_array32(cursor: &mut &[u8]) -> Result<[u8; 32], Error> {
    let b = take(cursor, 32)?;
    b.try_into().map_err(|_| sct_invalid())
}

/// Reconstructs the precertificate `TBSCertificate` DER bytes: `tbs_der`
/// (the real, issued `TBSCertificate`'s canonical DER encoding) with the
/// SCT-list extension removed.
///
/// Operates on raw `AnyRef` TLVs rather than typed `x509-cert` struct
/// fields deliberately: `TbsCertificateInner`'s fields are private with
/// no public constructor (by design -- `x509-cert` is a parsing crate
/// here, not a builder), so there is no supported way to hand back a
/// *modified* typed value. Slicing at the TLV level sidesteps that
/// entirely and needs no knowledge of what any field other than
/// `extensions` means; DER's canonical-encoding guarantee is what makes
/// re-assembling opaque TLVs byte-safe.
///
/// # Errors
///
/// Returns [`CertificateError::InvalidDer`] if `tbs_der` does not decode
/// as a plain sequence of TLVs, if its last field is not the expected
/// `[3] EXPLICIT` extensions wrapper, or if that wrapper's content does
/// not decode as `SEQUENCE OF Extension`.
fn precert_tbs_der(tbs_der: &[u8]) -> Result<Vec<u8>, Error> {
    let mut fields = Vec::<AnyRef<'_>>::from_der(tbs_der)
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    let ext_field = fields.pop().ok_or_else(|| {
        Error::Certificate(CertificateError::InvalidDer(
            "TBSCertificate has no fields".to_owned(),
        ))
    })?;
    if ext_field.tag() != EXTENSIONS_CONTEXT_TAG {
        return Err(Error::Certificate(CertificateError::InvalidDer(
            "TBSCertificate's last field is not a [3] EXPLICIT extensions wrapper".to_owned(),
        )));
    }

    let extensions = Extensions::from_der(ext_field.value())
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    let filtered: Extensions = extensions
        .into_iter()
        .filter(|ext| ext.extn_id != SCT_LIST_OID)
        .collect();
    let filtered_der = filtered
        .to_der()
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    let new_ext_field = AnyRef::new(EXTENSIONS_CONTEXT_TAG, &filtered_der)
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))?;
    fields.push(new_ext_field);

    fields
        .to_der()
        .map_err(|e| Error::Certificate(CertificateError::InvalidDer(e.to_string())))
}

/// Builds the RFC 6962 SS3.2 `digitally-signed` struct an SCT's signature
/// covers: `version(1)=0 || signature_type(1)=0 || timestamp(8) ||
/// entry_type(2)=1(precert_entry) || issuer_key_hash(32) ||
/// tbs_len(3) || tbs || ct_extensions(2-byte len + bytes)`.
///
/// # Errors
///
/// Returns [`CertificateError::SctInvalid`] if `precert_tbs` or
/// `ct_extensions` is too long for its TLS length-prefix field --
/// unreachable for any real certificate (kilobytes, not megabytes), kept
/// as a checked conversion rather than a truncating cast on principle.
fn digitally_signed_bytes(
    timestamp_ms: u64,
    issuer_key_hash: [u8; 32],
    precert_tbs: &[u8],
    ct_extensions: &[u8],
) -> Result<Vec<u8>, Error> {
    let tbs_len = u32::try_from(precert_tbs.len()).map_err(|_| sct_invalid())?;
    if tbs_len > 0x00FF_FFFF {
        return Err(sct_invalid());
    }
    let ct_ext_len = u16::try_from(ct_extensions.len()).map_err(|_| sct_invalid())?;

    let mut out =
        Vec::with_capacity(1 + 1 + 8 + 2 + 32 + 3 + precert_tbs.len() + 2 + ct_extensions.len());
    out.push(SCT_VERSION_V1);
    out.push(SIGNATURE_TYPE_CERTIFICATE_TIMESTAMP);
    out.extend_from_slice(&timestamp_ms.to_be_bytes());
    out.extend_from_slice(&LOG_ENTRY_TYPE_PRECERT.to_be_bytes());
    out.extend_from_slice(&issuer_key_hash);
    out.extend_from_slice(&tbs_len.to_be_bytes()[1..]); // 3-byte big-endian length
    out.extend_from_slice(precert_tbs);
    out.extend_from_slice(&ct_ext_len.to_be_bytes());
    out.extend_from_slice(ct_extensions);
    Ok(out)
}

/// Verifies one SCT: selects its CT log by recomputed key id (same
/// hygiene as [`crate::rekor::select_log_key`] -- never trust the trust
/// store's own `logId.keyId` label further than "some trusted key hashes
/// to this id"), checks its timestamp against that log key's `validFor`
/// window, and verifies its signature over the `digitally-signed` struct.
///
/// # Errors
///
/// Returns [`CertificateError::UnknownCtLog`] if no trust-store CT log
/// matches `sct.log_id`, [`CertificateError::SctOutsideKeyValidity`] if
/// the timestamp is outside that log key's `validFor` window, and
/// [`CertificateError::SctInvalid`] if the version/hash/signature
/// algorithm fields are not the v1/SHA-256/ECDSA this crate implements or
/// the signature does not verify.
pub(crate) fn verify_one_sct(
    sct: &ParsedSct,
    precert_tbs: &[u8],
    issuer_key_hash: [u8; 32],
    trust_store: &TrustStore,
) -> Result<(), Error> {
    let ctlog = trust_store
        .ctlogs
        .iter()
        .find(|log| Sha256::digest(&log.public_key.raw_bytes).as_slice() == sct.log_id)
        .ok_or(Error::Certificate(CertificateError::UnknownCtLog))?;

    let timestamp_seconds = i64::try_from(sct.timestamp_ms / 1000).map_err(|_| sct_invalid())?;
    let window = &ctlog.public_key.valid_for;
    let within_window =
        window.start <= timestamp_seconds && window.end.is_none_or(|end| timestamp_seconds < end);
    if !within_window {
        return Err(Error::Certificate(CertificateError::SctOutsideKeyValidity));
    }

    if sct.version != SCT_VERSION_V1
        || sct.hash_algorithm != HASH_ALGORITHM_SHA256
        || sct.signature_algorithm != SIGNATURE_ALGORITHM_ECDSA
    {
        return Err(sct_invalid());
    }

    let message = digitally_signed_bytes(
        sct.timestamp_ms,
        issuer_key_hash,
        precert_tbs,
        &sct.extensions,
    )?;
    let log_key = EcdsaVerifyingKey::from_spki_der(&ctlog.public_key.raw_bytes)?;
    if log_key.verify_der(&message, &sct.signature) {
        Ok(())
    } else {
        Err(sct_invalid())
    }
}

#[cfg(test)]
mod tests {
    use der::{Decode, Encode};
    use sha2::{Digest, Sha256};

    use super::{
        ParsedSct, digitally_signed_bytes, extract_sct_list_bytes, parse_sct_list, precert_tbs_der,
        verify_embedded_scts, verify_one_sct,
    };
    use crate::bundle::Bundle;
    use crate::error::{CertificateError, Error};
    use crate::trust::TrustStore;
    use crate::x509::{ValidatedLeaf, validate_chain};

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

    fn real_validated_leaf() -> Result<ValidatedLeaf, Box<dyn std::error::Error>> {
        let leaf_der = real_leaf_der()?;
        let trust_store = embedded_trust_store()?;
        Ok(validate_chain(
            &leaf_der,
            &trust_store,
            REAL_INTEGRATED_TIME,
        )?)
    }

    fn mutate_public_good_json(
        f: impl FnOnce(&mut serde_json::Value) -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<TrustStore, Box<dyn std::error::Error>> {
        let mut value: serde_json::Value =
            serde_json::from_slice(&read_fixture("trusted-roots/public-good.json")?)?;
        f(&mut value)?;
        Ok(TrustStore::from_json(&serde_json::to_vec(&value)?)?)
    }

    // -------------------------------------------------------------
    // Positive: real fixture, real trust store
    // -------------------------------------------------------------

    #[test]
    fn real_scts_verify_against_real_trust_store() -> Result<(), Box<dyn std::error::Error>> {
        let validated = real_validated_leaf()?;
        let trust_store = embedded_trust_store()?;
        verify_embedded_scts(&validated, &trust_store)?;
        Ok(())
    }

    #[test]
    fn real_sct_list_has_exactly_one_sct_with_known_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let validated = real_validated_leaf()?;
        let tbs = validated.leaf.tbs_certificate();
        let list_bytes = extract_sct_list_bytes(tbs)?;
        let scts = parse_sct_list(&list_bytes)?;
        if scts.len() != 1 {
            return Err(format!("expected 1 SCT, got {}", scts.len()).into());
        }
        let sct = &scts[0];
        if sct.version != 0 {
            return Err(format!("unexpected version: {}", sct.version).into());
        }
        let expected_log_id =
            hex::decode("dd3d306ac6c7113263191e1c99673702a24a5eb8de3cadff878a72802f29ee8e")?;
        if sct.log_id.as_slice() != expected_log_id.as_slice() {
            return Err("unexpected log_id".into());
        }
        if sct.timestamp_ms != 1_783_027_754_956 {
            return Err(format!("unexpected timestamp_ms: {}", sct.timestamp_ms).into());
        }
        if !sct.extensions.is_empty() {
            return Err("expected empty SCT extensions".into());
        }
        if sct.hash_algorithm != 4 {
            return Err(format!("unexpected hash_algorithm: {}", sct.hash_algorithm).into());
        }
        if sct.signature_algorithm != 3 {
            return Err(format!(
                "unexpected signature_algorithm: {}",
                sct.signature_algorithm
            )
            .into());
        }
        if sct.signature.len() != 71 {
            return Err(format!("unexpected signature length: {}", sct.signature.len()).into());
        }
        Ok(())
    }

    #[test]
    fn real_precert_tbs_has_known_length_matching_cryptography_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        // 1540 bytes: confirmed byte-for-byte identical to Python's
        // `cryptography` library's `Certificate.tbs_precertificate_bytes`
        // on this exact fixture before this module was written (see the
        // module doc comment).
        let validated = real_validated_leaf()?;
        let tbs_der = validated
            .leaf
            .tbs_certificate()
            .to_der()
            .map_err(|e| format!("failed to re-encode tbs: {e}"))?;
        let precert_tbs = precert_tbs_der(&tbs_der)?;
        if precert_tbs.len() != 1540 {
            return Err(format!("unexpected precert TBS length: {}", precert_tbs.len()).into());
        }
        Ok(())
    }

    // -------------------------------------------------------------
    // Positive: synthetic vectors (TLS-struct builder + SCT list parser)
    // -------------------------------------------------------------

    /// Hand-builds one TLS-encoded SCT (RFC 6962 SS3.2) from its fields,
    /// for round-tripping through [`parse_single_sct`]/[`parse_sct_list`]
    /// without depending on any real certificate.
    fn build_synthetic_sct(
        log_id: [u8; 32],
        timestamp_ms: u64,
        extensions: &[u8],
        hash_algorithm: u8,
        signature_algorithm: u8,
        signature: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0); // version
        out.extend_from_slice(&log_id);
        out.extend_from_slice(&timestamp_ms.to_be_bytes());
        out.extend_from_slice(&(u16::try_from(extensions.len()).unwrap_or(0)).to_be_bytes());
        out.extend_from_slice(extensions);
        out.push(hash_algorithm);
        out.push(signature_algorithm);
        out.extend_from_slice(&(u16::try_from(signature.len()).unwrap_or(0)).to_be_bytes());
        out.extend_from_slice(signature);
        out
    }

    fn wrap_sct_list(scts: &[Vec<u8>]) -> Vec<u8> {
        let mut list_body = Vec::new();
        for sct in scts {
            list_body.extend_from_slice(&(u16::try_from(sct.len()).unwrap_or(0)).to_be_bytes());
            list_body.extend_from_slice(sct);
        }
        let mut out = Vec::new();
        out.extend_from_slice(&(u16::try_from(list_body.len()).unwrap_or(0)).to_be_bytes());
        out.extend_from_slice(&list_body);
        out
    }

    #[test]
    fn parses_synthetic_sct_list_with_two_entries() -> Result<(), Box<dyn std::error::Error>> {
        let sct_a = build_synthetic_sct([0xAA; 32], 1_000, &[], 4, 3, &[0x30, 0x02, 0x01, 0x00]);
        let sct_b = build_synthetic_sct([0xBB; 32], 2_000, &[0x01, 0x02], 4, 3, &[0x30, 0x00]);
        let list = wrap_sct_list(&[sct_a, sct_b]);

        let parsed = parse_sct_list(&list)?;
        if parsed.len() != 2 {
            return Err(format!("expected 2 SCTs, got {}", parsed.len()).into());
        }
        if parsed[0].log_id != [0xAA; 32] || parsed[0].timestamp_ms != 1_000 {
            return Err("unexpected first synthetic SCT fields".into());
        }
        if parsed[1].log_id != [0xBB; 32]
            || parsed[1].timestamp_ms != 2_000
            || parsed[1].extensions != vec![0x01, 0x02]
        {
            return Err("unexpected second synthetic SCT fields".into());
        }
        Ok(())
    }

    #[test]
    fn rejects_sct_list_with_wrong_total_length_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let sct_a = build_synthetic_sct([0xAA; 32], 1_000, &[], 4, 3, &[0x30, 0x00]);
        let mut list = wrap_sct_list(&[sct_a]);
        // Corrupt the 2-byte total-length prefix so it no longer matches
        // the actual remaining byte count.
        list[1] = list[1].wrapping_add(1);
        match parse_sct_list(&list) {
            Err(Error::Certificate(CertificateError::SctInvalid)) => Ok(()),
            other => Err(format!("expected SctInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_sct_with_trailing_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let mut sct_a = build_synthetic_sct([0xAA; 32], 1_000, &[], 4, 3, &[0x30, 0x00]);
        sct_a.push(0xFF); // one byte more than the SCT's own fields account for
        let list = wrap_sct_list(&[sct_a]);
        match parse_sct_list(&list) {
            Err(Error::Certificate(CertificateError::SctInvalid)) => Ok(()),
            other => Err(format!("expected SctInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_truncated_sct() -> Result<(), Box<dyn std::error::Error>> {
        // A 2-byte SCT-length prefix claiming more bytes than actually
        // follow.
        let list = vec![0x00, 0x02, 0x00, 0x05, 0xAA];
        match parse_sct_list(&list) {
            Err(Error::Certificate(CertificateError::SctInvalid)) => Ok(()),
            other => Err(format!("expected SctInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn digitally_signed_bytes_matches_hand_computed_vector()
    -> Result<(), Box<dyn std::error::Error>> {
        let issuer_key_hash = [0x11; 32];
        let tbs = vec![0xAA, 0xBB, 0xCC];
        let ct_extensions = vec![0xDD, 0xEE];
        let built =
            digitally_signed_bytes(0x0102_0304_0506_0708, issuer_key_hash, &tbs, &ct_extensions)?;

        let mut expected = Vec::new();
        expected.push(0); // version
        expected.push(0); // signature_type
        expected.extend_from_slice(&0x0102_0304_0506_0708u64.to_be_bytes()); // timestamp
        expected.extend_from_slice(&1u16.to_be_bytes()); // entry_type = precert_entry
        expected.extend_from_slice(&issuer_key_hash);
        expected.extend_from_slice(&[0x00, 0x00, 0x03]); // tbs_len = 3, 3-byte BE
        expected.extend_from_slice(&tbs);
        expected.extend_from_slice(&2u16.to_be_bytes()); // ct_extensions len
        expected.extend_from_slice(&ct_extensions);

        if built != expected {
            return Err(format!("built={built:02x?}\nexpected={expected:02x?}").into());
        }
        Ok(())
    }

    // -------------------------------------------------------------
    // Negative
    // -------------------------------------------------------------

    #[test]
    fn ctlogs_stripped_from_synthetic_root_fails_unknown_ctlog()
    -> Result<(), Box<dyn std::error::Error>> {
        let validated = real_validated_leaf()?;
        let trust_store = mutate_public_good_json(|v| {
            v["ctlogs"] = serde_json::Value::Array(vec![]);
            Ok(())
        })?;
        match verify_embedded_scts(&validated, &trust_store) {
            Err(Error::Certificate(CertificateError::UnknownCtLog)) => Ok(()),
            other => Err(format!("expected UnknownCtLog, got {other:?}").into()),
        }
    }

    #[test]
    fn flipped_sct_signature_byte_fails() -> Result<(), Box<dyn std::error::Error>> {
        let validated = real_validated_leaf()?;
        let trust_store = embedded_trust_store()?;

        let tbs = validated.leaf.tbs_certificate();
        let list_bytes = extract_sct_list_bytes(tbs)?;
        let scts = parse_sct_list(&list_bytes)?;
        let mut corrupted: ParsedSct = scts
            .into_iter()
            .next()
            .ok_or("expected at least one real SCT")?;
        let last = corrupted.signature.len() - 1;
        corrupted.signature[last] ^= 0x01;

        let tbs_der = tbs
            .to_der()
            .map_err(|e| format!("failed to re-encode tbs: {e}"))?;
        let precert_tbs = precert_tbs_der(&tbs_der)?;
        let issuer_key_hash: [u8; 32] = Sha256::digest(&validated.issuer_spki_der).into();

        match verify_one_sct(&corrupted, &precert_tbs, issuer_key_hash, &trust_store) {
            Err(Error::Certificate(CertificateError::SctInvalid)) => Ok(()),
            other => Err(format!("expected SctInvalid, got {other:?}").into()),
        }
    }

    #[test]
    fn sct_timestamp_outside_synthetic_ctlog_valid_for_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let validated = real_validated_leaf()?;
        // The real SCT's log is `ctlogs[1]` (`https://ctfe.sigstore.dev/2022`,
        // `validFor.start: "2022-10-20T00:00:00Z"`, no end) -- confirmed
        // empirically. Cap its `validFor.end` before the real SCT
        // timestamp so the log key itself is otherwise untouched (the
        // signature still verifies; only the time window rejects it).
        let trust_store = mutate_public_good_json(|v| {
            v["ctlogs"][1]["publicKey"]["validFor"]["end"] =
                serde_json::Value::String("2023-01-01T00:00:00Z".to_owned());
            Ok(())
        })?;
        match verify_embedded_scts(&validated, &trust_store) {
            Err(Error::Certificate(CertificateError::SctOutsideKeyValidity)) => Ok(()),
            other => Err(format!("expected SctOutsideKeyValidity, got {other:?}").into()),
        }
    }

    #[test]
    fn leaf_without_sct_extension_is_sct_missing() -> Result<(), Box<dyn std::error::Error>> {
        // Any certificate-authority certificate works here: none carry an
        // embedded SCT list, and `verify_embedded_scts` only reads
        // `validated.leaf` before it would need the (deliberately dummy)
        // SPKI fields below.
        let trust_store = embedded_trust_store()?;
        let root_der = trust_store
            .certificate_authorities
            .first()
            .and_then(|ca| ca.certificates.first())
            .ok_or("expected at least one trust-store certificate")?
            .raw_bytes
            .clone();
        let cert: x509_cert::Certificate = Decode::from_der(root_der.as_slice())
            .map_err(|e| format!("failed to parse trust-store certificate: {e}"))?;
        let validated = ValidatedLeaf {
            leaf: cert,
            leaf_spki_der: Vec::new(),
            issuer_spki_der: Vec::new(),
            chain_length: 1,
        };
        match verify_embedded_scts(&validated, &trust_store) {
            Err(Error::Certificate(CertificateError::SctMissing)) => Ok(()),
            other => Err(format!("expected SctMissing, got {other:?}").into()),
        }
    }
}
