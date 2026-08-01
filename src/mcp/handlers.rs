use crate::detection::{compute_blake3_hash, get_metadata_snapshot};
use crate::store::queries::{
    get_parent_edges, get_tracked_node, insert_derivation_edge,
    insert_tracked_node, list_stale_nodes, upsert_source_document,
};
use crate::store::DbStore;
use crate::types::{NodeType, TrackedNode, ValueData};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::collections::{HashSet, VecDeque};
use std::path::Path;

pub fn handle_record_value(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let file_path = params
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'file_path'".to_string())?;

    let value_kind_str = params
        .get("value_kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'value_kind'".to_string())?;

    let line_number = params.get("line_number").and_then(|v| v.as_u64()).map(|n| n as u32);

    let val = match value_kind_str.to_uppercase().as_str() {
        "NUMBER" => {
            let num = params
                .get("number_value")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing 'number_value'".to_string())?;
            ValueData::Number(num)
        }
        "DATE" => {
            let dt_str = params
                .get("date_value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'date_value'".to_string())?;
            let dt = DateTime::parse_from_rfc3339(dt_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| format!("Invalid date format: {}", e))?;
            ValueData::Date(dt)
        }
        _ => return Err("Invalid value_kind for record_value".to_string()),
    };

    let path_obj = Path::new(file_path);
    let (mtime_nsec, file_size, hash) = if path_obj.exists() {
        let meta = get_metadata_snapshot(path_obj).map_err(|e| e.to_string())?;
        let h = compute_blake3_hash(path_obj).map_err(|e| e.to_string())?;
        (meta.mtime_nsec, meta.file_size, h)
    } else {
        (0, 0, "dummy_hash".to_string())
    };

    let db = DbStore::open_in_project(".").map_err(|e| e.to_string())?;
    let doc = upsert_source_document(db.conn(), file_path, mtime_nsec, file_size, &hash)
        .map_err(|e| e.to_string())?;

    let node_id = TrackedNode::generate_root_id(file_path, line_number, &val);
    let now = Utc::now();
    let node = TrackedNode {
        id: node_id.clone(),
        node_type: NodeType::Root,
        value_kind: val.kind(),
        value: val,
        source_doc_id: Some(doc.id),
        line_number,
        is_stale: false,
        stale_reason: None,
        created_at: now,
        updated_at: now,
    };

    insert_tracked_node(db.conn(), &node).map_err(|e| e.to_string())?;

    Ok(json!({
        "node_id": node_id,
        "is_stale": false
    }))
}

pub fn handle_record_derivation(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let parents = params
        .get("parent_node_ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing 'parent_node_ids'".to_string())?;

    let parent_ids: Vec<String> = parents
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    if parent_ids.is_empty() {
        return Err("parent_node_ids cannot be empty".to_string());
    }

    let op_type = params
        .get("operation_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'operation_type'".to_string())?;

    let value_kind_str = params
        .get("value_kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'value_kind'".to_string())?;

    let val = match value_kind_str.to_uppercase().as_str() {
        "NUMBER" => {
            let num = params
                .get("number_value")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| "Missing 'number_value'".to_string())?;
            ValueData::Number(num)
        }
        "DATE" => {
            let dt_str = params
                .get("date_value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'date_value'".to_string())?;
            let dt = DateTime::parse_from_rfc3339(dt_str)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| format!("Invalid date format: {}", e))?;
            ValueData::Date(dt)
        }
        "DURATION" => {
            let secs = params
                .get("duration_seconds")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| "Missing 'duration_seconds'".to_string())?;
            ValueData::Duration(Duration::seconds(secs))
        }
        _ => return Err("Invalid value_kind for record_derivation".to_string()),
    };

    let expression = params.get("expression").and_then(|v| v.as_str());

    let db = DbStore::open_in_project(".").map_err(|e| e.to_string())?;
    let conn = db.conn();

    let mut any_parent_stale = false;
    for pid in &parent_ids {
        if let Some(pnode) = get_tracked_node(conn, pid).map_err(|e| e.to_string())? {
            if pnode.is_stale {
                any_parent_stale = true;
            }
        }
    }

    let node_id = TrackedNode::generate_derived_id(&parent_ids, op_type, &val);
    let now = Utc::now();
    let node = TrackedNode {
        id: node_id.clone(),
        node_type: NodeType::Derived,
        value_kind: val.kind(),
        value: val,
        source_doc_id: None,
        line_number: None,
        is_stale: any_parent_stale,
        stale_reason: if any_parent_stale {
            Some("PARENT_NODE_STALE".to_string())
        } else {
            None
        },
        created_at: now,
        updated_at: now,
    };

    insert_tracked_node(conn, &node).map_err(|e| e.to_string())?;

    for pid in &parent_ids {
        insert_derivation_edge(conn, pid, &node_id, op_type, expression).map_err(|e| e.to_string())?;
    }

    Ok(json!({
        "node_id": node_id,
        "is_stale": any_parent_stale,
        "parent_count": parent_ids.len()
    }))
}

pub fn handle_query_provenance(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let node_id = params
        .get("node_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'node_id'".to_string())?;

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
        let mut parent_ids: Vec<String> = parent_edges.iter().map(|e| e.parent_node_id.clone()).collect();
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

pub fn handle_list_stale_values() -> Result<serde_json::Value, String> {
    let db = DbStore::open_in_project(".").map_err(|e| e.to_string())?;
    let stale_nodes = list_stale_nodes(db.conn()).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for node in stale_nodes {
        list.push(json!({
            "node_id": node.id,
            "value_kind": node.value_kind.to_string(),
            "value": node.value.to_string_repr(),
            "stale_reason": node.stale_reason,
        }));
    }

    Ok(json!({
        "stale_nodes": list
    }))
}
