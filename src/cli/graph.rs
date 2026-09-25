use crate::export::turtle::{self, check_deadline, load_snapshot, MAX_TURTLE_BYTES};
use crate::store::queries::{get_child_edges, list_all_nodes};
use crate::store::DbStore;
use std::time::{Duration, Instant};

/// Executes `rgt graph [-f text|mermaid|dot]`: exports the provenance DAG in
/// the requested format.
#[allow(dead_code)]
pub fn execute_graph(format: &str) -> Result<(), String> {
    execute_graph_with_options(format, false, Instant::now() + Duration::from_secs(1))
}

pub fn execute_graph_with_options(
    format: &str,
    include_absolute_paths: bool,
    deadline: Instant,
) -> Result<(), String> {
    if format.eq_ignore_ascii_case(turtle::format_name()) {
        return execute_turtle(include_absolute_paths, deadline);
    }
    if include_absolute_paths {
        return Err("--include-absolute-paths is only valid with `rgt graph --format ttl`".into());
    }

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

fn execute_turtle(include_absolute_paths: bool, deadline: Instant) -> Result<(), String> {
    check_deadline(deadline)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    let db = DbStore::open_in_project_with_timeout(".", remaining)
        .map_err(|e| format!("failed to open DB for Turtle export: {e}"))?;
    let snapshot = load_snapshot(db.conn(), deadline)?;
    let rendered = turtle::render_turtle(
        &snapshot,
        std::path::Path::new("."),
        include_absolute_paths,
        deadline,
    )?;
    if rendered.turtle.len() > MAX_TURTLE_BYTES {
        return Err(format!(
            "Turtle export exceeds the {MAX_TURTLE_BYTES}-byte output limit"
        ));
    }
    for child in rendered.ambiguous_children {
        eprintln!(
            "Warning: omitted derivation activity for '{}' because its stored derivation evidence is incomplete or inconsistent",
            child
        );
    }
    turtle::write_bytes_before_deadline(std::io::stdout(), rendered.turtle.into_bytes(), deadline)
}
