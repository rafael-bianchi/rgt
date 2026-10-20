//! Shared strict-decoding and resource-limit helpers used by every parser
//! module (`bundle`, `statement`, `trust`, `subject`, `policy`).
//!
//! Centralizing these keeps the hardening rules (DESIGN.md "Core decisions"
//! item 2) consistent across formats instead of re-implemented per module.

use crate::error::{ParseError, ResourceLimitError};
use crate::limits;

/// Decodes `s` as strict standard-alphabet, padded base64 (the `base64`
/// crate's default `STANDARD` engine: canonical padding required, no
/// tolerance for stray trailing bits). Rejects anything else.
pub(crate) fn strict_base64(field: &'static str, s: &str) -> Result<Vec<u8>, ParseError> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(s).map_err(|e| ParseError::Base64 {
        field,
        reason: e.to_string(),
    })
}

/// Decodes `s` as exactly `N` bytes of strict hex (rejects wrong length and
/// non-hex characters in one check).
pub(crate) fn strict_hex<const N: usize>(
    field: &'static str,
    s: &str,
) -> Result<[u8; N], ParseError> {
    let mut out = [0u8; N];
    hex::decode_to_slice(s, &mut out).map_err(|e| ParseError::Hex {
        field,
        reason: e.to_string(),
    })?;
    Ok(out)
}

/// Parses `s` as a strict base-10 `u64`, rejecting non-numeric strings
/// (including signs, whitespace, and leading zeros' odder cousins like
/// `"+5"`). Used for the protojson `int64`/`uint64` fields that arrive as
/// JSON strings (`logIndex`, `integratedTime`, `treeSize`).
pub(crate) fn strict_stringified_u64(field: &'static str, s: &str) -> Result<u64, ParseError> {
    // `str::parse::<u64>` already rejects signs, whitespace, and empty
    // strings; it only accepts an optional-free run of ASCII digits.
    s.parse::<u64>().map_err(|_| ParseError::NotAnInteger {
        field,
        value: s.to_owned(),
    })
}

/// Decodes `s` as a single PEM block labeled `label` (e.g. `"CERTIFICATE"`
/// for `-----BEGIN CERTIFICATE-----` / `-----END CERTIFICATE-----`),
/// returning the decoded DER bytes.
///
/// Strict: `s` (after trimming only leading/trailing whitespace) must
/// start with the exact begin marker and end with the exact end marker;
/// everything between them is treated as the base64 body (PEM line-wraps
/// it, so internal whitespace is stripped before decoding with the same
/// strict engine as [`strict_base64`]).
pub(crate) fn strict_pem(field: &'static str, label: &str, s: &str) -> Result<Vec<u8>, ParseError> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let trimmed = s.trim();
    let body = trimmed
        .strip_prefix(begin.as_str())
        .ok_or_else(|| ParseError::Base64 {
            field,
            reason: format!("missing \"{begin}\" marker"),
        })?
        .strip_suffix(end.as_str())
        .ok_or_else(|| ParseError::Base64 {
            field,
            reason: format!("missing \"{end}\" marker"),
        })?;
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    strict_base64(field, &compact)
}

/// Rejects `bytes` if it exceeds the crate-wide maximum input size. Must be
/// the first check performed by every public entry point that takes raw
/// caller-supplied bytes -- `from_json`, `from_json_lines`,
/// `from_github_response`, and `Statement::from_payload` -- before any
/// parsing work.
///
/// The rule is "every byte-taking public constructor", not a fixed list of
/// names: `Statement::from_payload` was originally omitted here precisely
/// because it did not match the `from_json*` naming, and stayed unbounded
/// as a result.
pub(crate) fn check_input_size(bytes: &[u8]) -> Result<(), ResourceLimitError> {
    if bytes.len() > limits::MAX_INPUT_BYTES {
        Err(ResourceLimitError::InputTooLarge {
            actual: bytes.len(),
            limit: limits::MAX_INPUT_BYTES,
        })
    } else {
        Ok(())
    }
}

