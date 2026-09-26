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

/// CLI entry point for `rgt verify`. Handles DB access, error formatting, and exit codes.
pub fn run_verify_cli(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: &str,
    result_unit: Option<&str>,
) {
    // FR-002 / FR-001 / FR-005: validate operation, expression (for
    // EXPRESSION), and parent count up front — invalid input is exit 2, never
    // a silent pass-through.
    if !valid_operation(operation) {
        eprintln!(
            "Error: unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG",
            operation
        );
        process::exit(2);
    }
    if operation == "EXPRESSION" {
        if result_unit.is_some() {
            eprintln!("Error: --result-unit is not valid for EXPRESSION");
            process::exit(2);
        }
        if let Some(expr) = expression {
            if let Err(e) = expression::validate_expression(expr) {
                eprintln!("Error: invalid --expression: {}", e);
                process::exit(2);
            }
        } else {
            eprintln!("Error: Missing --expression for EXPRESSION operation");
            process::exit(2);
        }
    }

    let claim = if matches!(operation, "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG") {
        let unit = DurationUnit::parse(result_unit.unwrap_or("seconds"))
            .unwrap_or_else(|error| invalid_input(error));
        Some(DurationClaim::parse(result, unit).unwrap_or_else(|error| invalid_input(error)))
    } else {
        None
    };

    let parent_ids: Vec<String> = parents.split(',').map(|s| s.trim().to_string()).collect();

    if parent_ids.len() > MAX_PARENTS {
        invalid_input(format!(
            "at most {MAX_PARENTS} parents supported (variables a-z), got {}",
            parent_ids.len()
        ));
    }
    if operation == "DATE_DIFF" && parent_ids.len() != 2 {
        invalid_input(format!(
            "DATE_DIFF requires exactly 2 Date parents, got {}",
            parent_ids.len()
        ));
    }
    if matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        validate_duration_parent_ids(operation, &parent_ids)
            .unwrap_or_else(|error| invalid_input(error));
    }

    let db = match crate::store::DbStore::open_in_project(".") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error: failed to open database: {}", e);
            process::exit(2);
        }
    };
    let conn = db.conn();

    let mut parent_values = Vec::new();
    for pid in &parent_ids {
        match crate::store::queries::get_tracked_node(conn, pid) {
            Ok(Some(node)) => parent_values.push(node.value),
            Ok(None) => {
                eprintln!("Error: parent node '{}' not found in database", pid);
                process::exit(2);
            }
            Err(e) => {
                eprintln!("Error: failed to load parent node '{}': {}", pid, e);
                process::exit(2);
            }
        }
    }

    if operation == "EXPRESSION" {
        if let Some(warning) = temporal_coercion_warning(&parent_values) {
            eprintln!("{warning}");
        }
    }

    let verification = match operation {
        "EXPRESSION" => {
            let parsed = result.parse::<f64>().unwrap_or_else(|_| {
                invalid_input("--result for EXPRESSION must be numeric".into())
            });
            if !parsed.is_finite() {
                invalid_input("--result for EXPRESSION must be finite".into());
            }
            verify(&parent_values, operation, expression, parsed)
        }
        "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG" => verify_temporal_claim(
            &parent_values,
            operation,
            claim.as_ref().expect("temporal claim parsed"),
        ),
        _ => unreachable!("valid_operation rejected unsupported operation"),
    };

    match verification {
        Ok(()) => {
            process::exit(0);
        }
        Err(msg) => {
            eprintln!(
                "Error: verification failed for {} \"{}\"\n  {}\n  hint: retry with the correct result",
                operation,
                expression.unwrap_or(""),
                msg
            );
            process::exit(1);
        }
    }
}

fn invalid_input(message: String) -> ! {
    eprintln!("Error: {message}");
    process::exit(2);
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
