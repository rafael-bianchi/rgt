use crate::query::{query_provenance, query_provenance_with_unit, QueryError};
use crate::types::DurationUnit;
use serde_json::json;

/// Executes `rgt query <node_id>`: queries the derivation lineage for a node and
/// prints it as text or JSON.
pub fn execute_query(node_id: &str, json_output: bool) -> Result<(), QueryError> {
    let result = query_provenance(node_id)?;

    print_query_result(result, json_output)
}

/// Executes a query with an optional selected Duration unit.
pub fn execute_query_with_unit(
    node_id: &str,
    json_output: bool,
    unit: Option<&str>,
) -> Result<(), QueryError> {
    let unit = unit
        .map(DurationUnit::parse)
        .transpose()
        .map_err(QueryError::InvalidInput)?;
    let result = query_provenance_with_unit(node_id, unit)?;

    print_query_result(result, json_output)
}

fn print_query_result(result: serde_json::Value, json_output: bool) -> Result<(), QueryError> {
    if json_output {
        let json_str =
            serde_json::to_string_pretty(&result).map_err(|e| QueryError::Error(e.to_string()))?;
        println!("{}", json_str);
    } else {
        let root_info = json!({
            "lineage": result
        });
        let json_str = serde_json::to_string_pretty(&root_info)
            .map_err(|e| QueryError::Error(e.to_string()))?;
        println!("{}", json_str);
    }

    Ok(())
}
