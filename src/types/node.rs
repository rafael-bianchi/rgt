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
    /// Generates a stable root-node ID from the value's provenance.
    ///
    /// `occurrence` is the per-line occurrence index (0-based). It is hashed
    /// only when non-zero, so a line's first/only value keeps the pre-change ID
    /// (backward compatible — re-recording single-value lines dedupes), while a
    /// same-line repeat gets a distinct ID (FR-002/FR-003).
    pub fn generate_root_id(
        file_path: &str,
        line_number: Option<u32>,
        occurrence: u32,
        val: &ValueData,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(file_path.as_bytes());
        if let Some(line) = line_number {
            hasher.update(&line.to_le_bytes());
        }
        if occurrence > 0 {
            hasher.update(&occurrence.to_le_bytes());
        }
        hasher.update(val.to_string_repr().as_bytes());
        let hash_hex = hasher.finalize().to_hex();
        format!("node_raw_{}", &hash_hex[..32])
    }

    pub fn generate_derived_id(parents: &[String], op: &str, val: &ValueData) -> String {
        let mut hasher = blake3::Hasher::new();
        for p in parents {
            hasher.update(p.as_bytes());
        }
        hasher.update(op.as_bytes());
        hasher.update(val.to_string_repr().as_bytes());
        let hash_hex = hasher.finalize().to_hex();
        format!("node_drv_{}", &hash_hex[..32])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ValueData;

    fn base_id(file: &str, line: Option<u32>, val: &ValueData) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(file.as_bytes());
        if let Some(l) = line {
            hasher.update(&l.to_le_bytes());
        }
        hasher.update(val.to_string_repr().as_bytes());
        format!("node_raw_{}", &hasher.finalize().to_hex()[..32])
    }

    #[test]
    fn root_id_occurrence_zero_is_deterministic_and_at_least_128_bits() {
        let v = ValueData::Number(120000.0);
        let a = TrackedNode::generate_root_id("f.csv", Some(1), 0, &v);
        let b = TrackedNode::generate_root_id("f.csv", Some(1), 0, &v);
        assert_eq!(a, b, "occurrence-0 must be deterministic");
        assert!(
            a.len() >= 8 + 32,
            "root ID must be at least 128 bits (32 hex chars), got {}",
            a.len()
        );
        assert_eq!(
            a,
            base_id("f.csv", Some(1), &v),
            "occurrence-0 must equal the file+line+value hash"
        );
    }

    #[test]
    fn derived_id_is_at_least_128_bits() {
        let v = ValueData::Number(42.0);
        let id = TrackedNode::generate_derived_id(&["a".to_string()], "EXPRESSION", &v);
        assert!(
            id.len() >= 8 + 32,
            "derived ID must be at least 128 bits, got {}",
            id.len()
        );
        let id2 = TrackedNode::generate_derived_id(&["a".to_string()], "EXPRESSION", &v);
        assert_eq!(id, id2, "derived ID must be deterministic");
    }

    #[test]
    fn root_id_occurrence_distinguishes_same_line_repeats() {
        let v = ValueData::Number(120000.0);
        let occ0 = TrackedNode::generate_root_id("f.csv", Some(1), 0, &v);
        let occ1 = TrackedNode::generate_root_id("f.csv", Some(1), 1, &v);
        assert_ne!(
            occ0, occ1,
            "same-line identical values must get distinct IDs"
        );
    }
}
