use crate::store::queries::delete_obsolete_nodes;
use crate::store::DbStore;

/// Executes `rgt gc`: transactionally deletes obsolete nodes (stale nodes with
/// no derived dependents) and reports how many were removed (FR-003/FR-004).
/// With `vacuum`, runs `VACUUM` after the delete commits to reclaim physical
/// space (SQLite forbids `VACUUM` inside a transaction).
pub fn execute_gc(vacuum: bool) -> Result<(), String> {
    let mut db =
        DbStore::open_in_project(".").map_err(|e| format!("failed to open database: {}", e))?;

    let tx = db
        .conn_mut()
        .transaction()
        .map_err(|e| format!("failed to begin transaction: {}", e))?;

    let removed = delete_obsolete_nodes(&tx)
        .map_err(|e| format!("failed to collect obsolete nodes: {}", e))?;

    tx.commit()
        .map_err(|e| format!("failed to commit gc transaction: {}", e))?;

    if vacuum {
        db.conn_mut()
            .execute_batch("VACUUM;")
            .map_err(|e| format!("failed to vacuum store: {}", e))?;
    }

    println!("Removed {} obsolete node(s)", removed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::queries::{insert_derivation_edge, insert_tracked_node, list_all_nodes};
    use crate::types::{NodeType, TrackedNode, ValueData};
    use chrono::Utc;

    fn node(id: &str, stale: bool) -> TrackedNode {
        TrackedNode {
            id: id.to_string(),
            node_type: NodeType::Root,
            value_kind: ValueData::Number(1.0).kind(),
            value: ValueData::Number(1.0),
            source_doc_id: None,
            line_number: None,
            is_stale: stale,
            stale_reason: if stale {
                Some("FILE_DELETED".into())
            } else {
                None
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn gc_removes_only_stale_leaves() {
        let db = DbStore::open_in_memory().unwrap();
        let conn = db.conn();
        insert_tracked_node(conn, &node("stale_leaf", true)).unwrap();
        insert_tracked_node(conn, &node("stale_parent", true)).unwrap();
        insert_tracked_node(conn, &node("fresh_leaf", false)).unwrap();
        insert_tracked_node(conn, &node("dependent", false)).unwrap();
        // stale_parent -> dependent (so stale_parent has a dependent -> kept).
        insert_derivation_edge(conn, "stale_parent", "dependent", "EXPRESSION", None).unwrap();

        let removed = delete_obsolete_nodes(conn).unwrap();
        assert_eq!(removed, 1, "only the stale leaf is obsolete");

        let mut remaining: Vec<String> = list_all_nodes(conn)
            .unwrap()
            .into_iter()
            .map(|n| n.id)
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            vec![
                "dependent".to_string(),
                "fresh_leaf".to_string(),
                "stale_parent".to_string()
            ]
        );
    }

    #[test]
    fn gc_with_nothing_obsolete_removes_zero() {
        let db = DbStore::open_in_memory().unwrap();
        let conn = db.conn();
        insert_tracked_node(conn, &node("fresh", false)).unwrap();
        insert_tracked_node(conn, &node("stale_with_dependent", true)).unwrap();
        insert_tracked_node(conn, &node("child", false)).unwrap();
        insert_derivation_edge(conn, "stale_with_dependent", "child", "EXPRESSION", None).unwrap();

        assert_eq!(delete_obsolete_nodes(conn).unwrap(), 0);
    }
}
