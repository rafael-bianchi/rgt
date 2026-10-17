use crate::types::ValueData;

/// Describes the legacy numeric coercion used by EXPRESSION for temporal inputs.
pub fn temporal_coercion_warning(parents: &[ValueData]) -> Option<String> {
    let has_date = parents
        .iter()
        .any(|value| matches!(value, ValueData::Date(_)));
    let has_duration = parents
        .iter()
        .any(|value| matches!(value, ValueData::Duration(_)));
    if !has_date && !has_duration {
        return None;
    }
    let mut details = Vec::new();
    if has_date {
        details.push("Date parents are interpreted as Unix timestamp seconds");
    }
    if has_duration {
        details.push("Duration parents are interpreted as elapsed seconds");
    }
    Some(format!(
        "Warning: legacy EXPRESSION coercion: {}; the expression result is a Number. Use DATE_DIFF or typed duration operations when you need Duration values.",
        details.join("; ")
    ))
}
use evalexpr::{Context, ContextWithMutableVariables};

/// Maximum `--expression` length in bytes. Exceeding it is invalid input
/// (FR-001; `contracts/verify-behavior.md`).
pub const MAX_EXPRESSION_LEN: usize = 4096;

/// Maximum parenthesis-nesting depth of an `--expression`. Exceeding it is
/// invalid input and, crucially, prevents evalexpr's recursive evaluator from
/// ever being handed an over-deep tree (no stack overflow — FR-001).
pub const MAX_EXPRESSION_DEPTH: usize = 256;

