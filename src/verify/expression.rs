use crate::types::ValueData;
use evalexpr::ContextWithMutableVariables;

fn approx_eq(a: f64, b: f64) -> bool {
    let abs = (a - b).abs();
    abs < 1e-9 || abs / b.abs().max(1e-10) < 1e-12
}

/// Verifies that an arithmetic expression evaluated against parent values matches a claimed result.
///
/// # Arguments
/// - `parents`: Parent node values bound to variables `a`, `b`, `c`, ... in order.
/// - `expression`: Formula string (e.g., `"(a + b) * c / 100"`). Supports `+`, `-`, `*`, `/`, `%`, `^`, parentheses.
/// - `result`: The caller-claimed numeric result.
///
/// # Returns
/// `Ok(())` if the evaluated expression matches `result` within float tolerance.
/// `Err(message)` on evaluation failure (unknown variable, division by zero) or mismatch.
pub fn verify_expression(
    parents: &[ValueData],
    expression: &str,
    result: f64,
) -> Result<(), String> {
    let mut context = evalexpr::HashMapContext::<evalexpr::DefaultNumericTypes>::new();

    for (i, parent) in parents.iter().enumerate() {
        let var_name = ((b'a' + i as u8) as char).to_string();
        let val = match parent {
            ValueData::Number(n) => *n,
            ValueData::Duration(d) => d.num_seconds() as f64,
            ValueData::Date(d) => d.timestamp() as f64,
        };
        context
            .set_value(var_name, evalexpr::Value::Float(val))
            .map_err(|e| format!("Failed to bind variable: {}", e))?;
    }

    let computed = evalexpr::eval_with_context(expression, &context)
        .map_err(|e| format!("Expression evaluation failed: {}", e))?;

    let computed_val: f64 = match computed.as_float() {
        Ok(v) => v,
        Err(_) => return Err("Expression did not evaluate to a number".to_string()),
    };

    if approx_eq(computed_val, result) {
        Ok(())
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
}
