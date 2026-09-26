use crate::store::queries::{
    get_tracked_node, insert_derivation_edge, insert_or_reuse_duration, insert_tracked_node,
};
use crate::store::DbStore;
use crate::types::{DurationClaim, DurationUnit, NodeType, TrackedNode, ValueData};
use crate::verify::{
    compute_date_diff_seconds, compute_duration_aggregate, temporal_coercion_warning,
    valid_operation, validate_duration_parent_ids, validate_expression, verify, MAX_PARENTS,
};
use chrono::Duration;
use std::process;

pub fn execute_derive(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: Option<&str>,
    result_unit: Option<&str>,
    unit: Option<&str>,
) -> Result<(), String> {
    if !valid_operation(operation) {
        input_error(format!(
            "unknown operation '{operation}' — valid operations: EXPRESSION, DATE_DIFF, DURATION_SUM, DURATION_AVG"
        ));
    }
    if result_unit.is_some() && result.is_none() {
        input_error("--result-unit requires --result".into());
    }
    let display_unit =
        unit.map(|name| DurationUnit::parse(name).unwrap_or_else(|error| input_error(error)));
    let claim = match (result, result_unit) {
        (Some(token), unit) if is_duration_operation(operation) => {
            let unit = DurationUnit::parse(unit.unwrap_or("seconds"))
                .unwrap_or_else(|error| input_error(error));
            Some(DurationClaim::parse(token, unit).unwrap_or_else(|error| input_error(error)))
        }
        (Some(_), Some(_)) => {
            input_error("--result-unit is only valid for temporal operations".into())
        }
        (None, Some(_)) => input_error("--result-unit requires --result".into()),
        _ => None,
    };

    let expression_result = if operation == "EXPRESSION" {
        if display_unit.is_some() {
            input_error("--unit is only valid for Duration operations".into());
        }
        if let Some(expression) = expression {
            validate_expression(expression)
                .unwrap_or_else(|error| input_error(format!("invalid --expression: {error}")));
        } else {
            input_error("Missing --expression for EXPRESSION operation".into());
        }
        let token = result
            .unwrap_or_else(|| input_error("Missing --result for EXPRESSION operation".into()));
        let parsed = token
            .parse::<f64>()
            .unwrap_or_else(|_| input_error("--result for EXPRESSION must be numeric".into()));
        if !parsed.is_finite() {
            input_error("--result for EXPRESSION must be finite".into());
        }
        Some(parsed)
    } else {
        None
    };

    let parent_ids = parents
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if parent_ids.is_empty() {
        input_error("--parents must contain at least one node ID".into());
    }
    if parent_ids.len() > MAX_PARENTS {
        input_error(format!(
            "at most {MAX_PARENTS} parents supported (variables a-z), got {}",
            parent_ids.len()
        ));
    }
    if operation == "DATE_DIFF" && parent_ids.len() != 2 {
        input_error(format!(
            "DATE_DIFF requires exactly 2 Date parents, got {}",
            parent_ids.len()
        ));
    }
    if matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        validate_duration_parent_ids(operation, &parent_ids)
            .unwrap_or_else(|error| input_error(error));
    }

    let mut db = DbStore::open_in_project(".")
        .map_err(|error| format!("failed to open database: {error}"))?;
    let conn = db.conn_mut();
    let mut parent_values = Vec::with_capacity(parent_ids.len());
    for parent_id in &parent_ids {
        match get_tracked_node(conn, parent_id) {
            Ok(Some(node)) => parent_values.push(node.value),
            Ok(None) => input_error(format!("parent node '{parent_id}' not found in database")),
            Err(error) => input_error(format!("failed to load parent node '{parent_id}': {error}")),
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
            .unwrap_or_else(|error| calculation_error(operation, expression.unwrap_or(""), error));
            if let Some(claim) = &claim {
                claim.verify(seconds).unwrap_or_else(|error| {
                    calculation_error(operation, expression.unwrap_or(""), error)
                });
            }
            ValueData::Duration(Duration::try_seconds(seconds).unwrap_or_else(|| {
                calculation_error(
                    operation,
                    expression.unwrap_or(""),
                    format!(
                        "computed seconds '{seconds}' are outside the supported Duration range"
                    ),
                )
            }))
        }
        "EXPRESSION" => {
            let expected = expression_result.expect("validated above");
            verify(&parent_values, operation, expression, expected).unwrap_or_else(|error| {
                calculation_error(operation, expression.unwrap_or(""), error)
            });
            ValueData::Number(expected)
        }
        _ => unreachable!("valid_operation rejected unsupported operation"),
    };

    let now = chrono::Utc::now();
    let node_id = if let ValueData::Duration(duration) = &value {
        let seconds = duration.num_seconds();
        let canonical_id =
            TrackedNode::generate_duration_derived_id(&parent_ids, operation, seconds)
                .unwrap_or_else(|error| input_error(error));
        let node = TrackedNode {
            id: canonical_id,
            node_type: NodeType::Derived,
            value_kind: value.kind(),
            value: value.clone(),
            source_doc_id: None,
            line_number: None,
            is_stale: false,
            stale_reason: None,
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
        insert_or_reuse_duration(conn, &node, &parent_ids, operation, &legacy_candidates)?
    } else {
        let node_id = TrackedNode::generate_derived_id(&parent_ids, operation, &value);
        let node = TrackedNode {
            id: node_id.clone(),
            node_type: NodeType::Derived,
            value_kind: value.kind(),
            value: value.clone(),
            source_doc_id: None,
            line_number: None,
            is_stale: false,
            stale_reason: None,
            created_at: now,
            updated_at: now,
        };
        let tx = conn
            .unchecked_transaction()
            .map_err(|error| format!("failed to begin derivation transaction: {error}"))?;
        insert_tracked_node(&tx, &node)
            .map_err(|error| format!("failed to insert derived node: {error}"))?;
        for parent_id in &parent_ids {
            insert_derivation_edge(&tx, parent_id, &node_id, operation, expression)?;
        }
        tx.commit()
            .map_err(|error| format!("failed to commit derivation transaction: {error}"))?;
        node_id
    };

    println!("Derived node: {node_id}");
    if let (Some(unit), ValueData::Duration(duration)) = (display_unit, value) {
        let display = unit.display(duration.num_seconds());
        println!(
            "Duration: {} ({} seconds)",
            display.text, display.duration_seconds
        );
    }
    Ok(())
}

fn input_error(message: String) -> ! {
    eprintln!("Error: {message}");
    process::exit(2);
}

fn is_duration_operation(operation: &str) -> bool {
    matches!(operation, "DATE_DIFF" | "DURATION_SUM" | "DURATION_AVG")
}

fn calculation_error(operation: &str, expression: &str, message: String) -> ! {
    eprintln!(
        "Error: verification failed for {operation} \"{expression}\"\n  {message}\n  hint: retry with the correct result"
    );
    process::exit(1);
}
