use crate::graph::invalidation::InvalidationCascade;
use crate::store::queries::{list_all_nodes, list_stale_nodes};
use crate::store::DbStore;
use serde_json::json;

/// Executes `rgt status`: evaluates file changes, builds the invalidation cascade,
/// and prints a summary of total/active/stale nodes (text or JSON).
pub fn execute_status(stale_only: bool, json_output: bool) -> Result<(), String> {
    let db = DbStore::open_in_project(".").map_err(|e| format!("Failed to open DB: {}", e))?;

    let mut cascade = InvalidationCascade::build_from_db(&db)
        .map_err(|e| format!("Failed to build invalidation cascade: {}", e))?;

    execute_status_with(
        &db,
        &mut cascade,
        |c, d| c.evaluate_and_invalidate_all(d, "."),
        stale_only,
        json_output,
    )
}

/// Testable core of `execute_status`. The invalidation step is injectable so the
/// FR-004 propagation can be tested deterministically: a failing staleness check
/// MUST surface as an error (never print "all fresh" after a discarded failure).
fn execute_status_with(
    db: &DbStore,
    cascade: &mut InvalidationCascade,
    invalidate: impl FnOnce(&mut InvalidationCascade, &DbStore) -> rusqlite::Result<usize>,
    stale_only: bool,
    json_output: bool,
) -> Result<(), String> {
    // FR-004: propagate the staleness-check result instead of discarding it.
    invalidate(cascade, db).map_err(|e| format!("Staleness check failed: {}", e))?;

    let conn = db.conn();
    let all_nodes = list_all_nodes(conn).map_err(|e| e.to_string())?;
    let stale_nodes = if stale_only {
        list_stale_nodes(conn).map_err(|e| e.to_string())?
    } else {
        list_stale_nodes(conn).map_err(|e| e.to_string())?
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::invalidation::InvalidationCascade;

    /// FR-004: a failing staleness-check step must surface as an error, never
    /// print "all fresh" after a discarded failure.
    #[test]
    fn failing_invalidation_step_is_propagated_not_discarded() {
        let db = DbStore::open_in_memory().unwrap();
        let mut cascade = InvalidationCascade::build_from_db(&db).unwrap();

        let err = execute_status_with(
            &db,
            &mut cascade,
            |_, _| {
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(5),
                    Some("simulated failure".to_string()),
                ))
            },
            false,
            false,
        )
        .unwrap_err();

        assert!(err.contains("Staleness check failed"), "{}", err);
    }

    /// FR-004: a succeeding step produces the normal report (no error).
    #[test]
    fn succeeding_invalidation_step_produces_report() {
        let db = DbStore::open_in_memory().unwrap();
        let mut cascade = InvalidationCascade::build_from_db(&db).unwrap();
        let result = execute_status_with(
            &db,
            &mut cascade,
            |c, d| c.evaluate_and_invalidate_all(d, "."),
            false,
            false,
        );
        assert!(result.is_ok(), "{:?}", result.err());
    }
}
