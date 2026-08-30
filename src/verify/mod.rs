mod date_diff;
mod expression;

pub use expression::validate_expression;

use crate::types::ValueData;
use std::process;

/// The only valid `--operation` values (case-sensitive).
pub const VALID_OPERATIONS: [&str; 2] = ["EXPRESSION", "DATE_DIFF"];

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
/// or the operation is not in `{EXPRESSION, DATE_DIFF}`.
pub fn verify(
    parents: &[ValueData],
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<(), String> {
    if !valid_operation(operation) {
        return Err(format!(
            "unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF",
            operation
        ));
    }
    match operation {
        "EXPRESSION" => {
            let expr = expression
                .ok_or_else(|| "Missing --expression for EXPRESSION operation".to_string())?;
            expression::verify_expression(parents, expr, result)
        }
        "DATE_DIFF" => {
            let result_secs = result as i64;
            date_diff::verify_date_diff(parents, result_secs)
        }
        _ => unreachable!("valid_operation rejected above"),
    }
}

/// CLI entry point for `rgt verify`. Handles DB access, error formatting, and exit codes.
pub fn run_verify_cli(parents: &str, operation: &str, expression: Option<&str>, result: f64) {
    // FR-002 / FR-001 / FR-005: validate operation, expression (for
    // EXPRESSION), and parent count up front — invalid input is exit 2, never
    // a silent pass-through.
    if !valid_operation(operation) {
        eprintln!(
            "Error: unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF",
            operation
        );
        process::exit(2);
    }
    if operation == "EXPRESSION" {
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

    let parent_ids: Vec<String> = parents.split(',').map(|s| s.trim().to_string()).collect();

    if parent_ids.len() > MAX_PARENTS {
        eprintln!(
            "Error: at most {} parents supported (variables a-z), got {}",
            MAX_PARENTS,
            parent_ids.len()
        );
        process::exit(2);
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

    match verify(&parent_values, operation, expression, result) {
        Ok(()) => {
            process::exit(0);
        }
        Err(msg) => {
            let parts: Vec<&str> = msg.split(": ").collect();
            let detail = if parts.len() == 2 { parts[1] } else { &msg };
            eprintln!(
                "Error: verification failed for {} \"{}\"\n  {}\n  hint: retry with the correct result",
                operation,
                expression.unwrap_or(""),
                detail
            );
            process::exit(1);
        }
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
        assert!(err.contains("EXPRESSION, DATE_DIFF"), "{}", err);
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
        assert!(verify(&parents, "DATE_DIFF", None, 864_000.0).is_ok());
    }
}