/// Variables available to an expression: `a`..`z` (FR-005).
const PARENT_VARIABLES: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// Rejects an `--expression` that exceeds the documented length or nesting
/// depth (or is empty), before any evalexpr parse/eval. Returns a clean,
/// user-recoverable error naming the violated limit.
pub fn validate_expression(expression: &str) -> Result<(), String> {
    if expression.trim().is_empty() {
        return Err("expression is empty".to_string());
    }
    if expression.len() > MAX_EXPRESSION_LEN {
        return Err(format!(
            "expression exceeds the maximum length of {} bytes (got {})",
            MAX_EXPRESSION_LEN,
            expression.len()
        ));
    }
    let mut depth: usize = 0;
    for ch in expression.chars() {
        match ch {
            '(' => {
                depth += 1;
                if depth > MAX_EXPRESSION_DEPTH {
                    return Err(format!(
                        "expression exceeds the maximum nesting depth of {} levels",
                        MAX_EXPRESSION_DEPTH
                    ));
                }
            }
            ')' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    Ok(())
}

fn approx_eq(a: f64, b: f64) -> bool {
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    if a == b {
        return true;
    }
    let largest = a.abs().max(b.abs());
    if largest > 1e-14 && a.is_sign_positive() != b.is_sign_positive() {
        return false;
    }
    let diff = (a - b).abs();
    diff <= (1e-9 * largest).max(f64::EPSILON * 8.0)
}

/// Checks that all variables for `parents` (`a`.. up to `parents.len()`) are referenced in `expression`.
fn validate_parent_variable_references(
    expression: &str,
    parent_count: usize,
) -> Result<(), String> {
    let bytes = expression.as_bytes();
    for i in 0..parent_count {
        let var = PARENT_VARIABLES[i] as char;
        let mut found = false;
        for (idx, &b) in bytes.iter().enumerate() {
            if b == var as u8 {
                let prev_ok = idx == 0
                    || (!bytes[idx - 1].is_ascii_alphanumeric() && bytes[idx - 1] != b'_');
                let next_ok = idx + 1 == bytes.len()
                    || (!bytes[idx + 1].is_ascii_alphanumeric() && bytes[idx + 1] != b'_');
                if prev_ok && next_ok {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            return Err(format!(
                "parent variable '{}' is not referenced in expression '{}'",
                var, expression
            ));
        }
    }
    Ok(())
}

/// Verifies that an arithmetic expression evaluated against parent values matches a claimed result.
///
/// # Arguments
/// - `parents`: Parent node values bound to variables `a`, `b`, `c`, ... in order (at most 26).
/// - `expression`: Formula string (e.g., `"(a + b) * c / 100"`). Supports `+`, `-`, `*`, `/`, `%`, `^`, parentheses.
/// - `result`: The caller-claimed numeric result.
///
/// # Returns
/// `Ok(computed_val)` if the evaluated expression matches `result` within float tolerance.
/// `Err(message)` on evaluation failure (unknown variable, division by zero) or mismatch.
pub fn verify_expression(
    parents: &[ValueData],
    expression: &str,
    result: f64,
) -> Result<f64, String> {
    // FR-001: reject over-limit expressions before any evaluation.
    validate_expression(expression)?;

    // FR-005: at most MAX_PARENTS (variables a-z); never rely on unchecked
    // `u8` arithmetic for the variable name.
    if parents.len() > super::MAX_PARENTS {
        return Err(format!(
            "at most {} parents supported (variables a-z), got {}",
            super::MAX_PARENTS,
            parents.len()
        ));
    }

    // Verify all parent variables are referenced in the expression
    validate_parent_variable_references(expression, parents.len())?;

    let mut context = evalexpr::HashMapContext::<evalexpr::DefaultNumericTypes>::new();

    for (i, parent) in parents.iter().enumerate() {
        let var_name = (PARENT_VARIABLES[i] as char).to_string();
        let val = match parent {
            ValueData::Number(n) => *n,
            ValueData::Duration(d) => d.num_seconds() as f64,
            ValueData::Date(d) => d.timestamp() as f64,
        };
        context
            .set_value(var_name, evalexpr::Value::Float(val))
            .map_err(|e| format!("Failed to bind variable: {}", e))?;
    }

    // FR-004: only bound variables and the documented arithmetic operators
    // (`+ - * / % ^`, parentheses) are usable. Builtin functions (`random`,
    // `str::*`, `math::*`, `if`, regex helpers, ...) are disabled so
    // evaluation is deterministic.
    context
        .set_builtin_functions_disabled(true)
        .map_err(|e| format!("Failed to restrict expression context: {}", e))?;

    let computed = evalexpr::eval_with_context(expression, &context)
        .map_err(|e| format!("Expression evaluation failed: {}", e))?;

    // FR-003: coerce integer-typed results to float (`as_number`), so pure
    // literal expressions like `2*50` verify against `100`.
    let computed_val: f64 = computed
        .as_number()
        .map_err(|_| "Expression did not evaluate to a number".to_string())?;

    if !computed_val.is_finite() {
        return Err(format!(
            "expression evaluated to non-finite value: {}",
            computed_val
        ));
    }

    if approx_eq(computed_val, result) {
        Ok(computed_val)
    } else {
        Err(format!(
            "{}: expected {}, got {}",
            expression, computed_val, result
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    // ---- T002: validate_expression limits (foundational) ----

    #[test]
    fn validate_expression_rejects_overlength() {
        let long = "1".repeat(MAX_EXPRESSION_LEN + 1);
        let err = validate_expression(&long).unwrap_err();
        assert!(err.contains("maximum length"), "{}", err);
    }

    #[test]
    fn validate_expression_rejects_overdepth() {
        let deep = format!(
            "{}1{}",
            "(".repeat(MAX_EXPRESSION_DEPTH + 1),
            ")".repeat(MAX_EXPRESSION_DEPTH + 1)
        );
        let err = validate_expression(&deep).unwrap_err();
        assert!(err.contains("nesting depth"), "{}", err);
    }

    #[test]
    fn validate_expression_accepts_at_limit() {
        let deep = format!(
            "{}1{}",
            "(".repeat(MAX_EXPRESSION_DEPTH),
            ")".repeat(MAX_EXPRESSION_DEPTH)
        );
        assert!(validate_expression(&deep).is_ok());
    }

    #[test]
    fn validate_expression_rejects_empty_and_whitespace() {
        assert!(validate_expression("").is_err());
        assert!(validate_expression("   \n\t").is_err());
    }

    // ---- T006: verify_expression never crashes on over-limit input ----

    #[test]
    fn verify_expression_deep_nesting_returns_clean_error() {
        // 1000 levels far exceeds MAX_EXPRESSION_DEPTH (256); must return a
        // clean Err (no stack overflow).
        let deep = format!("{}1{}", "(".repeat(1000), ")".repeat(1000));
        let err = verify_expression(&[], &deep, 1.0).unwrap_err();
        assert!(err.contains("nesting depth"), "{}", err);
    }

    #[test]
    fn verify_expression_overlength_returns_clean_error() {
        let long = format!("1 + {}", "0".repeat(MAX_EXPRESSION_LEN));
        let err = verify_expression(&[], &long, 1.0).unwrap_err();
        assert!(err.contains("maximum length"), "{}", err);
    }

    #[test]
    fn verify_expression_valid_inlimit_still_works() {
        let parents = vec![ValueData::Number(100.0), ValueData::Number(200.0)];
        assert!(verify_expression(&parents, "a + b", 300.0).is_ok());
    }

    // ---- T011: integer coercion + builtins disabled (US3) ----

    #[test]
    fn verify_expression_integer_literal_coerces() {
        assert_eq!(verify_expression(&[], "2*50", 100.0), Ok(100.0));
    }

    #[test]
    fn verify_expression_rejects_random_builtin() {
        let err = verify_expression(&[], "random()", 1.0).unwrap_err();
        assert!(
            !err.is_empty(),
            "random() must be rejected (builtins disabled)"
        );
    }

    #[test]
    fn verify_expression_rejects_str_builtin() {
        assert!(verify_expression(&[], "str::len(\"a\")", 1.0).is_err());
    }

    #[test]
    fn verify_expression_rejects_if_builtin() {
        assert!(verify_expression(&[], "if(a > b, 1, 2)", 1.0).is_err());
    }

    #[test]
    fn verify_expression_is_deterministic() {
        let parents = vec![ValueData::Number(100.0), ValueData::Number(200.0)];
        let a = verify_expression(&parents, "a + b", 300.0);
        let b = verify_expression(&parents, "a + b", 300.0);
        assert_eq!(a, b);
    }

    // ---- T013: parent cap (US4) ----

    #[test]
    fn verify_expression_rejects_more_than_26_parents() {
        let parents = (0..27).map(|_| ValueData::Number(1.0)).collect::<Vec<_>>();
        let err = verify_expression(&parents, "a", 1.0).unwrap_err();
        assert!(err.contains("at most 26 parents"), "{}", err);
    }

    #[test]
    fn verify_expression_accepts_exactly_26_parents() {
        let parents = (0..26).map(|_| ValueData::Number(1.0)).collect::<Vec<_>>();
        let expr = (0..26)
            .map(|i| (PARENT_VARIABLES[i] as char).to_string())
            .collect::<Vec<_>>()
            .join(" + ");
        assert!(verify_expression(&parents, &expr, 26.0).is_ok());
    }

    // ---- existing behavior ----

    #[test]
    fn test_expression_addition_match() {
        let parents = vec![ValueData::Number(100.0), ValueData::Number(200.0)];
        assert!(verify_expression(&parents, "a + b", 300.0).is_ok());
    }

    #[test]
    fn test_expression_multiplication_match() {
        let parents = vec![ValueData::Number(5.0), ValueData::Number(3.0)];
        assert!(verify_expression(&parents, "a * b", 15.0).is_ok());
    }

    #[test]
    fn test_expression_mismatch() {
        let parents = vec![ValueData::Number(100.0), ValueData::Number(200.0)];
        assert!(verify_expression(&parents, "a + b", 500.0).is_err());
    }

    #[test]
    fn test_expression_compound() {
        let parents = vec![
            ValueData::Number(300.0),
            ValueData::Number(150.0),
            ValueData::Number(2.0),
        ];
        assert!(verify_expression(&parents, "(a + b) * c / 100", 9.0).is_ok());
    }

    #[test]
    fn test_expression_float_tolerance() {
        let parents = vec![ValueData::Number(10.0), ValueData::Number(3.0)];
        assert!(verify_expression(&parents, "a / b", 3.33333333333333).is_ok());
    }

    #[test]
    fn test_expression_division_by_zero() {
        let parents = vec![ValueData::Number(10.0), ValueData::Number(0.0)];
        assert!(verify_expression(&parents, "a / b", 0.0).is_err());
    }

    #[test]
    fn test_expression_unknown_variable() {
        let parents = vec![ValueData::Number(10.0)];
        assert!(verify_expression(&parents, "a + d", 10.0).is_err());
    }

    #[test]
    fn test_expression_duration_parent() {
        let parents = vec![
            ValueData::Duration(Duration::seconds(1209600)),
            ValueData::Number(14.0),
        ];
        assert!(verify_expression(&parents, "a / b", 86400.0).is_ok());
    }

    // ---- Spec 047: T001 Unused parent variables ----

    #[test]
    fn test_expression_rejects_missing_parent_variable_references() {
        let parents = vec![ValueData::Number(10.0), ValueData::Number(20.0)];
        // 'b' is never used:
        let err = verify_expression(&parents, "a + 0", 10.0).unwrap_err();
        assert!(
            err.contains("parent variable 'b' is not referenced"),
            "expected error about unused parent 'b', got: {err}"
        );

        // Neither 'a' nor 'b' is used:
        let err2 = verify_expression(&parents, "42", 42.0).unwrap_err();
        assert!(
            err2.contains("parent variable 'a' is not referenced")
                || err2.contains("parent variable 'b' is not referenced"),
            "expected error about unused parent, got: {err2}"
        );
    }

    // ---- Spec 047: T002 Tolerance hardening ----

    #[test]
    fn test_approx_eq_rejects_small_number_false_positives() {
        // Must reject 0.0 vs 1e-10
        assert!(!approx_eq(1e-10, 0.0), "1e-10 must not match 0.0");
        assert!(!approx_eq(0.0, 1e-10), "0.0 must not match 1e-10");

        // Must reject opposite signs
        assert!(!approx_eq(1e-10, -1e-10), "opposite signs must not match");
        assert!(!approx_eq(1e-8, -1e-8), "opposite signs must not match");

        // Relative tolerance for small quantities
        assert!(approx_eq(1e-10, 1.0000000001e-10));
        assert!(!approx_eq(1e-10, 2e-10));

        // Sub-epsilon exact equality near zero
        assert!(approx_eq(0.0, 0.0));
    }
}
