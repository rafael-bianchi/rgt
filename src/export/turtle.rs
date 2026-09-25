//! Deterministic, bounded Turtle export of the selected project's provenance.

use crate::store::queries::get_or_create_rdf_project_id;
use crate::types::{NodeType, TrackedNode, ValueData, ValueKind};
use crate::verify::valid_operation;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::{Connection, Row};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::time::Instant;

pub const MAX_VALUES: usize = 10_000;
pub const MAX_EDGES: usize = 30_000;
pub const MAX_TURTLE_BYTES: usize = 67_108_864;
pub const IRI_BASE: &str = "https://github.com/rafael-bianchi/rgt/iri/v1/";
pub const VOCAB: &str = "https://github.com/rafael-bianchi/rgt/vocab#";
pub const CAPTURE_AGENTS: [&str; 8] = [
    "claude-code",
    "cursor",
    "copilot",
    "gemini",
    "vibe",
    "opencode",
    "pi",
    "hermes",
];

pub fn capture_agent_supported(agent: &str) -> bool {
    CAPTURE_AGENTS.contains(&agent) && crate::hooks::installer::is_capture_capable_agent(agent)
}

#[derive(Debug, Clone)]
pub struct ExportNode {
    pub id: String,
    pub node_type: NodeType,
    pub value: ValueData,
    pub source_doc_id: Option<i64>,
    pub line_number: Option<u32>,
    pub is_stale: bool,
}

