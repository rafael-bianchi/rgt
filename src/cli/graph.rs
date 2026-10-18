use crate::export::turtle::{self, check_deadline, load_snapshot, MAX_TURTLE_BYTES};
use crate::store::queries::{get_child_edges, list_all_nodes};
use crate::store::DbStore;
use std::time::{Duration, Instant};

fn escape_mermaid(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
}

fn escape_dot(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

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
    let output = render_graph_string(format, include_absolute_paths, deadline)?;
    print!("{output}");
    Ok(())
}

pub fn render_graph_string(
    format: &str,
    include_absolute_paths: bool,
    deadline: Instant,
) -> Result<String, String> {
    let fmt_lower = format.to_lowercase();
    if !matches!(fmt_lower.as_str(), "text" | "mermaid" | "dot" | "ttl") {
        return Err(format!(
            "unsupported graph format '{}' (supported: text, mermaid, dot, ttl)",
            format
        ));
    }
    if fmt_lower == "ttl" {
        return render_turtle_string(include_absolute_paths, deadline);
    }
    if include_absolute_paths {
        return Err("--include-absolute-paths is only valid with `rgt graph --format ttl`".into());
    }

    let mut db = match DbStore::open_existing_in_project(".")
        .map_err(|e| format!("Failed to open DB: {}", e))?
    {
        Some(db) => db,
        None => {
            return Ok(match fmt_lower.as_str() {
                "mermaid" => "graph TD\n".to_string(),
                "dot" => "digraph RGT {\n}\n".to_string(),
                _ => "=== RGT Provenance DAG Tree ===\n".to_string(),
            });
        }
    };
    crate::graph::invalidation::sync_staleness_with_deadline(&mut db, Some(deadline))
        .map_err(|e| format!("Staleness check failed: {}", e))?;
    let conn = db.conn();

    let all_nodes = list_all_nodes(conn).map_err(|e| e.to_string())?;
    let mut out = String::new();

    match fmt_lower.as_str() {
        "mermaid" => {
            out.push_str("graph TD\n");
            for node in &all_nodes {
                let status_icon = if node.is_stale { " (STALE)" } else { "" };
                let val_str = escape_mermaid(&node.value.to_string_repr());
                let kind_str = escape_mermaid(&node.value_kind.to_string());
                use std::fmt::Write;
                let _ = writeln!(
                    out,
                    "    {}[\"{}: {}{}\"]",
                    node.id, kind_str, val_str, status_icon
                );

                let edges = get_child_edges(conn, &node.id)
                    .map_err(|e| format!("Failed to query child edges: {}", e))?;
                for edge in edges {
                    let op_str = escape_mermaid(&edge.operation_type);
                    let _ = writeln!(
                        out,
                        "    {} -->|\"{}\"| {}",
                        edge.parent_node_id, op_str, edge.child_node_id
                    );
                }
            }
        }
        "dot" => {
            out.push_str("digraph RGT {\n");
            for node in &all_nodes {
                let color = if node.is_stale { "red" } else { "black" };
                let val_str = escape_dot(&node.value.to_string_repr());
                let kind_str = escape_dot(&node.value_kind.to_string());
                use std::fmt::Write;
                let _ = writeln!(
                    out,
                    "    \"{}\" [label=\"{}: {}\", color={}];",
                    node.id, kind_str, val_str, color
                );

                let edges = get_child_edges(conn, &node.id)
                    .map_err(|e| format!("Failed to query child edges: {}", e))?;
                for edge in edges {
                    let op_str = escape_dot(&edge.operation_type);
                    let _ = writeln!(
                        out,
                        "    \"{}\" -> \"{}\" [label=\"{}\"];",
                        edge.parent_node_id, edge.child_node_id, op_str
                    );
                }
            }
            out.push_str("}\n");
        }
        _ => {
            out.push_str("=== RGT Provenance DAG Tree ===\n");
            for node in &all_nodes {
                let status = if node.is_stale { "STALE" } else { "VALID" };
                use std::fmt::Write;
                let _ = writeln!(
                    out,
                    "[{}] {} = {} ({})",
                    node.id,
                    node.value_kind,
                    node.value.to_string_repr(),
                    status
                );
                let edges = get_child_edges(conn, &node.id)
                    .map_err(|e| format!("Failed to query child edges: {}", e))?;
                for edge in edges {
                    let _ = writeln!(
                        out,
                        "    └── {} --> {}",
                        edge.operation_type, edge.child_node_id
                    );
                }
            }
        }
    }

    Ok(out)
}

fn render_turtle_string(include_absolute_paths: bool, deadline: Instant) -> Result<String, String> {
    check_deadline(deadline)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    let mut db = match DbStore::open_existing_in_project_with_timeout(".", remaining)
        .map_err(|e| format!("failed to open DB for Turtle export: {e}"))?
    {
        Some(db) => db,
        None => {
            let empty_snapshot = turtle::ExportSnapshot {
                project_id: "uninitialized".to_string(),
                nodes: Vec::new(),
                sources: Vec::new(),
                edges: Vec::new(),
                captures: Vec::new(),
            };
            let project_root = std::path::PathBuf::from(".");
            let rendered = turtle::render_turtle(
                &empty_snapshot,
                &project_root,
                include_absolute_paths,
                deadline,
            )?;
            return Ok(rendered.turtle);
        }
    };
    crate::graph::invalidation::sync_staleness_with_deadline(&mut db, Some(deadline))
        .map_err(|e| format!("Staleness check failed: {e}"))?;
    let snapshot = load_snapshot(db.conn(), deadline)?;
    let project_root =
        DbStore::find_project_root(".").unwrap_or_else(|| std::path::PathBuf::from("."));
    let rendered =
        turtle::render_turtle(&snapshot, &project_root, include_absolute_paths, deadline)?;
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
    Ok(rendered.turtle)
}
