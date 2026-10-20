//! In-toto Statement v1 parsing (the decoded `dsseEnvelope.payload`).
//!
//! Multi-subject statements are the normal case, not an edge case: a
//! GitHub workflow provenance statement typically lists every artifact
//! produced by a release as a subject (DESIGN.md "Fixture findings").
//! Matching a caller's digest means finding it in that set.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::{Error, ParseError, ResourceLimitError, UnsupportedError};
use crate::limits;
use crate::parse_util;
use crate::strict_json;
use crate::subject::Subject;

/// The only in-toto statement `_type` this crate understands.
pub const STATEMENT_TYPE: &str = "https://in-toto.io/Statement/v1";

/// A parsed and structurally-hardened in-toto Statement.
///
/// The predicate is kept as an opaque [`serde_json::Value`]: its shape
/// depends on `predicate_type` (SLSA provenance, GitHub's release
/// predicate, ...) and interpreting it is out of scope for this crate's
/// parsing layer.
#[derive(Debug, Clone)]
pub struct Statement {
    /// Every subject the statement makes claims about. Usable subjects
    /// (those with a well-formed `sha256` entry) are `Some`; subjects
    /// whose digest map has other algorithms but no `sha256` key are
    /// still kept, with `sha256: None`.
    pub subjects: Vec<StatementSubject>,
    /// The predicate's type URI, e.g.
    /// `"https://slsa.dev/provenance/v1"`.
    pub predicate_type: String,
    /// The predicate body, opaque to this crate.
    pub predicate: serde_json::Value,
}

impl Statement {
    /// Parses `bytes` (an already base64-decoded DSSE payload) as an
    /// in-toto Statement.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitError::InputTooLarge`] if `bytes` exceeds the
    /// crate-wide input limit, [`ParseError::Json`] for malformed JSON
    /// (including duplicate object keys), [`UnsupportedError::StatementType`]
    /// if `_type` is not [`STATEMENT_TYPE`],
    /// [`ResourceLimitError::TooManySubjects`] if the subject count exceeds
    /// the crate's limit, and
    /// [`ParseError::MalformedSubject`] if any subject's digest map is
    /// empty or has a malformed `sha256` value.
    pub fn from_payload(bytes: &[u8]) -> Result<Self, Error> {
        parse_util::check_input_size(bytes)?;
        let value = strict_json::parse_strict(bytes)?;
        let raw: RawStatement =
            serde_json::from_value(value).map_err(|e| ParseError::Json(e.to_string()))?;
        Self::from_raw(raw)
    }

    fn from_raw(raw: RawStatement) -> Result<Self, Error> {
        let RawStatement {
            type_,
            subject,
            predicate_type,
            predicate,
        } = raw;
        if type_ != STATEMENT_TYPE {
            return Err(Error::Unsupported(UnsupportedError::StatementType {
                found: type_,
            }));
        }
        parse_util::check_count(&subject, limits::MAX_STATEMENT_SUBJECTS, |actual, limit| {
            ResourceLimitError::TooManySubjects { actual, limit }
        })?;
        let subjects = subject
            .into_iter()
            .enumerate()
            .map(|(index, raw_subject)| StatementSubject::from_raw(index, raw_subject))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Statement {
            subjects,
            predicate_type,
            predicate,
        })
    }

    /// Returns `true` if `subject`'s digest matches the `sha256` of any
    /// subject in this statement.
    ///
    /// This is a pure data lookup over signed-but-unverified content: it
    /// says the statement *claims* this digest as a subject, nothing
    /// about whether that claim has been cryptographically verified. See
    /// [`crate::Verifier`] for verification.
    #[must_use]
    pub fn contains_subject(&self, subject: &Subject) -> bool {
        self.find_subject(subject).is_some()
    }

    /// Returns the first statement subject whose digest matches `subject`,
    /// if any.
    ///
    /// Like [`Statement::contains_subject`], this is a pure data lookup
    /// over signed-but-unverified content — it says the statement
    /// *claims* this digest as a subject (and, if the claim carries one,
    /// under this name), nothing about whether that claim has been
    /// cryptographically verified. See [`crate::Verifier`] for
    /// verification.
    #[must_use]
    pub fn find_subject(&self, subject: &Subject) -> Option<&StatementSubject> {
        self.subjects
            .iter()
            .find(|s| s.sha256.as_ref() == Some(subject))
    }
}