#[derive(Debug, Clone)]
pub struct ExportSource {
    pub id: i64,
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct ExportEdge {
    pub id: i64,
    pub parent_node_id: String,
    pub child_node_id: String,
    pub operation_type: String,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureAssociation {
    pub node_id: String,
    pub agent_name: String,
}

#[derive(Debug, Clone)]
pub struct ExportSnapshot {
    pub project_id: String,
    pub nodes: Vec<ExportNode>,
    pub sources: Vec<ExportSource>,
    pub edges: Vec<ExportEdge>,
    pub captures: Vec<CaptureAssociation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub relative_path: Option<String>,
    pub absolute_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationActivity {
    pub parent_ids: Vec<String>,
    pub operation_type: String,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedTurtle {
    pub turtle: String,
    pub ambiguous_children: Vec<String>,
}

/// Reads one strict, consistent export snapshot. The project token is created
/// before the read transaction so its insertion cannot upgrade a read lock.
pub fn load_snapshot(conn: &Connection, deadline: Instant) -> Result<ExportSnapshot, String> {
    check_deadline(deadline)?;
    let project_id = get_or_create_rdf_project_id(conn)
        .map_err(|e| format!("failed to read the RDF project identity: {e}"))?;

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("failed to begin the Turtle export snapshot: {e}"))?;
    preflight_limit(
        &tx,
        "tracked_nodes",
        MAX_VALUES,
        "recorded values",
        deadline,
    )?;
    preflight_limit(
        &tx,
        "derivation_edges",
        MAX_EDGES,
        "derivation edges",
        deadline,
    )?;

    let nodes = load_nodes(&tx, deadline)?;
    let node_ids: HashSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
    let sources = load_sources(&tx, deadline)?;
    let source_ids: HashSet<_> = sources.iter().map(|source| source.id).collect();
    for node in &nodes {
        if let Some(source_id) = node.source_doc_id {
            if !source_ids.contains(&source_id) {
                return Err(format!(
                    "root value '{}' references missing source document {}",
                    node.id, source_id
                ));
            }
        }
    }

    let edges = load_edges(&tx, deadline)?;
    for edge in &edges {
        if !node_ids.contains(edge.parent_node_id.as_str()) {
            return Err(format!(
                "derivation edge {} references missing parent '{}'",
                edge.id, edge.parent_node_id
            ));
        }
        if !node_ids.contains(edge.child_node_id.as_str()) {
            return Err(format!(
                "derivation edge {} references missing child '{}'",
                edge.id, edge.child_node_id
            ));
        }
    }

    let captures = load_captures(&tx, &node_ids, deadline)?;
    tx.commit()
        .map_err(|e| format!("failed to finish the Turtle export snapshot: {e}"))?;
    check_deadline(deadline)?;

    Ok(ExportSnapshot {
        project_id,
        nodes,
        sources,
        edges,
        captures,
    })
}

fn preflight_limit(
    conn: &Connection,
    table: &str,
    limit: usize,
    label: &str,
    deadline: Instant,
) -> Result<(), String> {
    let sql = format!("SELECT 1 FROM {table} LIMIT ?1");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("failed to check Turtle {label} limit: {e}"))?;
    let mut rows = stmt
        .query([limit as i64 + 1])
        .map_err(|e| format!("failed to check Turtle {label} limit: {e}"))?;
    let mut count = 0usize;
    while rows
        .next()
        .map_err(|e| format!("failed to check Turtle {label} limit: {e}"))?
        .is_some()
    {
        count += 1;
        if count > limit {
            return Err(format!(
                "Turtle export supports at most {limit} {label}; found more"
            ));
        }
        if count & 255 == 0 {
            check_deadline(deadline)?;
        }
    }
    check_deadline(deadline)
}

fn load_nodes(conn: &Connection, deadline: Instant) -> Result<Vec<ExportNode>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, node_type, value_kind, number_val, date_val, duration_secs,
                    source_doc_id, line_number, is_stale
             FROM tracked_nodes ORDER BY id",
        )
        .map_err(|e| format!("failed to prepare Turtle value snapshot: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("failed to read Turtle value snapshot: {e}"))?;
    let mut nodes = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("failed to read Turtle value snapshot: {e}"))?
    {
        if nodes.len() % 256 == 0 {
            check_deadline(deadline)?;
        }
        nodes.push(parse_node(row)?);
    }
    Ok(nodes)
}

fn parse_node(row: &Row<'_>) -> Result<ExportNode, String> {
    let id: String = row.get(0).map_err(|e| format!("invalid value ID: {e}"))?;
    if id.is_empty() {
        return Err("tracked value has an empty ID".into());
    }
    let node_type_text: String = row
        .get(1)
        .map_err(|e| format!("value '{id}' has invalid node type: {e}"))?;
    let node_type = NodeType::from_str(&node_type_text)
        .map_err(|_| format!("value '{id}' has unsupported node type '{node_type_text}'"))?;
    let kind_text: String = row
        .get(2)
        .map_err(|e| format!("value '{id}' has invalid value kind: {e}"))?;
    let kind = ValueKind::from_str(&kind_text)
        .map_err(|_| format!("value '{id}' has unsupported value kind '{kind_text}'"))?;
    let number: Option<f64> = row
        .get(3)
        .map_err(|e| format!("value '{id}' has invalid number: {e}"))?;
    let date: Option<String> = row
        .get(4)
        .map_err(|e| format!("value '{id}' has invalid date: {e}"))?;
    let seconds: Option<i64> = row
        .get(5)
        .map_err(|e| format!("value '{id}' has invalid duration: {e}"))?;
    let value = match kind {
        ValueKind::Number => {
            if date.is_some() || seconds.is_some() {
                return Err(format!("number value '{id}' has conflicting typed columns"));
            }
            let number =
                number.ok_or_else(|| format!("number value '{id}' is missing number_val"))?;
            if !number.is_finite() {
                return Err(format!("number value '{id}' is not finite"));
            }
            ValueData::Number(number)
        }
        ValueKind::Date => {
            if number.is_some() || seconds.is_some() {
                return Err(format!("date value '{id}' has conflicting typed columns"));
            }
            let date = date.ok_or_else(|| format!("date value '{id}' is missing date_val"))?;
            let parsed = DateTime::parse_from_rfc3339(&date)
                .map_err(|e| format!("date value '{id}' is invalid: {e}"))?;
            ValueData::Date(parsed.with_timezone(&Utc))
        }
        ValueKind::Duration => {
            if number.is_some() || date.is_some() {
                return Err(format!(
                    "duration value '{id}' has conflicting typed columns"
                ));
            }
            let seconds =
                seconds.ok_or_else(|| format!("duration value '{id}' is missing duration_secs"))?;
            let duration = Duration::try_seconds(seconds).ok_or_else(|| {
                format!("duration value '{id}' has duration_secs '{seconds}' out of range")
            })?;
            ValueData::Duration(duration)
        }
    };
    let source_doc_id: Option<i64> = row
        .get(6)
        .map_err(|e| format!("value '{id}' has invalid source ID: {e}"))?;
    let line: Option<i64> = row
        .get(7)
        .map_err(|e| format!("value '{id}' has invalid source line: {e}"))?;
    let line_number = match line {
        Some(line) if line > 0 => Some(
            u32::try_from(line)
                .map_err(|_| format!("value '{id}' has out-of-range source line"))?,
        ),
        Some(_) => return Err(format!("value '{id}' has a non-positive source line")),
        None => None,
    };
    if matches!(node_type, NodeType::Derived) && (source_doc_id.is_some() || line_number.is_some())
    {
        return Err(format!(
            "derived value '{id}' unexpectedly has a source location"
        ));
    }
    let stale: i64 = row
        .get(8)
        .map_err(|e| format!("value '{id}' has invalid stale state: {e}"))?;
    let is_stale = match stale {
        0 => false,
        1 => true,
        _ => return Err(format!("value '{id}' has invalid stale state '{stale}'")),
    };

    Ok(ExportNode {
        id,
        node_type,
        value,
        source_doc_id,
        line_number,
        is_stale,
    })
}

fn load_sources(conn: &Connection, deadline: Instant) -> Result<Vec<ExportSource>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, file_path FROM source_documents
             WHERE id IN (SELECT DISTINCT source_doc_id FROM tracked_nodes WHERE source_doc_id IS NOT NULL)
             ORDER BY id",
        )
        .map_err(|e| format!("failed to prepare Turtle source snapshot: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ExportSource {
                id: row.get(0)?,
                file_path: row.get(1)?,
            })
        })
        .map_err(|e| format!("failed to read Turtle source snapshot: {e}"))?;
    let mut sources = Vec::new();
    for source in rows {
        check_deadline(deadline)?;
        sources.push(source.map_err(|e| format!("failed to decode Turtle source snapshot: {e}"))?);
    }
    Ok(sources)
}

