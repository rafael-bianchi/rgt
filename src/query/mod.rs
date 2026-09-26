use crate::store::queries::{get_parent_edges, get_tracked_node};
use crate::store::DbStore;
use crate::types::{DurationUnit, ValueData};
use serde_json::json;
use std::collections::{HashSet, VecDeque};

#[derive(Debug)]
pub enum QueryError {
    InvalidInput(String),
    Error(String),
}

/// Queries the complete provenance lineage for a node using BFS traversal.
///
/// Returns JSON with the node and its `lineage_steps`, `total_ancestors`, and `is_stale`.
/// Uses a cycle guard (HashSet) to prevent infinite loops.
pub fn query_provenance(node_id: &str) -> Result<serde_json::Value, QueryError> {
    query_provenance_with_unit(node_id, None)
}

/// Queries provenance and optionally adds a selected-unit view to the requested
/// node's lineage entry. Ancestors retain their existing response shape.
pub fn query_provenance_with_unit(
    node_id: &str,
    unit: Option<DurationUnit>,
) -> Result<serde_json::Value, QueryError> {
    let db = DbStore::open_in_project(".").map_err(|e| QueryError::Error(e.to_string()))?;
    let conn = db.conn();

    let node = get_tracked_node(conn, node_id)
        .map_err(|e| QueryError::Error(e.to_string()))?
        .ok_or_else(|| QueryError::Error(format!("Node not found: {}", node_id)))?;
    if unit.is_some() && !matches!(&node.value, ValueData::Duration(_)) {
        return Err(QueryError::InvalidInput(
            "--unit requires the requested node to be a Duration".to_string(),
        ));
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(node_id.to_string());

    let mut lineage_steps = Vec::new();
    let is_stale = node.is_stale;

    while let Some(current_id) = queue.pop_front() {
        if visited.contains(&current_id) {
            continue;
        }

        let current_node = match get_tracked_node(conn, &current_id)
            .map_err(|e| QueryError::Error(e.to_string()))?
        {
            Some(n) => n,
            None => continue,
        };
        visited.insert(current_id.clone());

        let parent_edges =
            get_parent_edges(conn, &current_id).map_err(|e| QueryError::Error(e.to_string()))?;
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

    if let Some(unit) = unit {
        let seconds = match &node.value {
            ValueData::Duration(duration) => duration.num_seconds(),
            _ => unreachable!("Duration checked above"),
        };
        let display = unit.display(seconds);
        if let Some(requested_step) = lineage_steps.first_mut() {
            requested_step["duration_seconds"] = json!(display.duration_seconds);
            requested_step["selected_unit_display"] = json!({
                "unit": display.unit.name(),
                "decimal": display.decimal,
                "approximate": display.approximate,
                "text": display.text,
            });
        }
    }

    Ok(json!({
        "node_id": node.id,
        "is_stale": is_stale,
        "total_ancestors": total_ancestors,
        "lineage_steps": lineage_steps
    }))
}
