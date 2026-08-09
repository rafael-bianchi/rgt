use crate::store::queries::{get_parent_edges, get_tracked_node};
use crate::store::DbStore;
use serde_json::json;
use std::collections::{HashSet, VecDeque};

/// Queries the complete provenance lineage for a node using BFS traversal.
///
/// Returns JSON with the node and its `lineage_steps`, `total_ancestors`, and `is_stale`.
/// Uses a cycle guard (HashSet) to prevent infinite loops.
pub fn query_provenance(node_id: &str) -> Result<serde_json::Value, String> {
    let db = DbStore::open_in_project(".").map_err(|e| e.to_string())?;
    let conn = db.conn();

    let node = get_tracked_node(conn, node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Node not found: {}", node_id))?;

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(node_id.to_string());

    let mut lineage_steps = Vec::new();
    let is_stale = node.is_stale;

    while let Some(current_id) = queue.pop_front() {
        if visited.contains(&current_id) {
            continue;
        }

        let current_node = match get_tracked_node(conn, &current_id).map_err(|e| e.to_string())? {
            Some(n) => n,
            None => continue,
        };
        visited.insert(current_id.clone());

        let parent_edges = get_parent_edges(conn, &current_id).map_err(|e| e.to_string())?;
        let mut parent_ids: Vec<String> = parent_edges
            .iter()
            .map(|e| e.parent_node_id.clone())
            .collect();
        parent_ids.sort();

        lineage_steps.push(json!({
            "node_id": current_node.id,
            "value_kind": current_node.value_kind.to_string(),
            "value": current_node.value.to_string_repr(),
            "is_stale": current_node.is_stale,
            "stale_reason": current_node.stale_reason,
            "parent_ids": parent_ids,
            "line_number": current_node.line_number,
            "node_type": format!("{:?}", current_node.node_type),
        }));

        for pid in &parent_ids {
            if !visited.contains(pid) {
                queue.push_back(pid.clone());
            }
        }
    }

    let total_ancestors = visited.len().saturating_sub(1);

    Ok(json!({
        "node_id": node.id,
        "is_stale": is_stale,
        "total_ancestors": total_ancestors,
        "lineage_steps": lineage_steps
    }))
}