fn load_edges(conn: &Connection, deadline: Instant) -> Result<Vec<ExportEdge>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, parent_node_id, child_node_id, operation_type, expression
             FROM derivation_edges ORDER BY id",
        )
        .map_err(|e| format!("failed to prepare Turtle derivation snapshot: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ExportEdge {
                id: row.get(0)?,
                parent_node_id: row.get(1)?,
                child_node_id: row.get(2)?,
                operation_type: row.get(3)?,
                expression: row.get(4)?,
            })
        })
        .map_err(|e| format!("failed to read Turtle derivation snapshot: {e}"))?;
    let mut edges = Vec::new();
    for edge in rows {
        if edges.len() % 256 == 0 {
            check_deadline(deadline)?;
        }
        edges.push(edge.map_err(|e| format!("failed to decode Turtle derivation snapshot: {e}"))?);
    }
    Ok(edges)
}

fn load_captures(
    conn: &Connection,
    node_ids: &HashSet<&str>,
    deadline: Instant,
) -> Result<Vec<CaptureAssociation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT node_id, agent_name FROM capture_associations ORDER BY node_id, agent_name",
        )
        .map_err(|e| format!("failed to prepare Turtle capture snapshot: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CaptureAssociation {
                node_id: row.get(0)?,
                agent_name: row.get(1)?,
            })
        })
        .map_err(|e| format!("failed to read Turtle capture snapshot: {e}"))?;
    let mut captures = Vec::new();
    let mut seen = HashSet::new();
    for capture in rows {
        check_deadline(deadline)?;
        let capture =
            capture.map_err(|e| format!("failed to decode Turtle capture snapshot: {e}"))?;
        if !node_ids.contains(capture.node_id.as_str()) {
            return Err(format!(
                "capture association references missing value '{}'",
                capture.node_id
            ));
        }
        if !capture_agent_supported(&capture.agent_name) {
            return Err(format!(
                "capture association for '{}' has unsupported agent '{}'",
                capture.node_id, capture.agent_name
            ));
        }
        if !seen.insert((capture.node_id.clone(), capture.agent_name.clone())) {
            return Err(format!(
                "duplicate capture association for '{}' and '{}'",
                capture.node_id, capture.agent_name
            ));
        }
        captures.push(capture);
    }
    Ok(captures)
}

