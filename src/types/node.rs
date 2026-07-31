use crate::types::value::{ValueData, ValueKind};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Root,
    Derived,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Root => write!(f, "ROOT"),
            NodeType::Derived => write!(f, "DERIVED"),
        }
    }
}

impl std::str::FromStr for NodeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ROOT" => Ok(NodeType::Root),
            "DERIVED" => Ok(NodeType::Derived),
            _ => Err(format!("Unknown node type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: i64,
    pub file_path: String,
    pub mtime_nsec: i64,
    pub file_size: u64,
    pub blake3_hash: String,
    pub last_checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedNode {
    pub id: String,
    pub node_type: NodeType,
    pub value_kind: ValueKind,
    pub value: ValueData,
    pub source_doc_id: Option<i64>,
    pub line_number: Option<u32>,
    pub is_stale: bool,
    pub stale_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TrackedNode {
    pub fn generate_root_id(file_path: &str, line_number: Option<u32>, val: &ValueData) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(file_path.as_bytes());
        if let Some(line) = line_number {
            hasher.update(&line.to_le_bytes());
        }
        hasher.update(val.to_string_repr().as_bytes());
        let hash_hex = hasher.finalize().to_hex();
        format!("node_raw_{}", &hash_hex[..12])
    }

    pub fn generate_derived_id(parents: &[String], op: &str, val: &ValueData) -> String {
        let mut hasher = blake3::Hasher::new();
        for p in parents {
            hasher.update(p.as_bytes());
        }
        hasher.update(op.as_bytes());
        hasher.update(val.to_string_repr().as_bytes());
        let hash_hex = hasher.finalize().to_hex();
        format!("node_drv_{}", &hash_hex[..12])
    }
}
