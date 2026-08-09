use crate::store::queries::{get_child_edges, list_all_nodes};
use crate::store::DbStore;

/// Executes `rgt graph [-f text|mermaid|dot]`: exports the provenance DAG in
/// the requested format.
pub fn execute_graph(format: &str) -> Result<(), String> {
    let db = DbStore::open_in_project(".").map_err(|e| format!("Failed to open DB: {}", e))?;
    let conn = db.conn();

    let all_nodes = list_all_nodes(conn).map_err(|e| e.to_string())?;

    match format.to_lowercase().as_str() {
        "mermaid" => {
            println!("graph TD");
            for node in &all_nodes {
                let status_icon = if node.is_stale { " (STALE)" } else { "" };
                println!(
                    "    {}[\"{}: {}{}\"]",
                    node.id,
                    node.value_kind,
                    node.value.to_string_repr(),
                    status_icon
                );

                if let Ok(edges) = get_child_edges(conn, &node.id) {
                    for edge in edges {
                        println!(
                            "    {} -->|\"{}\"| {}",
                            edge.parent_node_id, edge.operation_type, edge.child_node_id
                        );
                    }
                }
            }
        }
        "dot" => {
            println!("digraph RGT {{");
            for node in &all_nodes {
                let color = if node.is_stale { "red" } else { "black" };
                println!(
                    "    \"{}\" [label=\"{}: {}\", color={}];",
                    node.id,
                    node.value_kind,
                    node.value.to_string_repr(),
                    color
                );

                if let Ok(edges) = get_child_edges(conn, &node.id) {
                    for edge in edges {
                        println!(
                            "    \"{}\" -> \"{}\" [label=\"{}\"];",
                            edge.parent_node_id, edge.child_node_id, edge.operation_type
                        );
                    }
                }
            }
            println!("}}");
        }
        _ => {
            println!("=== RGT Provenance DAG Tree ===");
            for node in &all_nodes {
                let status = if node.is_stale { "STALE" } else { "VALID" };
                println!(
                    "[{}] {} = {} ({})",
                    node.id,
                    node.value_kind,
                    node.value.to_string_repr(),
                    status
                );
                if let Ok(edges) = get_child_edges(conn, &node.id) {
                    for edge in edges {
                        println!("    └── {} --> {}", edge.operation_type, edge.child_node_id);
                    }
                }
            }
        }
    }

    Ok(())
}
