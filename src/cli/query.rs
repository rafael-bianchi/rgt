use crate::query::query_provenance;
use serde_json::json;

/// Executes `rgt query <node_id>`: queries the derivation lineage for a node and
/// prints it as text or JSON.
pub fn execute_query(node_id: &str, json_output: bool) -> Result<(), String> {
    let result = query_provenance(node_id)?;

    if json_output {
        let json_str = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
        println!("{}", json_str);
    } else {
        let root_info = json!({
            "lineage": result
        });
        let json_str = serde_json::to_string_pretty(&root_info).map_err(|e| e.to_string())?;
        println!("{}", json_str);
    }

    Ok(())
}
