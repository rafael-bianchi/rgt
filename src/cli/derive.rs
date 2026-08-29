use crate::store::queries::{get_tracked_node, insert_derivation_edge, insert_tracked_node};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode, ValueData};
use crate::verify::{valid_operation, validate_expression, verify, MAX_PARENTS};
use chrono::Duration;
use std::process;

pub fn execute_derive(
    parents: &str,
    operation: &str,
    expression: Option<&str>,
    result: f64,
) -> Result<(), String> {
    // FR-002 / FR-001 / FR-005: validate operation, expression (for
    // EXPRESSION), and parent count up front — invalid input is exit 2, never
    // a silent skip, and never a record of an unverified value.
    if !valid_operation(operation) {
        eprintln!(
            "Error: unknown operation '{}' — valid operations: EXPRESSION, DATE_DIFF",
            operation
        );
        process::exit(2);
    }
    if operation == "EXPRESSION" {
        if let Some(expr) = expression {
            if let Err(e) = validate_expression(expr) {
                eprintln!("Error: invalid --expression: {}", e);
                process::exit(2);
            }
        } else {
            eprintln!("Error: Missing --expression for EXPRESSION operation");
            process::exit(2);
        }
    }

    let parent_ids: Vec<String> = parents
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if parent_ids.is_empty() {
        eprintln!("Error: --parents must contain at least one node ID");
        process::exit(2);
    }
    if parent_ids.len() > MAX_PARENTS {
        eprintln!(
            "Error: at most {} parents supported (variables a-z), got {}",
            MAX_PARENTS,
            parent_ids.len()
        );
        process::exit(2);
    }

    let db =
        DbStore::open_in_project(".").map_err(|e| format!("failed to open database: {}", e))?;
    let conn = db.conn();

    let mut parent_values = Vec::new();
    for pid in &parent_ids {
        match get_tracked_node(conn, pid) {
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

    match operation {
        "EXPRESSION" => {}
        "DATE_DIFF" => {
            if parent_ids.len() != 2 {
                eprintln!(
                    "Error: DATE_DIFF requires exactly 2 Date parents, got {}",
                    parent_ids.len()
                );
                process::exit(2);
            }
        }
        _ => unreachable!("valid_operation rejected above"),
    }

    match verify(&parent_values, operation, expression, result) {
        Ok(()) => {}
        Err(msg) => {
            let parts = msg.split(": ").collect::<Vec<_>>();
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

    let value = match operation {
        "DATE_DIFF" => ValueData::Duration(Duration::seconds(result as i64)),
        _ => ValueData::Number(result),
    };

    let node_id = TrackedNode::generate_derived_id(&parent_ids, operation, &value);
    let now = chrono::Utc::now();

    let node = TrackedNode {
        id: node_id.clone(),
        node_type: NodeType::Derived,
        value_kind: value.kind(),
        value,
        source_doc_id: None,
        line_number: None,
        is_stale: false,
        stale_reason: None,
        created_at: now,
        updated_at: now,
    };

    insert_tracked_node(conn, &node)
        .map_err(|e| format!("failed to insert derived node: {}", e))?;

    for pid in &parent_ids {
        insert_derivation_edge(conn, pid, &node_id, operation, expression)
            .map_err(|e| format!("failed to insert derivation edge: {}", e))?;
    }

    println!("Derived node: {}", node_id);

    Ok(())
}