pub fn check_deadline(deadline: Instant) -> Result<(), String> {
    if Instant::now() >= deadline {
        Err("Turtle export exceeded its one-second command deadline".into())
    } else {
        Ok(())
    }
}

/// Name registered by the `graph --format` CLI.
pub fn format_name() -> &'static str {
    "ttl"
}

fn valid_project_token(token: &str) -> bool {
    token.len() == 32
        && token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub fn project_iri(token: &str) -> Result<String, String> {
    if !valid_project_token(token) {
        return Err(
            "invalid RDF project token: expected 32 lowercase hexadecimal characters".into(),
        );
    }
    Ok(format!("{IRI_BASE}project/{token}"))
}

pub fn encode_iri_component(component: &str) -> String {
    let mut encoded = String::with_capacity(component.len());
    for byte in component.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(*byte as char);
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

pub fn node_iri(token: &str, node_id: &str) -> Result<String, String> {
    Ok(format!(
        "{}/node/{}",
        project_iri(token)?,
        encode_iri_component(node_id)
    ))
}

pub fn source_iri(token: &str, source_id: i64) -> Result<String, String> {
    if source_id < 0 {
        return Err("invalid negative source document ID".into());
    }
    Ok(format!("{}/source/{source_id}", project_iri(token)?))
}

pub fn derivation_activity_iri(token: &str, child_id: &str) -> Result<String, String> {
    Ok(format!(
        "{}/activity/derive/{}",
        project_iri(token)?,
        encode_iri_component(child_id)
    ))
}

pub fn qualified_usage_iri(token: &str, child_id: &str, index: usize) -> Result<String, String> {
    Ok(format!(
        "{}/usage/derive/{}/{index}",
        project_iri(token)?,
        encode_iri_component(child_id)
    ))
}

pub fn capture_activity_iri(token: &str, node_id: &str, agent: &str) -> Result<String, String> {
    if !capture_agent_supported(agent) {
        return Err(format!("unsupported capture agent '{agent}'"));
    }
    Ok(format!(
        "{}/activity/capture/{}/{}",
        project_iri(token)?,
        encode_iri_component(node_id),
        agent
    ))
}

pub fn agent_iri(agent: &str) -> String {
    format!("{IRI_BASE}agent/{agent}")
}

#[allow(dead_code)]
pub fn escape_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0008}' => escaped.push_str("\\b"),
            '\u{000C}' => escaped.push_str("\\f"),
            ch if ch.is_control() && (ch as u32) <= 0xffff => {
                let _ = write!(escaped, "\\u{:04X}", ch as u32);
            }
            ch if ch.is_control() => {
                let _ = write!(escaped, "\\U{:08X}", ch as u32);
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

struct TurtleLiteral<'a>(&'a str);

impl std::fmt::Display for TurtleLiteral<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut plain_start = 0;
        for (index, ch) in self.0.char_indices() {
            if !ch.is_control() && !matches!(ch, '\\' | '"') {
                continue;
            }
            formatter.write_str(&self.0[plain_start..index])?;
            match ch {
                '\\' => formatter.write_str("\\\\")?,
                '"' => formatter.write_str("\\\"")?,
                '\n' => formatter.write_str("\\n")?,
                '\r' => formatter.write_str("\\r")?,
                '\t' => formatter.write_str("\\t")?,
                '\u{0008}' => formatter.write_str("\\b")?,
                '\u{000C}' => formatter.write_str("\\f")?,
                ch if ch.is_control() && (ch as u32) <= 0xffff => {
                    write!(formatter, "\\u{:04X}", ch as u32)?;
                }
                ch if ch.is_control() => {
                    write!(formatter, "\\U{:08X}", ch as u32)?;
                }
                _ => unreachable!(),
            }
            plain_start = index + ch.len_utf8();
        }
        formatter.write_str(&self.0[plain_start..])?;
        Ok(())
    }
}

