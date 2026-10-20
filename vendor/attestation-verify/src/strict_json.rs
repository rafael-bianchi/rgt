//! Strict JSON parsing: rejects duplicate object keys and oversized value
//! trees.
//!
//! `serde_json` silently lets the last occurrence of a duplicate object key
//! win, both when deserializing directly into a struct and when collecting
//! into a [`serde_json::Value`]. Once a `Value` tree has been built, the
//! information that a duplicate existed is already gone, so rejecting
//! duplicates has to happen during the initial parse, not as a later check
//! on the parsed tree.
//!
//! DESIGN.md "Core decisions" item 2 requires duplicate-key rejection as
//! part of hardened parsing. This module implements it once, generically,
//! with a custom [`Visitor`] that mirrors `serde_json::Value`'s own
//! deserialization but tracks the keys seen at each object nesting level
//! and errors on a repeat. Callers get back an ordinary `serde_json::Value`
//! on success and then use `serde_json::from_value` to extract typed data,
//! so field-level typing still goes through ordinary serde derive.
//!
//! Nesting depth is bounded by `serde_json`'s own recursion guard (128
//! levels by default): that limit is enforced inside the deserializer's own
//! object/array token parsing, independent of which `Visitor` is driving
//! it, so it applies here the same as it would to any other visitor.
//!
//! Breadth is bounded here, by [`limits::MAX_JSON_NODES`]: depth and input
//! length together still permit a compact input to expand into a `Value`
//! tree an order of magnitude larger than the input. The budget covers
//! every document this crate strict-parses -- bundle, in-toto statement,
//! trusted root, and Rekor canonicalized body -- because unknown fields
//! are tolerated in all of them, so the oversized part of the tree need
//! not be anything the caller ever reads.

use std::cell::Cell;

use serde::de::{DeserializeSeed, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};

use crate::error::{Error, ParseError, ResourceLimitError};
use crate::limits;

/// Parses `bytes` as JSON, rejecting duplicate object keys at any nesting
/// level, rejecting documents over [`limits::MAX_JSON_NODES`] value nodes,
/// and rejecting trailing bytes after the JSON value. Returns a plain
/// [`serde_json::Value`] on success, for further typed extraction via
/// `serde_json::from_value`.
pub(crate) fn parse_strict(bytes: &[u8]) -> Result<serde_json::Value, Error> {
    let nodes = Cell::new(0_usize);
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let value = de
        .deserialize_any(StrictJsonVisitor { nodes: &nodes })
        .map_err(|e| {
            // Serde error payloads are strings, so the budget cell -- not the
            // returned error -- is what distinguishes budget exhaustion from
            // an ordinary parse failure.
            if nodes.get() > limits::MAX_JSON_NODES {
                Error::ResourceLimit(ResourceLimitError::TooManyJsonNodes {
                    limit: limits::MAX_JSON_NODES,
                })
            } else {
                Error::Parse(ParseError::Json(e.to_string()))
            }
        })?;
    de.end()
        .map_err(|e| Error::Parse(ParseError::Json(e.to_string())))?;
    Ok(value)
}

/// [`Visitor`] that rebuilds a [`serde_json::Value`] while checking for
/// duplicate object keys and charging every produced node against a shared
/// budget. Also implements [`DeserializeSeed`] (delegating to
/// `deserialize_any`) so it can be used as the seed for recursive
/// array/map elements, which is what makes both the duplicate-key check
/// and the node budget apply at every nesting level rather than just the
/// top one.
#[derive(Clone, Copy)]
struct StrictJsonVisitor<'a> {
    /// Nodes produced so far, shared across the whole parse. Counts up so
    /// that `parse_strict` can tell an exhausted budget apart from any
    /// other deserialization failure after the fact.
    nodes: &'a Cell<usize>,
}

