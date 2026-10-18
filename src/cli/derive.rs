use crate::store::queries::{
    get_tracked_node, insert_derivation_edge, insert_or_reuse_duration, insert_tracked_node,
};
use crate::store::DbStore;
use crate::types::{
    DurationClaim, DurationDisplay, DurationUnit, NodeType, TrackedNode, ValueData,
};
use crate::verify::{
    compute_date_diff_seconds, compute_duration_aggregate, temporal_coercion_warning,
    valid_operation, validate_duration_parent_ids, validate_expression, verify, VerifyDeriveError,
    MAX_PARENTS,
};
use chrono::Duration;
use std::process;

/// Core derivation logic shared by the CLI and MCP server mode.
/// Returns a typed error separating invalid input (CLI exit 2) from verification
/// failures (CLI exit 1) without calling `process::exit`.
pub fn execute_derive_core(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: Option<&str>,
    result_unit: Option<&str>,
    unit: Option<&str>,
) -> Result<(String, Option<DurationDisplay>), VerifyDeriveError> {
    if !valid_operation(operation) {
        return Err(VerifyDeriveError::InvalidInput(format!(
            "unknown operation '{operation}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG"
        )));
    }
    if result_unit.is_some() && result.is_none() {
        return Err(VerifyDeriveError::InvalidInput(
            "--result-unit requires --result".into(),
        ));
    }
    let display_unit = match unit {
        Some(name) => Some(DurationUnit::parse(name).map_err(VerifyDeriveError::InvalidInput)?),
        None => None,
    };
    let claim = match (result, result_unit) {
        (Some(token), unit) if is_duration_operation(operation) => {
            let unit = DurationUnit::parse(unit.unwrap_or("seconds"))
                .map_err(VerifyDeriveError::InvalidInput)?;
            Some(DurationClaim::parse(token, unit).map_err(VerifyDeriveError::InvalidInput)?)
        }
        (Some(_), Some(_)) => {
            return Err(VerifyDeriveError::InvalidInput(
                "--result-unit is only valid for temporal operations".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(VerifyDeriveError::InvalidInput(
                "--result-unit requires --result".into(),
            ));
        }
        _ => None,
    };

    let expression_result = if operation == "EXPRESSION" {
        if display_unit.is_some() {
            return Err(VerifyDeriveError::InvalidInput(
                "--unit is only valid for Duration operations".into(),
            ));
        }
        if let Some(expression) = expression {
            validate_expression(expression).map_err(|error| {
                VerifyDeriveError::InvalidInput(format!("invalid --expression: {error}"))
            })?;
        } else {
            return Err(VerifyDeriveError::InvalidInput(
                "Missing --expression for EXPRESSION operation".into(),
            ));
        }
        let token = result.ok_or_else(|| {
            VerifyDeriveError::InvalidInput("Missing --result for EXPRESSION operation".into())
        })?;
        let parsed = token.parse::<f64>().map_err(|_| {
            VerifyDeriveError::InvalidInput("--result for EXPRESSION must be numeric".into())
        })?;
        if !parsed.is_finite() {
            return Err(VerifyDeriveError::InvalidInput(
                "--result for EXPRESSION must be finite".into(),
            ));
        }
        Some(parsed)
    } else {
        if expression.is_some() {
            return Err(VerifyDeriveError::InvalidInput(
                "--expression is only valid for EXPRESSION operation".into(),
            ));
        }
        None
    };

    let parent_ids = parents
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
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
    if operation == "EXPRESSION" {
        let mut seen = std::collections::HashSet::new();
        for id in &parent_ids {
            if !seen.insert(id) {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "duplicate parent ID '{id}' is not allowed in EXPRESSION derivations"
                )));
            }
        }
    }
    if matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        validate_duration_parent_ids(operation, &parent_ids)
            .map_err(VerifyDeriveError::InvalidInput)?;
    }

    let mut db = DbStore::open_in_project(".").map_err(|error| {
        VerifyDeriveError::Internal(format!("failed to open database: {error}"))
    })?;
    crate::graph::invalidation::sync_staleness(&mut db)
        .map_err(|error| VerifyDeriveError::Internal(format!("staleness check failed: {error}")))?;
    let conn = db.conn_mut();
    let mut parent_values = Vec::with_capacity(parent_ids.len());
    let mut any_parent_stale = false;
    for parent_id in &parent_ids {
        match get_tracked_node(conn, parent_id) {
            Ok(Some(node)) => {
                if node.is_stale {
                    any_parent_stale = true;
                }
                parent_values.push(node.value);
            }
            Ok(None) => {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "parent node '{parent_id}' not found in database"
                )))
            }
            Err(error) => {
                return Err(VerifyDeriveError::InvalidInput(format!(
                    "failed to load parent node '{parent_id}': {error}"
                )))
            }
        }
    }

    if operation == "EXPRESSION" {
        if let Some(warning) = temporal_coercion_warning(&parent_values) {
            eprintln!("{warning}");
        }
    }

    let value = match operation {
        "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG" => {
            let seconds = if operation == "DATE_DIFF" {
                compute_date_diff_seconds(&parent_values)
            } else {
                compute_duration_aggregate(operation, &parent_values)
            }
            .map_err(VerifyDeriveError::VerificationFailed)?;
            if let Some(claim) = &claim {
                claim
                    .verify(seconds)
                    .map_err(VerifyDeriveError::VerificationFailed)?;
            }
            ValueData::Duration(Duration::try_seconds(seconds).ok_or_else(|| {
                VerifyDeriveError::VerificationFailed(format!(
                    "computed seconds '{seconds}' are outside the supported Duration range"
                ))
            })?)
        }
        "EXPRESSION" => {
            let expected = expression_result.expect("validated above");
            let computed_val =
                verify(&parent_values, operation, expression, expected).map_err(|err| {
                    if err.contains(": expected ") {
                        VerifyDeriveError::VerificationFailed(err)
                    } else {
                        VerifyDeriveError::InvalidInput(err)
                    }
                })?;
            ValueData::Number(computed_val)
        }
        _ => unreachable!("valid_operation rejected unsupported operation"),
    };

    let now = chrono::Utc::now();
    let stale_reason = any_parent_stale.then(|| "PARENT_NODE_STALE".to_string());
    let node_id = if let ValueData::Duration(duration) = &value {
        let seconds = duration.num_seconds();
        let canonical_id =
            TrackedNode::generate_duration_derived_id(&parent_ids, operation, seconds)
                .map_err(VerifyDeriveError::InvalidInput)?;
        let node = TrackedNode {
            id: canonical_id,
            node_type: NodeType::Derived,
            value_kind: value.kind(),
            value: value.clone(),
            source_doc_id: None,
            line_number: None,
            is_stale: any_parent_stale,
            stale_reason: stale_reason.clone(),
            created_at: now,
            updated_at: now,
        };
        let legacy_candidates = if operation == "DATE_DIFF" {
            let legacy_id = TrackedNode::generate_derived_id(&parent_ids, operation, &value);
            let short_length = "node_drv_".len() + 12;
            vec![legacy_id.clone(), legacy_id[..short_length].to_string()]
        } else {
            Vec::new()
        };
        insert_or_reuse_duration(conn, &node, &parent_ids, operation, &legacy_candidates)
            .map_err(VerifyDeriveError::Internal)?
    } else {
        let node_id =
            TrackedNode::generate_derived_id_with_expr(&parent_ids, operation, &value, expression);
        let node = TrackedNode {
            id: node_id.clone(),
            node_type: NodeType::Derived,
            value_kind: value.kind(),
            value: value.clone(),
            source_doc_id: None,
            line_number: None,
            is_stale: any_parent_stale,
            stale_reason,
            created_at: now,
            updated_at: now,
        };
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|error| {
                VerifyDeriveError::Internal(format!(
                    "failed to begin derivation transaction: {error}"
                ))
            })?;
        let mut tx_parent_stale = false;
        for parent_id in &parent_ids {
            if let Ok(Some(p_node)) = get_tracked_node(&tx, parent_id) {
                if p_node.is_stale {
                    tx_parent_stale = true;
                }
            }
        }
        let final_stale = any_parent_stale || tx_parent_stale;
        let final_stale_reason = final_stale.then(|| "PARENT_NODE_STALE".to_string());
        let mut final_node = node;
        final_node.is_stale = final_stale;
        final_node.stale_reason = final_stale_reason;

        insert_tracked_node(&tx, &final_node).map_err(|error| {
            VerifyDeriveError::Internal(format!("failed to insert derived node: {error}"))
        })?;
        for parent_id in &parent_ids {
            insert_derivation_edge(&tx, parent_id, &node_id, operation, expression).map_err(
                |error| VerifyDeriveError::Internal(format!("failed to insert edge: {error}")),
            )?;
        }
        tx.commit().map_err(|error| {
            VerifyDeriveError::Internal(format!("failed to commit derivation transaction: {error}"))
        })?;
        node_id
    };

    let display = match (display_unit, value) {
        (Some(unit), ValueData::Duration(duration)) => Some(unit.display(duration.num_seconds())),
        _ => None,
    };
    Ok((node_id, display))
}

/// Convenience derive function for numeric/MCP callers.
pub fn execute_derive_mcp(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<String, VerifyDeriveError> {
    let result_str = result.to_string();
    let (node_id, _) = execute_derive_core(
        parents,
        operation,
        expression,
        Some(&result_str),
        None,
        None,
    )?;
    Ok(node_id)
}

/// CLI entry point for `rgt derive`.
pub fn execute_derive(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: Option<&str>,
    result_unit: Option<&str>,
    unit: Option<&str>,
) -> Result<(), String> {
    match execute_derive_core(parents, operation, expression, result, result_unit, unit) {
        Ok((node_id, display)) => {
            println!("Derived node: {node_id}");
            if let Some(display) = display {
                println!(
                    "Duration: {} ({} seconds)",
                    display.text, display.duration_seconds
                );
            }
            Ok(())
        }
        Err(e) => {
            e.print_cli(operation, expression);
            process::exit(e.exit_code());
        }
    }
}

fn is_duration_operation(operation: &str) -> bool {
    matches!(operation, "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG")
}
