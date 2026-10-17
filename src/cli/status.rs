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

    let root = DbStore::find_project_root(".").unwrap_or_else(|| std::path::PathBuf::from("."));
    execute_status_with(
        &db,
        &mut cascade,
        |c, d| c.evaluate_and_invalidate_all(d, root),
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
    let stale_nodes = list_stale_nodes(conn).map_err(|e| e.to_string())?;

    let summary = build_status_summary(&all_nodes, &stale_nodes);

    if json_output {
        let output = if stale_only {
            json!({
                "stale_nodes": summary["stale_nodes"],
                "stale_details": summary["stale_details"],
            })
        } else {
            summary
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        print!("{}", format_status_text(&summary, &stale_nodes, stale_only));
    }

    Ok(())
}

/// Formats the human-readable status report as a String.
pub fn format_status_text(
    summary: &serde_json::Value,
    stale_nodes: &[crate::types::TrackedNode],
    stale_only: bool,
) -> String {
    let mut out = String::new();
    if stale_only {
        if stale_nodes.is_empty() {
            out.push_str("✓ No stale nodes found.\n");
        } else {
            out.push_str(&format!("⚠️  Stale Nodes Found ({}):\n", stale_nodes.len()));
            for n in stale_nodes {
                out.push_str(&format!(
                    "  - [{}] Kind: {} | Value: {} | Reason: {}\n",
                    n.id,
                    n.value_kind,
                    n.value.to_string_repr(),
                    n.stale_reason.as_deref().unwrap_or("UNKNOWN")
                ));
            }
        }
    } else {
        out.push_str("=== RGT Provenance Graph Status ===\n");
        out.push_str(&format!("Total Nodes:  {}\n", summary["total_nodes"]));
        out.push_str(&format!("Active Nodes: {}\n", summary["active_nodes"]));
        out.push_str(&format!("Stale Nodes:  {}\n\n", summary["stale_nodes"]));

        if stale_nodes.is_empty() {
            out.push_str("✓ All tracked values are fresh and valid.\n");
        } else {
            out.push_str("⚠️  Stale Nodes Found:\n");
            for n in stale_nodes {
                out.push_str(&format!(
                    "  - [{}] Kind: {} | Value: {} | Reason: {}\n",
                    n.id,
                    n.value_kind,
                    n.value.to_string_repr(),
                    n.stale_reason.as_deref().unwrap_or("UNKNOWN")
                ));
            }
        }
    }
    out
}

/// Builds the graph-state summary JSON (spec 041 FR-004): returned as a value so
/// the CLI's `--json` branch and the MCP `get_status` tool share one construction.
pub fn build_status_summary(
    all_nodes: &[crate::types::TrackedNode],
    stale_nodes: &[crate::types::TrackedNode],
) -> serde_json::Value {
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

    json!({
        "total_nodes": all_nodes.len(),
        "active_nodes": all_nodes.len() - stale_nodes.len(),
        "stale_nodes": stale_nodes.len(),
        "stale_details": stale_details,
    })
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

    #[test]
    fn format_status_text_stale_only_empty_and_populated() {
        use crate::types::{NodeType, TrackedNode, ValueData};
        use chrono::Utc;

        let empty_summary = serde_json::json!({
            "total_nodes": 5,
            "active_nodes": 5,
            "stale_nodes": 0,
            "stale_details": [],
        });
        let empty_out = format_status_text(&empty_summary, &[], true);
        assert_eq!(empty_out, "✓ No stale nodes found.\n");

        let stale_node = TrackedNode {
            id: "stale_1".to_string(),
            node_type: NodeType::Root,
            value_kind: ValueData::Number(42.0).kind(),
            value: ValueData::Number(42.0),
            source_doc_id: None,
            line_number: Some(10),
            is_stale: true,
            stale_reason: Some("FILE_MODIFIED".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let pop_summary = serde_json::json!({
            "total_nodes": 5,
            "active_nodes": 4,
            "stale_nodes": 1,
            "stale_details": [{ "node_id": "stale_1" }],
        });
        let pop_out = format_status_text(&pop_summary, &[stale_node], true);
        assert!(!pop_out.contains("=== RGT Provenance Graph Status ==="));
        assert!(pop_out.contains("⚠️  Stale Nodes Found (1):"));
        assert!(pop_out.contains("[stale_1] Kind: NUMBER | Value: 42 | Reason: FILE_MODIFIED"));
    }
}