pub fn xsd_duration_lexical(seconds: i64) -> String {
    if seconds == 0 {
        return "PT0S".into();
    }
    let sign = if seconds < 0 { "-" } else { "" };
    let mut remainder = seconds.unsigned_abs();
    let days = remainder / 86_400;
    remainder %= 86_400;
    let hours = remainder / 3_600;
    remainder %= 3_600;
    let minutes = remainder / 60;
    let secs = remainder % 60;
    let mut lexical = format!("{sign}P");
    if days > 0 {
        let _ = write!(lexical, "{days}D");
    }
    if hours > 0 || minutes > 0 || secs > 0 || days == 0 {
        lexical.push('T');
        if hours > 0 {
            let _ = write!(lexical, "{hours}H");
        }
        if minutes > 0 {
            let _ = write!(lexical, "{minutes}M");
        }
        if secs > 0 {
            let _ = write!(lexical, "{secs}S");
        }
    }
    lexical
}

pub fn source_location(
    stored_path: &str,
    project_root: &Path,
    include_absolute: bool,
) -> SourceLocation {
    let project = match project_root.canonicalize() {
        Ok(project) => project,
        Err(_) => {
            return SourceLocation {
                relative_path: None,
                absolute_path: None,
            }
        }
    };
    let path = Path::new(stored_path);
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project.join(path)
    };
    let canonical = candidate.canonicalize().ok();
    let relative_path = canonical.as_ref().and_then(|canonical| {
        canonical
            .strip_prefix(&project)
            .ok()
            .map(path_to_turtle_string)
    });
    let absolute_path =
        include_absolute.then(|| path_to_turtle_string(canonical.as_deref().unwrap_or(&candidate)));
    SourceLocation {
        relative_path,
        absolute_path,
    }
}

fn path_to_turtle_string(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    }
}

struct BoundedTurtleWriter<'a> {
    output: &'a mut String,
    byte_limit: usize,
    deadline: Instant,
    failure: Option<String>,
}

impl FmtWrite for BoundedTurtleWriter<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if let Err(error) = check_deadline(self.deadline) {
            self.failure = Some(error);
            return Err(std::fmt::Error);
        }
        if self.output.len().saturating_add(text.len()) > self.byte_limit {
            self.failure = Some(format!(
                "Turtle export exceeds the {}-byte output limit",
                self.byte_limit
            ));
            return Err(std::fmt::Error);
        }
        self.output.push_str(text);
        Ok(())
    }
}

fn append_turtle(
    output: &mut String,
    arguments: std::fmt::Arguments<'_>,
    byte_limit: usize,
    deadline: Instant,
) -> Result<(), String> {
    let mut writer = BoundedTurtleWriter {
        output,
        byte_limit,
        deadline,
        failure: None,
    };
    match std::fmt::write(&mut writer, arguments) {
        Ok(()) => Ok(()),
        Err(_) => Err(writer
            .failure
            .unwrap_or_else(|| "failed to format Turtle output".into())),
    }
}

#[allow(dead_code)]
pub fn classify_derivation(
    child: &ExportNode,
    all_edges: &[ExportEdge],
    nodes: &HashMap<String, ExportNode>,
) -> Option<DerivationActivity> {
    let mut edges = all_edges
        .iter()
        .filter(|edge| edge.child_node_id == child.id)
        .collect::<Vec<_>>();
    edges.sort_by_key(|edge| edge.id);
    classify_derivation_group(child, &edges, nodes)
}

