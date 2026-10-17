mod date_diff;
mod duration;
mod expression;

pub use date_diff::compute_date_diff_seconds;
pub use duration::{compute_duration_aggregate, validate_duration_parent_ids};
pub use expression::{temporal_coercion_warning, validate_expression};

use crate::types::{DurationClaim, DurationUnit, ValueData};
use std::process;

/// The only valid `--operation` values (case-sensitive).
pub const VALID_OPERATIONS: [&str; 4] = ["EXPRESSION", "DATE_DIFF", "DURATION_SUM", "DURATION_AVG"];

/// Maximum number of `--parents` (variables `a`–`z`). Exceeding it is invalid
/// input (FR-005; `contracts/verify-behavior.md`).
pub const MAX_PARENTS: usize = 26;

/// Whether `op` is a valid `--operation` value (FR-002): case-sensitive
/// allow-list. Casing slips (`expression`, `Expression`, `date_diff`) are
/// invalid input, never silently skipped.
pub fn valid_operation(op: &str) -> bool {
    VALID_OPERATIONS.contains(&op)
}

/// Verifies that a derived value matches what would be computed from parent nodes.
///
/// Returns `Ok(())` if the result matches.
/// Returns `Err(message)` if the result does not match (verification failed)
/// or the operation is not in the supported operation set.
///
/// This floating-point API supports `EXPRESSION` only. Use
/// [`verify_temporal_claim`] for `DATE_DIFF`, `DURATION_SUM`, and
/// `DURATION_AVG` so temporal claims retain exact decimal precision.
pub fn verify(
    parents: &[ValueData],
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<(), String> {
    if !valid_operation(operation) {
        return Err(format!(
            "unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG",
            operation
        ));
    }
    match operation {
        "EXPRESSION" => {
            let expr = expression
                .ok_or_else(|| "Missing --expression for EXPRESSION operation".to_string())?;
            expression::verify_expression(parents, expr, result)
        }
        "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG" => Err(format!(
            "{operation} requires the exact temporal claim API; call verify_temporal_claim with DurationClaim instead of f64"
        )),
        _ => unreachable!("valid_operation rejected above"),
    }
}

/// Verifies a temporal result from an exact decimal claim and fixed unit.
///
/// Unlike [`verify`], this API does not pass through `f64`, so fractional or
/// precision-rounded claims cannot be truncated into a valid whole-second
/// result. `EXPRESSION` remains available through [`verify`] because it has
/// its established floating-point Number semantics.
pub fn verify_temporal_claim(
    parents: &[ValueData],
    operation: &str,
    claim: &DurationClaim,
) -> Result<(), String> {
    if !valid_operation(operation) {
        return Err(format!(
            "unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG",
            operation
        ));
    }
    match operation {
        "DATE_DIFF" => date_diff::verify_date_diff(parents, claim),
        "DURATION_SUM" | "DURATION_AVG" => {
            let expected_seconds = duration::compute_duration_aggregate(operation, parents)?;
            claim.verify(expected_seconds)
        }
        "EXPRESSION" => Err("verify_temporal_claim does not support EXPRESSION".into()),
        _ => unreachable!("valid_operation rejected above"),
    }
}

pub mod error;
pub use error::VerifyDeriveError;

/// Core verification logic shared by the CLI and MCP server mode.
/// Returns a typed error separating invalid input (CLI exit 2) from verification
/// failures (CLI exit 1) without calling `process::exit`.
pub fn run_verify_core(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: &str,
    result_unit: Option<&str>,
) -> Result<(), VerifyDeriveError> {
    if !valid_operation(operation) {
        return Err(VerifyDeriveError::InvalidInput(format!(
            "unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG",
            operation
        )));
    }
    if operation == "EXPRESSION" {
        if result_unit.is_some() {
            return Err(VerifyDeriveError::InvalidInput(
                "--result-unit is not valid for EXPRESSION".into(),
            ));
        }
        if let Some(expr) = expression {
            if let Err(e) = expression::validate_expression(expr) {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "invalid --expression: {}",
                    e
                )));
            }
        } else {
            return Err(VerifyDeriveError::InvalidInput(
                "Missing --expression for EXPRESSION operation".into(),
            ));
        }
    }

    let claim = if matches!(operation, "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG") {
        let unit = DurationUnit::parse(result_unit.unwrap_or("seconds"))
            .map_err(VerifyDeriveError::InvalidInput)?;
        Some(DurationClaim::parse(result, unit).map_err(VerifyDeriveError::InvalidInput)?)
    } else {
        None
    };

    let parent_ids: Vec<String> = parents
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if parent_ids.is_empty() {
        return Err(VerifyDeriveError::InvalidInput(
            "--parents must contain at least one node ID".into(),
        ));
    }
    if parent_ids.len() > MAX_PARENTS {
        return Err(VerifyDeriveError::InvalidInput(format!(
            "at most {MAX_PARENTS} parents supported (variables a-z), got {}",
            parent_ids.len()
        )));
    }
    if operation == "DATE_DIFF" && parent_ids.len() != 2 {
        return Err(VerifyDeriveError::InvalidInput(format!(
            "DATE_DIFF requires exactly 2 Date parents, got {}",
            parent_ids.len()
        )));
    }
    if matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        validate_duration_parent_ids(operation, &parent_ids)
            .map_err(VerifyDeriveError::InvalidInput)?;
    }

    let db = crate::store::DbStore::open_in_project(".")
        .map_err(|e| VerifyDeriveError::Internal(format!("failed to open database: {}", e)))?;
    crate::graph::invalidation::sync_staleness(&db)
        .map_err(|e| VerifyDeriveError::Internal(format!("staleness check failed: {}", e)))?;
    let conn = db.conn();

    let mut parent_values = Vec::new();
    for pid in &parent_ids {
        match crate::store::queries::get_tracked_node(conn, pid) {
            Ok(Some(node)) => parent_values.push(node.value),
            Ok(None) => {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "parent node '{}' not found in database",
                    pid
                )));
            }
            Err(e) => {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "failed to load parent node '{}': {}",
                    pid, e
                )));
            }
        }
    }

    if operation == "EXPRESSION" {
        if let Some(warning) = temporal_coercion_warning(&parent_values) {
            eprintln!("{warning}");
        }
    }

    match operation {
        "EXPRESSION" => {
            let parsed = result.parse::<f64>().map_err(|_| {
                VerifyDeriveError::InvalidInput("--result for EXPRESSION must be numeric".into())
            })?;
            if !parsed.is_finite() {
                return Err(VerifyDeriveError::InvalidInput(
                    "--result for EXPRESSION must be finite".into(),
                ));
            }
            verify(&parent_values, operation, expression, parsed)
                .map_err(VerifyDeriveError::VerificationFailed)
        }
        "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG" => verify_temporal_claim(
            &parent_values,
            operation,
            claim.as_ref().expect("temporal claim parsed"),
        )
        .map_err(VerifyDeriveError::VerificationFailed),
        _ => unreachable!("valid_operation rejected unsupported operation"),
    }
}

