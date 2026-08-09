use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GraphEngine {
    graph: DiGraph<String, String>, // NodeId -> NodeId with Operation label
    node_map: HashMap<String, NodeIndex>,
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphEngine {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub fn get_or_add_node(&mut self, node_id: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(node_id) {
            idx
        } else {
            let idx = self.graph.add_node(node_id.to_string());
            self.node_map.insert(node_id.to_string(), idx);
            idx
        }
    }

    pub fn add_derivation_edge(
        &mut self,
        parent_id: &str,
        child_id: &str,
        op: &str,
    ) -> Result<(), String> {
        let parent_idx = self.get_or_add_node(parent_id);
        let child_idx = self.get_or_add_node(child_id);

        let edge_idx = self.graph.add_edge(parent_idx, child_idx, op.to_string());

        if is_cyclic_directed(&self.graph) {
            self.graph.remove_edge(edge_idx);
            return Err(format!(
                "Adding edge from {} to {} introduces a cycle",
                parent_id, child_id
            ));
        }

        Ok(())
    }

    pub fn get_downstream_dependents(&self, root_node_id: &str) -> Vec<String> {
        let mut dependents = Vec::new();
        if let Some(&start_idx) = self.node_map.get(root_node_id) {
            let mut bfs = Bfs::new(&self.graph, start_idx);
            while let Some(nx) = bfs.next(&self.graph) {
                if nx != start_idx {
                    if let Some(id) = self.graph.node_weight(nx) {
                        dependents.push(id.clone());
                    }
                }
            }
        }
        dependents
    }

    #[allow(dead_code)]
    pub fn get_upstream_provenance(&self, target_node_id: &str) -> Vec<String> {
        let mut upstream = Vec::new();
        let reversed = petgraph::visit::Reversed(&self.graph);
        if let Some(&start_idx) = self.node_map.get(target_node_id) {
            let mut bfs = Bfs::new(&reversed, start_idx);
            while let Some(nx) = bfs.next(&reversed) {
                if let Some(id) = self.graph.node_weight(nx) {
                    upstream.push(id.clone());
                }
            }
        }
        upstream
    }
}