fn classify_derivation_group(
    child: &ExportNode,
    edges: &[&ExportEdge],
    nodes: &HashMap<String, ExportNode>,
) -> Option<DerivationActivity> {
    if child.node_type != NodeType::Derived {
        return None;
    }
    let first = *edges.first()?;
    let operation_type = first.operation_type.clone();
    let expression = first.expression.clone();
    if !valid_operation(&operation_type) {
        return None;
    }
    if edges
        .iter()
        .any(|edge| edge.operation_type != operation_type || edge.expression != expression)
    {
        return None;
    }
    let mut parent_ids = Vec::with_capacity(edges.len());
    let mut unique = HashSet::new();
    for edge in edges {
        if !unique.insert(edge.parent_node_id.as_str()) || !nodes.contains_key(&edge.parent_node_id)
        {
            return None;
        }
        parent_ids.push(edge.parent_node_id.clone());
    }
    let complete_id = TrackedNode::generate_derived_id(&parent_ids, &operation_type, &child.value);
    let id_matches = child.id == complete_id
        || (complete_id.starts_with("node_drv_")
            && child.id.len() == "node_drv_".len() + 12
            && child.id == complete_id[.."node_drv_".len() + 12]);
    if !id_matches {
        return None;
    }
    Some(DerivationActivity {
        parent_ids,
        operation_type,
        expression,
    })
}

pub fn render_turtle(
    snapshot: &ExportSnapshot,
    project_root: &Path,
    include_absolute_paths: bool,
    deadline: Instant,
) -> Result<RenderedTurtle, String> {
    render_turtle_with_limit(
        snapshot,
        project_root,
        include_absolute_paths,
        MAX_TURTLE_BYTES,
        deadline,
    )
}

