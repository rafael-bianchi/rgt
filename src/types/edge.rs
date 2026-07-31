use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationEdge {
    pub id: i64,
    pub parent_node_id: String,
    pub child_node_id: String,
    pub operation_type: String,
    pub expression: Option<String>,
}