/// Convenience verify function for numeric/MCP callers.
pub fn run_verify(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<(), VerifyDeriveError> {
    run_verify_core(parents, operation, expression, &result.to_string(), None)
}

/// CLI entry point for `rgt verify`. Handles DB access, error formatting, and exit codes.
pub fn run_verify_cli(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: &str,
    result_unit: Option<&str>,
) {
    if let Err(e) = run_verify_core(parents, operation, expression, result, result_unit) {
        e.print_cli(operation, expression);
        process::exit(e.exit_code());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    // ---- T003: valid_operation ----

    #[test]
    fn valid_operation_accepts_canonical() {
        assert!(valid_operation("EXPRESSION"));
        assert!(valid_operation("DATE_DIFF"));
    }

    #[test]
    fn valid_operation_rejects_case_variants_and_unknown() {
        for op in [
            "expression",
            "Expression",
            "date_diff",
            "",
            "RANDOM",
            "verify",
        ] {
            assert!(!valid_operation(op), "{} must be rejected", op);
        }
    }

    // ---- T008: verify() rejects unknown operations ----

    #[test]
    fn verify_rejects_unknown_operation() {
        let err = verify(&[], "expression", None, 0.0).unwrap_err();
        assert!(err.contains("unknown operation"), "{}", err);
        assert!(
            err.contains("EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG"),
            "{}",
            err
        );
    }

    #[test]
    fn verify_canonical_operation_is_not_an_unknown_operation_error() {
        // EXPRESSION with a missing --expression is a different error, not an
        // unknown-operation error.
        let err = verify(&[], "EXPRESSION", None, 1.0).unwrap_err();
        assert!(!err.contains("unknown operation"), "{}", err);
    }

    // ---- DATE_DIFF still works end to end ----

    #[test]
    fn verify_date_diff_via_verify() {
        let d1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        let error = verify(&parents, "DATE_DIFF", None, 864_000.0).unwrap_err();
        assert!(error.contains("verify_temporal_claim"), "{error}");
        let claim = DurationClaim::parse("10", DurationUnit::Days).unwrap();
        assert!(verify_temporal_claim(&parents, "DATE_DIFF", &claim).is_ok());
    }
}