pub fn render_turtle_with_limit(
    snapshot: &ExportSnapshot,
    project_root: &Path,
    include_absolute_paths: bool,
    byte_limit: usize,
    deadline: Instant,
) -> Result<RenderedTurtle, String> {
    let project = project_iri(&snapshot.project_id)?;
    let namespace = format!("{project}/");
    let mut output = String::new();
    let ambiguous_children = {
        let mut emit = |arguments: std::fmt::Arguments<'_>| -> Result<(), String> {
            append_turtle(&mut output, arguments, byte_limit, deadline)
        };

        emit(format_args!("@prefix prov: <http://www.w3.org/ns/prov#> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n@prefix rgt: <{VOCAB}> .\n\n"))?;
        if !snapshot.nodes.is_empty()
            || !snapshot.sources.is_empty()
            || !snapshot.captures.is_empty()
        {
            emit(format_args!("<{project}> a rgt:Project .\n"))?;
        }

        let sources: HashMap<_, _> = snapshot
            .sources
            .iter()
            .map(|source| (source.id, source))
            .collect();
        let nodes: HashMap<_, _> = snapshot
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.clone()))
            .collect();
        let mut sorted_sources = snapshot.sources.iter().collect::<Vec<_>>();
        sorted_sources.sort_by_key(|source| source.id);
        let mut sorted_nodes = snapshot.nodes.iter().collect::<Vec<_>>();
        sorted_nodes.sort_by(|left, right| left.id.cmp(&right.id));
        let mut sorted_edges = snapshot.edges.iter().collect::<Vec<_>>();
        sorted_edges.sort_by_key(|edge| edge.id);
        let mut sorted_captures = snapshot.captures.iter().collect::<Vec<_>>();
        sorted_captures.sort_by(|left, right| {
            (&left.node_id, &left.agent_name).cmp(&(&right.node_id, &right.agent_name))
        });
        let mut ambiguous_children = Vec::new();

        for source in sorted_sources {
            let iri = source_iri(&snapshot.project_id, source.id)?;
            ensure_project_instance(&iri, &namespace)?;
            let location = source_location(&source.file_path, project_root, include_absolute_paths);
            emit(format_args!(
            "<{iri}> a prov:Entity, rgt:SourceDocument .\n<{iri}> rgt:inProject <{project}> .\n"
        ))?;
            if let Some(relative) = location.relative_path {
                emit(format_args!(
                    "<{iri}> rgt:relativePath \"{}\" .\n",
                    TurtleLiteral(&relative)
                ))?;
            }
            if let Some(absolute) = location.absolute_path {
                emit(format_args!(
                    "<{iri}> rgt:absolutePath \"{}\" .\n",
                    TurtleLiteral(&absolute)
                ))?;
            }
        }

        for node in &sorted_nodes {
            let iri = node_iri(&snapshot.project_id, &node.id)?;
            ensure_project_instance(&iri, &namespace)?;
            emit(format_args!(
                "<{iri}> a prov:Entity .\n<{iri}> rgt:inProject <{project}> .\n"
            ))?;
            emit(format_args!(
                "<{iri}> rgt:localNodeId \"{}\" .\n",
                TurtleLiteral(&node.id)
            ))?;
            emit(format_args!(
                "<{iri}> rgt:valueKind \"{}\" .\n",
                node.value.kind()
            ))?;
            let value_literal = match &node.value {
                ValueData::Number(value) => format!("\"{:?}\"^^xsd:double", value),
                ValueData::Date(value) => format!(
                    "\"{}\"^^xsd:dateTime",
                    value.to_rfc3339_opts(SecondsFormat::AutoSi, true)
                ),
                ValueData::Duration(value) => {
                    format!(
                        "\"{}\"^^xsd:duration",
                        xsd_duration_lexical(value.num_seconds())
                    )
                }
            };
            emit(format_args!("<{iri}> rgt:value {value_literal} .\n"))?;
            emit(format_args!(
                "<{iri}> rgt:isStale \"{}\"^^xsd:boolean .\n",
                node.is_stale
            ))?;
            if let Some(line) = node.line_number {
                emit(format_args!(
                    "<{iri}> rgt:lineNumber \"{line}\"^^xsd:integer .\n"
                ))?;
            }
            if let Some(source_id) = node.source_doc_id {
                let source = sources.get(&source_id).ok_or_else(|| {
                    format!(
                        "root value '{}' references missing source document {source_id}",
                        node.id
                    )
                })?;
                let source_resource = source_iri(&snapshot.project_id, source.id)?;
                ensure_project_instance(&source_resource, &namespace)?;
                emit(format_args!(
                    "<{iri}> prov:wasDerivedFrom <{source_resource}> .\n"
                ))?;
            }
        }

        for edge in &sorted_edges {
            let child = node_iri(&snapshot.project_id, &edge.child_node_id)?;
            let parent = node_iri(&snapshot.project_id, &edge.parent_node_id)?;
            ensure_project_instance(&child, &namespace)?;
            ensure_project_instance(&parent, &namespace)?;
            emit(format_args!("<{child}> prov:wasDerivedFrom <{parent}> .\n"))?;
        }

        let mut edges_by_child: HashMap<&str, Vec<&ExportEdge>> = HashMap::new();
        for (index, edge) in sorted_edges.iter().enumerate() {
            if index % 256 == 0 {
                check_deadline(deadline)?;
            }
            edges_by_child
                .entry(edge.child_node_id.as_str())
                .or_default()
                .push(edge);
        }
        let mut supported = Vec::new();
        for (index, child) in sorted_nodes.iter().enumerate() {
            if index % 256 == 0 {
                check_deadline(deadline)?;
            }
            if child.node_type != NodeType::Derived {
                continue;
            }
            let incoming = edges_by_child
                .get(child.id.as_str())
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            match classify_derivation_group(child, incoming, &nodes) {
                Some(activity) => supported.push((child, activity)),
                None => {
                    ambiguous_children.push(child.id.clone());
                }
            }
        }
        for (child, activity) in supported {
            let activity_resource = derivation_activity_iri(&snapshot.project_id, &child.id)?;
            ensure_project_instance(&activity_resource, &namespace)?;
            emit(format_args!("<{activity_resource}> a prov:Activity .\n<{activity_resource}> rgt:inProject <{project}> .\n"))?;
            emit(format_args!(
                "<{activity_resource}> rgt:operationType \"{}\" .\n",
                TurtleLiteral(&activity.operation_type)
            ))?;
            if let Some(expression) = activity.expression {
                emit(format_args!(
                    "<{activity_resource}> rgt:expression \"{}\" .\n",
                    TurtleLiteral(&expression)
                ))?;
            }
            let child_resource = node_iri(&snapshot.project_id, &child.id)?;
            emit(format_args!(
                "<{child_resource}> prov:wasGeneratedBy <{activity_resource}> .\n"
            ))?;
            for (index, parent_id) in activity.parent_ids.iter().enumerate() {
                let parent = node_iri(&snapshot.project_id, parent_id)?;
                emit(format_args!(
                    "<{activity_resource}> prov:used <{parent}> .\n"
                ))?;
                let usage = qualified_usage_iri(&snapshot.project_id, &child.id, index)?;
                ensure_project_instance(&usage, &namespace)?;
                emit(format_args!(
                    "<{activity_resource}> prov:qualifiedUsage <{usage}> .\n"
                ))?;
                emit(format_args!("<{usage}> a prov:Usage .\n<{usage}> prov:entity <{parent}> .\n<{usage}> rgt:operandIndex \"{index}\"^^xsd:integer .\n"))?;
            }
        }

        let mut agents = HashSet::new();
        for capture in sorted_captures {
            let activity =
                capture_activity_iri(&snapshot.project_id, &capture.node_id, &capture.agent_name)?;
            let value = node_iri(&snapshot.project_id, &capture.node_id)?;
            let agent = agent_iri(&capture.agent_name);
            ensure_project_instance(&activity, &namespace)?;
            emit(format_args!(
                "<{activity}> a prov:Activity .\n<{activity}> rgt:inProject <{project}> .\n"
            ))?;
            emit(format_args!(
            "<{activity}> prov:used <{value}> .\n<{activity}> prov:wasAssociatedWith <{agent}> .\n"
        ))?;
            agents.insert(capture.agent_name.as_str());
        }
        let mut agents = agents.into_iter().collect::<Vec<_>>();
        agents.sort_unstable();
        for agent_name in agents {
            let agent = agent_iri(agent_name);
            emit(format_args!(
                "<{agent}> a prov:Agent .\n<{agent}> rgt:agentName \"{agent_name}\" .\n"
            ))?;
        }
        check_deadline(deadline)?;
        ambiguous_children
    };
    Ok(RenderedTurtle {
        turtle: output,
        ambiguous_children,
    })
}

/// Completes the final output write on a dedicated thread and waits only for
/// the remaining command budget. A timed-out worker is deliberately detached:
/// the CLI exits nonzero immediately, which terminates a worker blocked in an
/// operating-system pipe write.
pub fn write_bytes_before_deadline<W>(
    writer: W,
    bytes: Vec<u8>,
    deadline: Instant,
) -> Result<(), String>
where
    W: Write + Send + 'static,
{
    check_deadline(deadline)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut writer = writer;
        let result = writer
            .write_all(&bytes)
            .and_then(|()| writer.flush())
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    let remaining = deadline.saturating_duration_since(Instant::now());
    match receiver.recv_timeout(remaining) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("failed writing Turtle to stdout: {error}")),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(
            "Turtle export exceeded its one-second command deadline while writing stdout".into(),
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("Turtle stdout writer stopped before reporting completion".into())
        }
    }
}

fn ensure_project_instance(iri: &str, namespace: &str) -> Result<(), String> {
    if iri.starts_with(namespace) {
        Ok(())
    } else {
        Err(format!(
            "RGT instance IRI is outside the current project namespace: {iri}"
        ))
    }
}
