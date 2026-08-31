use crate::query::{query_provenance, query_provenance_with_unit, QueryError};
use crate::types::DurationUnit;
use serde_json::Value;

/// Executes `rgt query <node_id>`: queries the derivation lineage for a node and
/// prints it as human-readable text by default, or as JSON with `--json`.
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
        println!("{}", render_lineage_text(&result));
    }

    Ok(())
}

/// Renders a `query_provenance` result as human-readable text.
///
/// Consumes the same JSON value used by the `--json` branch, so both modes
/// present identical lineage data (steps in the same order). Every field access
/// falls back safely: missing line numbers print `n/a`, a missing stale reason
/// is omitted, and roots print `parents: none`.
pub fn render_lineage_text(result: &Value) -> String {
    let mut out = String::new();
    out.push_str("=== RGT Provenance Lineage ===\n");

    let node_id = result["node_id"].as_str().unwrap_or("?");
    let is_stale = result["is_stale"].as_bool().unwrap_or(false);
    let total_ancestors = result["total_ancestors"].as_u64().unwrap_or(0);
    out.push_str(&format!(
        "node: {node_id} | stale: {} | total ancestors: {total_ancestors}\n",
        if is_stale { "YES" } else { "NO" }
    ));
    out.push('\n');

    let steps = result["lineage_steps"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if steps.is_empty() {
        out.push_str("(no lineage steps)\n");
        return out;
    }

    for (i, step) in steps.iter().enumerate() {
        out.push_str(&format!(
            "STEP {} — {}\n",
            i + 1,
            step["node_id"].as_str().unwrap_or("?")
        ));
        out.push_str(&format!(
            "  type:   {}\n",
            step["node_type"].as_str().unwrap_or("UNKNOWN")
        ));
        out.push_str(&format!(
            "  value:  {}\n",
            step["value"].as_str().unwrap_or("?")
        ));
        out.push_str(&format!(
            "  kind:   {}\n",
            step["value_kind"].as_str().unwrap_or("?")
        ));
        let stale = step["is_stale"].as_bool().unwrap_or(false);
        let reason = step["stale_reason"].as_str().unwrap_or("").to_string();
        out.push_str(&format!(
            "  stale:  {}{}\n",
            if stale { "YES" } else { "NO" },
            if stale && !reason.is_empty() {
                format!(" ({reason})")
            } else {
                String::new()
            }
        ));
        if let Some(display) = step.get("selected_unit_display") {
            out.push_str(&format!(
                "  display: {} ({})\n",
                display["text"].as_str().unwrap_or("?"),
                display["unit"].as_str().unwrap_or("?")
            ));
        }
        let line = step["line_number"].as_u64();
        out.push_str(&format!(
            "  line:   {}\n",
            line.map(|l| l.to_string())
                .unwrap_or_else(|| "n/a".to_string())
        ));
        let parents: Vec<String> = step["parent_ids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|p| p.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "  parents: {}\n",
            if parents.is_empty() {
                "none".to_string()
            } else {
                parents.join(", ")
            }
        ));
        out.push('\n');
    }

    out.pop(); // drop the trailing blank line
    out
}
