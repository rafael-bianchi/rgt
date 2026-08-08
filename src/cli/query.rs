use crate::mcp::handlers::handle_query_provenance;
use serde_json::json;

/// Executes `rgt query <node_id>`: queries the derivation lineage for a node and
/// prints it as text or JSON.
pub fn execute_query(node_id: &str, json_output: bool) -> Result<(), String> {
    let params = json!({ "node_id": node_id });
    let result = handle_query_provenance(&params)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!("=== Lineage for Node: {} ===", node_id);
        if let Some(steps) = result.get("lineage_steps").and_then(|v| v.as_array()) {
            for (idx, step) in steps.iter().enumerate() {
                let nid = step.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
                let kind = step
                    .get("value_kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let val = step.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let stale = step
                    .get("is_stale")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                println!(
                    "Step {}: Node [{}] (Kind: {}, Value: {}) -> Stale: {}",
                    idx + 1,
                    nid,
                    kind,
                    val,
                    stale
                );
            }
        }
    }

    Ok(())
}