/// Rejects `items` if its length exceeds `limit`, using `make_err` to
/// build the specific [`ResourceLimitError`] variant for the collection in
/// question.
pub(crate) fn check_count<T>(
    items: &[T],
    limit: usize,
    make_err: impl FnOnce(usize, usize) -> ResourceLimitError,
) -> Result<(), ResourceLimitError> {
    if items.len() > limit {
        Err(make_err(items.len(), limit))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{check_count, check_input_size, strict_base64, strict_hex, strict_stringified_u64};
    use crate::error::{ParseError, ResourceLimitError};

    #[test]
    fn strict_base64_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let decoded = strict_base64("field", "aGVsbG8=")?;
        if decoded != b"hello" {
            return Err("base64 decode mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn strict_base64_rejects_missing_padding() -> Result<(), Box<dyn std::error::Error>> {
        match strict_base64("field", "aGVsbG8") {
            Err(ParseError::Base64 { .. }) => Ok(()),
            other => Err(format!("expected Base64 error, got {other:?}").into()),
        }
    }

    #[test]
    fn strict_base64_rejects_trailing_bits() -> Result<(), Box<dyn std::error::Error>> {
        // "/w==" is the canonical single-byte encoding of 0xFF (the last 4
        // bits, unused by a single byte, are zero). "/x==" uses the same
        // alphabet and padding length but sets those unused bits to a
        // nonzero value while decoding to the same byte; a strict engine
        // (decode_allow_trailing_bits = false) must reject it.
        match strict_base64("field", "/x==") {
            Err(ParseError::Base64 { .. }) => Ok(()),
            other => Err(format!("expected Base64 error, got {other:?}").into()),
        }
    }

    #[test]
    fn strict_hex_round_trips() -> Result<(), Box<dyn std::error::Error>> {
        let out: [u8; 2] = strict_hex("field", "0aff")?;
        if out != [0x0a, 0xff] {
            return Err("hex decode mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn strict_hex_rejects_wrong_length() -> Result<(), Box<dyn std::error::Error>> {
        match strict_hex::<32>("field", "abcd") {
            Err(ParseError::Hex { .. }) => Ok(()),
            other => Err(format!("expected Hex error, got {other:?}").into()),
        }
    }

    #[test]
    fn strict_hex_rejects_non_hex_chars() -> Result<(), Box<dyn std::error::Error>> {
        match strict_hex::<2>("field", "zz") {
            Err(ParseError::Hex { .. }) => Ok(()),
            other => Err(format!("expected Hex error, got {other:?}").into()),
        }
    }

    #[test]
    fn strict_stringified_u64_parses_digits() -> Result<(), Box<dyn std::error::Error>> {
        let value = strict_stringified_u64("field", "2049189324")?;
        if value != 2_049_189_324 {
            return Err("u64 parse mismatch".into());
        }
        Ok(())
    }

    #[test]
    fn strict_stringified_u64_rejects_non_numeric() -> Result<(), Box<dyn std::error::Error>> {
        match strict_stringified_u64("field", "12abc") {
            Err(ParseError::NotAnInteger { .. }) => Ok(()),
            other => Err(format!("expected NotAnInteger error, got {other:?}").into()),
        }
    }

    #[test]
    fn strict_stringified_u64_rejects_negative() -> Result<(), Box<dyn std::error::Error>> {
        match strict_stringified_u64("field", "-5") {
            Err(ParseError::NotAnInteger { .. }) => Ok(()),
            other => Err(format!("expected NotAnInteger error, got {other:?}").into()),
        }
    }

    #[test]
    fn check_input_size_allows_small_input() -> Result<(), Box<dyn std::error::Error>> {
        check_input_size(b"small")?;
        Ok(())
    }

    #[test]
    fn check_input_size_rejects_oversized_input() -> Result<(), Box<dyn std::error::Error>> {
        let big = vec![0u8; crate::limits::MAX_INPUT_BYTES + 1];
        match check_input_size(&big) {
            Err(ResourceLimitError::InputTooLarge { .. }) => Ok(()),
            other => Err(format!("expected InputTooLarge error, got {other:?}").into()),
        }
    }

    #[test]
    fn check_count_allows_within_limit() -> Result<(), Box<dyn std::error::Error>> {
        let items = [1, 2, 3];
        check_count(&items, 3, |actual, limit| {
            ResourceLimitError::TooManyBundles { actual, limit }
        })?;
        Ok(())
    }

    #[test]
    fn check_count_rejects_over_limit() -> Result<(), Box<dyn std::error::Error>> {
        let items = [1, 2, 3, 4];
        match check_count(&items, 3, |actual, limit| {
            ResourceLimitError::TooManyBundles { actual, limit }
        }) {
            Err(ResourceLimitError::TooManyBundles {
                actual: 4,
                limit: 3,
            }) => Ok(()),
            other => Err(format!("expected TooManyBundles error, got {other:?}").into()),
        }
    }
}
