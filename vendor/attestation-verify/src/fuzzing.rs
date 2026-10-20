//! Feature-gated adapters for the repository's separate cargo-fuzz crate.
//!
//! These functions deliberately expose only `Result`-returning, parser-level
//! entry points. They are hidden from normal generated documentation and are
//! not compiled unless the `fuzzing` feature is selected; the default library
//! API and dependency graph are unchanged.
//!
//! The checkpoint-signature adapter intentionally exercises the internal
//! cryptographic path without a public [`Verifier`] policy: fuzzing needs to
//! reach signature parsing and verification for arbitrary envelopes, while
//! every normal verification still requires `CheckpointOriginPolicy`.

use crate::error::Error;

/// SHA-256 log ID of the Rekor v1 key that signed the committed checkpoint
/// fuzz seeds.
const CHECKPOINT_CORPUS_LOG_ID: [u8; 32] = [
    0xc0, 0xd2, 0x3d, 0x6a, 0xd4, 0x06, 0x97, 0x3f, 0x95, 0x59, 0xf3, 0xba, 0x2d, 0x1c, 0xa0, 0x1f,
    0x84, 0x14, 0x7d, 0x8f, 0xfc, 0x5b, 0x84, 0x45, 0xc2, 0x24, 0xf9, 0x8b, 0x95, 0x91, 0x80, 0x1d,
];

/// Parses one decoded Rekor canonicalized body without entering verification.
///
/// This adapter lets the external fuzz crate exercise the parser before
/// cryptographic binding checks reject a mutated body.
pub fn parse_rekor_canonicalized_body(bytes: &[u8]) -> Result<(), Error> {
    crate::parse_util::check_input_size(bytes)?;
    crate::rekor::parse_tlog_entry_body(bytes).map(|_| ())
}

/// Parses one Rekor signed-note/checkpoint envelope.
pub fn parse_checkpoint(envelope: &str) -> Result<(), Error> {
    crate::parse_util::check_input_size(envelope.as_bytes())?;
    crate::rekor::parse_checkpoint(envelope).map(|_| ())
}

/// Verifies one Rekor signed-note/checkpoint envelope against the embedded
/// public-good root's Rekor v1 log key.
///
/// The one adapter here that goes past parsing. It exists because the
/// checkpoint signature block is the only place in this crate where the
/// *amount* of cryptographic work is chosen by the input: the key-hint
/// filter, the `MAX_CHECKPOINT_SIGNATURES` bound, and the ECDSA loop all
/// live after `parse_checkpoint` returns, so a parser-only target cannot
/// reach them. Running them under libFuzzer puts that loop under the
/// `-timeout` and `-rss_limit_mb` budgets.
///
/// The envelope is anchored to its own tree size and root hash — see
/// [`crate::rekor::verify_checkpoint_self_anchored`] for why.
pub fn verify_checkpoint(envelope: &str) -> Result<(), Error> {
    crate::parse_util::check_input_size(envelope.as_bytes())?;
    let (log, log_key) = embedded_rekor_v1_log()?;
    crate::rekor::verify_checkpoint_self_anchored(envelope, log, log_key)
}

/// A transparency log and the verifying key parsed from it.
type SelectedLog = (
    crate::trust::TransparencyLog,
    crate::dsse::EcdsaVerifyingKey,
);

/// The embedded public-good root's transparency log that signed the committed
/// checkpoint corpus, parsed once.
///
/// Cached because the alternative — re-parsing the trust root on every
/// fuzz iteration — would dominate the run and starve the code actually
/// under test. Selection uses the corpus key's SHA-256 log ID, so adding or
/// reordering other ECDSA log keys cannot silently stop the seeds from
/// reaching signature verification.
fn embedded_rekor_v1_log() -> Result<&'static SelectedLog, Error> {
    use std::sync::OnceLock;

    static LOG: OnceLock<Option<SelectedLog>> = OnceLock::new();

    LOG.get_or_init(|| {
        let trust_store = crate::trust::TrustStore::embedded_public_good().ok()?;
        let (log, key) =
            crate::rekor::select_log_key(&trust_store, &CHECKPOINT_CORPUS_LOG_ID).ok()?;
        Some((log.clone(), key))
    })
    .as_ref()
    .ok_or(Error::Transparency(
        crate::error::TransparencyError::UnknownLogKey,
    ))
}

/// Parses one TLS-encoded RFC 6962 `SignedCertificateTimestampList`.
pub fn parse_sct_list(bytes: &[u8]) -> Result<(), Error> {
    crate::parse_util::check_input_size(bytes)?;
    crate::sct::parse_sct_list(bytes).map(|_| ())
}

/// Parses one RFC 3339 timestamp using the same field parser as trusted-root
/// validity windows.
pub fn parse_rfc3339(timestamp: &str) -> Result<i64, Error> {
    crate::parse_util::check_input_size(timestamp.as_bytes())?;
    crate::time::parse_rfc3339("fuzzing.timestamp", timestamp).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    use super::{CHECKPOINT_CORPUS_LOG_ID, embedded_rekor_v1_log};

    #[test]
    fn selected_log_key_hint_matches_committed_checkpoint_seed()
    -> Result<(), Box<dyn std::error::Error>> {
        const SEED: &str = include_str!("../fuzz/corpus/checkpoint_verify/golden-envelope.txt");

        let encoded_signature = SEED
            .lines()
            .find_map(|line| line.strip_prefix("— rekor.sigstore.dev "))
            .ok_or("checkpoint seed has no Rekor signature")?;
        let signature = STANDARD.decode(encoded_signature)?;
        let seed_key_hint = signature
            .get(..4)
            .ok_or("checkpoint seed signature has no key hint")?;

        let (log, _) = embedded_rekor_v1_log()?;
        if log.log_id_key_id != CHECKPOINT_CORPUS_LOG_ID {
            return Err("selected log label does not match the corpus log ID".into());
        }
        let selected_key_hint = crate::rekor::checkpoint_key_hint(log)
            .ok_or("selected log has no checkpoint key hint")?;
        if seed_key_hint != selected_key_hint {
            return Err("checkpoint seed key hint does not match the selected log".into());
        }

        Ok(())
    }
}
