mod date_diff;
mod expression;

use crate::types::ValueData;
use std::process;

/// Verifies that a derived value matches what would be computed from parent nodes.
///
/// Returns `Ok(())` if the result matches or the operation is unknown (pass-through).
/// Returns `Err(message)` if the result does not match (verification failed).
pub fn verify(
    parents: &[ValueData],
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<(), String> {
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
        _ => Ok(()), // unknown operation — pass through
    }
}

/// CLI entry point for `rgt verify`. Handles DB access, error formatting, and exit codes.
pub fn run_verify_cli(parents: &str, operation: &str, expression: Option<&str>, result: f64) {
    let parent_ids: Vec<String> = parents.split(',').map(|s| s.trim().to_string()).collect();

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

    let unknown = !matches!(operation, "EXPRESSION" | "DATE_DIFF");
    if unknown {
        eprintln!(
            "Warning: operation '{}' is not verifiable — skipping verification",
            operation
        );
        process::exit(0);
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
