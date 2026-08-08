use crate::graph::InvalidationCascade;
use crate::store::queries::{list_all_nodes, list_stale_nodes};
use crate::store::DbStore;
use serde_json::json;

/// Executes `rgt status`: evaluates file changes, builds the invalidation cascade,
/// and prints a summary of total/active/stale nodes (text or JSON).
pub fn execute_status(_stale_only: bool, json_output: bool) -> Result<(), String> {
    let db = DbStore::open_in_project(".").map_err(|e| format!("Failed to open DB: {}", e))?;

    let mut cascade = InvalidationCascade::build_from_db(&db)
        .map_err(|e| format!("Failed to build invalidation cascade: {}", e))?;

    let _ = cascade.evaluate_and_invalidate_all(&db, ".");

    let conn = db.conn();
    let all_nodes = list_all_nodes(conn).map_err(|e| e.to_string())?;
    let stale_nodes = list_stale_nodes(conn).map_err(|e| e.to_string())?;

    if json_output {
        let stale_details: Vec<_> = stale_nodes
            .iter()
            .map(|n| {
                json!({
                    "node_id": n.id,
                    "value_kind": n.value_kind.to_string(),
                    "value": n.value.to_string_repr(),
                    "stale_reason": n.stale_reason,
                })
            })
            .collect();

        let output = json!({
            "total_nodes": all_nodes.len(),
            "active_nodes": all_nodes.len() - stale_nodes.len(),
            "stale_nodes": stale_nodes.len(),
            "stale_details": stale_details,
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        println!("=== RGT Provenance Graph Status ===");
        println!("Total Nodes:  {}", all_nodes.len());
        println!("Active Nodes: {}", all_nodes.len() - stale_nodes.len());
        println!("Stale Nodes:  {}", stale_nodes.len());
        println!();

        if stale_nodes.is_empty() {
            println!("✓ All tracked values are fresh and valid.");
        } else {
            println!("⚠️  Stale Nodes Found:");
            for n in &stale_nodes {
                println!(
                    "  - [{}] Kind: {} | Value: {} | Reason: {}",
                    n.id,
                    n.value_kind,
                    n.value.to_string_repr(),
                    n.stale_reason.as_deref().unwrap_or("UNKNOWN")
                );
            }
        }
    }

    Ok(())
}