impl StrictJsonVisitor<'_> {
    /// Charges one produced [`serde_json::Value`] against the budget.
    fn charge<E>(self) -> Result<(), E>
    where
        E: serde::de::Error,
    {
        let used = self.nodes.get().saturating_add(1);
        self.nodes.set(used);
        if used > limits::MAX_JSON_NODES {
            return Err(E::custom(format!(
                "JSON document exceeds the limit of {} value nodes",
                limits::MAX_JSON_NODES
            )));
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for StrictJsonVisitor<'_> {
    type Value = serde_json::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for StrictJsonVisitor<'_> {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        serde_json::Number::from_f64(v).map_or_else(
            || Err(E::custom("JSON number is not finite")),
            |n| Ok(serde_json::Value::Number(n)),
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::String(v.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.charge()?;
        Ok(serde_json::Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.charge()?;
        let mut out = Vec::new();
        while let Some(elem) = seq.next_element_seed(self)? {
            out.push(elem);
        }
        Ok(serde_json::Value::Array(out))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.charge()?;
        let mut out = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if out.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object key `{key}`"
                )));
            }
            let value = map.next_value_seed(self)?;
            out.insert(key, value);
        }
        Ok(serde_json::Value::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_strict;
    use crate::error::{Error, ParseError, ResourceLimitError};
    use crate::limits;

    #[test]
    fn accepts_well_formed_json() -> Result<(), Box<dyn std::error::Error>> {
        let value = parse_strict(br#"{"a":1,"b":[1,2,{"c":true}],"d":null,"e":1.5}"#)?;
        if value["a"] != 1 {
            return Err("field a did not round-trip".into());
        }
        if value["b"][2]["c"] != true {
            return Err("nested field c did not round-trip".into());
        }
        if !value["d"].is_null() {
            return Err("field d should be null".into());
        }
        Ok(())
    }

    #[test]
    fn rejects_top_level_duplicate_key() -> Result<(), Box<dyn std::error::Error>> {
        match parse_strict(br#"{"a":1,"a":2}"#) {
            Err(Error::Parse(ParseError::Json(_))) => Ok(()),
            Err(other) => Err(format!("expected ParseError::Json, got {other:?}").into()),
            Ok(_) => Err("expected duplicate key to be rejected".into()),
        }
    }

    #[test]
    fn rejects_nested_object_duplicate_key() -> Result<(), Box<dyn std::error::Error>> {
        match parse_strict(br#"{"a":{"b":1,"b":2}}"#) {
            Err(Error::Parse(ParseError::Json(_))) => Ok(()),
            Err(other) => Err(format!("expected ParseError::Json, got {other:?}").into()),
            Ok(_) => Err("expected nested duplicate key to be rejected".into()),
        }
    }

    #[test]
    fn rejects_duplicate_key_inside_array_element() -> Result<(), Box<dyn std::error::Error>> {
        match parse_strict(br#"{"a":[{"b":1,"b":2}]}"#) {
            Err(Error::Parse(ParseError::Json(_))) => Ok(()),
            Err(other) => Err(format!("expected ParseError::Json, got {other:?}").into()),
            Ok(_) => Err("expected duplicate key inside array element to be rejected".into()),
        }
    }

    #[test]
    fn rejects_trailing_garbage() -> Result<(), Box<dyn std::error::Error>> {
        match parse_strict(br#"{"a":1} garbage"#) {
            Err(Error::Parse(ParseError::Json(_))) => Ok(()),
            Err(other) => Err(format!("expected ParseError::Json, got {other:?}").into()),
            Ok(_) => Err("expected trailing garbage to be rejected".into()),
        }
    }

    #[test]
    fn rejects_truncated_json() -> Result<(), Box<dyn std::error::Error>> {
        match parse_strict(br#"{"a":"#) {
            Err(Error::Parse(ParseError::Json(_))) => Ok(()),
            Err(other) => Err(format!("expected ParseError::Json, got {other:?}").into()),
            Ok(_) => Err("expected truncated JSON to be rejected".into()),
        }
    }

    /// Returns `[0,0,...]` with `elements` elements.
    fn zero_array_json(elements: usize) -> String {
        let mut out = String::with_capacity(elements * 2 + 2);
        out.push('[');
        for i in 0..elements {
            if i > 0 {
                out.push(',');
            }
            out.push('0');
        }
        out.push(']');
        out
    }

    #[test]
    fn accepts_array_at_the_node_budget() -> Result<(), Box<dyn std::error::Error>> {
        // The array itself is one node, so MAX_JSON_NODES - 1 elements fit.
        let json = zero_array_json(limits::MAX_JSON_NODES - 1);
        let value = parse_strict(json.as_bytes())?;
        let Some(array) = value.as_array() else {
            return Err("expected an array".into());
        };
        if array.len() != limits::MAX_JSON_NODES - 1 {
            return Err(
                format!("expected a full-budget array, got {} elements", array.len()).into(),
            );
        }
        Ok(())
    }

    #[test]
    fn rejects_array_over_the_node_budget() -> Result<(), Box<dyn std::error::Error>> {
        let json = zero_array_json(limits::MAX_JSON_NODES);
        match parse_strict(json.as_bytes()) {
            Err(Error::ResourceLimit(ResourceLimitError::TooManyJsonNodes { limit }))
                if limit == limits::MAX_JSON_NODES =>
            {
                Ok(())
            }
            other => Err(format!("expected TooManyJsonNodes, got {other:?}").into()),
        }
    }

    #[test]
    fn counts_nodes_nested_inside_objects() -> Result<(), Box<dyn std::error::Error>> {
        // One object per element plus the outer array: an element count of
        // half the budget already exceeds it.
        let elements = limits::MAX_JSON_NODES / 2;
        let mut json = String::from("[");
        for i in 0..elements {
            if i > 0 {
                json.push(',');
            }
            json.push_str("{\"a\":0}");
        }
        json.push(']');

        match parse_strict(json.as_bytes()) {
            Err(Error::ResourceLimit(ResourceLimitError::TooManyJsonNodes { limit }))
                if limit == limits::MAX_JSON_NODES =>
            {
                Ok(())
            }
            other => Err(format!("expected TooManyJsonNodes, got {other:?}").into()),
        }
    }
}