/// One statement subject (`subject[]`).
#[derive(Debug, Clone)]
pub struct StatementSubject {
    /// The subject's name/filename, if given.
    pub name: Option<String>,
    /// The subject's sha256 digest, if its digest map has a well-formed
    /// one. `None` when the digest map uses only other algorithms.
    pub sha256: Option<Subject>,
}

impl StatementSubject {
    fn from_raw(index: usize, raw: RawStatementSubject) -> Result<Self, Error> {
        if raw.digest.is_empty() {
            return Err(Error::Parse(ParseError::MalformedSubject(format!(
                "subject[{index}] has an empty digest map"
            ))));
        }
        let sha256 = match raw.digest.get("sha256") {
            Some(hex) => {
                let subject = Subject::from_digest_hex(hex).map_err(|_| {
                    Error::Parse(ParseError::MalformedSubject(format!(
                        "subject[{index}] has a malformed sha256 digest"
                    )))
                })?;
                Some(subject)
            }
            None => None,
        };
        Ok(StatementSubject {
            name: raw.name,
            sha256,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStatement {
    #[serde(rename = "_type")]
    type_: String,
    #[serde(default)]
    subject: Vec<RawStatementSubject>,
    predicate_type: String,
    #[serde(default)]
    predicate: serde_json::Value,
}

#[derive(Deserialize)]
struct RawStatementSubject {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    digest: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::{STATEMENT_TYPE, Statement};
    use crate::error::{Error, ParseError, ResourceLimitError, UnsupportedError};
    use crate::limits;
    use crate::subject::Subject;

    fn valid_statement_json(subjects: &str) -> String {
        format!(
            r#"{{"_type":"{STATEMENT_TYPE}","subject":[{subjects}],"predicateType":"https://example.test/predicate","predicate":{{}}}}"#
        )
    }

    #[test]
    fn parses_minimal_valid_statement() -> Result<(), Box<dyn std::error::Error>> {
        let json = valid_statement_json(
            r#"{"name":"a.txt","digest":{"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
        );
        let statement = Statement::from_payload(json.as_bytes())?;
        if statement.subjects.len() != 1 {
            return Err("expected exactly one subject".into());
        }
        let expected = Subject::from_digest_hex(&"a".repeat(64))?;
        if !statement.contains_subject(&expected) {
            return Err("expected subject to be found".into());
        }
        Ok(())
    }

    #[test]
    fn subject_without_sha256_is_kept_but_not_matchable() -> Result<(), Box<dyn std::error::Error>>
    {
        let json = valid_statement_json(r#"{"name":"a.txt","digest":{"sha512":"deadbeef"}}"#);
        let statement = Statement::from_payload(json.as_bytes())?;
        if statement.subjects.len() != 1 {
            return Err("expected exactly one subject".into());
        }
        if statement.subjects[0].sha256.is_some() {
            return Err("expected no sha256 for a sha512-only digest map".into());
        }
        Ok(())
    }

    #[test]
    fn rejects_empty_digest_map() -> Result<(), Box<dyn std::error::Error>> {
        let json = valid_statement_json(r#"{"name":"a.txt","digest":{}}"#);
        match Statement::from_payload(json.as_bytes()) {
            Err(Error::Parse(ParseError::MalformedSubject(_))) => Ok(()),
            other => Err(format!("expected MalformedSubject error, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_malformed_sha256_value() -> Result<(), Box<dyn std::error::Error>> {
        let json = valid_statement_json(r#"{"digest":{"sha256":"tooshort"}}"#);
        match Statement::from_payload(json.as_bytes()) {
            Err(Error::Parse(ParseError::MalformedSubject(_))) => Ok(()),
            other => Err(format!("expected MalformedSubject error, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_wrong_statement_type() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{"_type":"https://example.test/not-a-statement","subject":[],"predicateType":"x","predicate":{}}"#;
        match Statement::from_payload(json.as_bytes()) {
            Err(Error::Unsupported(UnsupportedError::StatementType { .. })) => Ok(()),
            other => Err(format!("expected StatementType error, got {other:?}").into()),
        }
    }

    #[test]
    fn rejects_oversized_payload_before_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let payload = vec![b' '; limits::MAX_INPUT_BYTES + 1];
        match Statement::from_payload(&payload) {
            Err(Error::ResourceLimit(ResourceLimitError::InputTooLarge { actual, limit }))
                if actual == payload.len() && limit == limits::MAX_INPUT_BYTES =>
            {
                Ok(())
            }
            other => Err(format!("expected InputTooLarge error, got {other:?}").into()),
        }
    }
}
