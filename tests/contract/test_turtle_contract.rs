use chrono::{Duration, TimeZone, Utc};
use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;
use rgt::export::turtle::{
    agent_iri, capture_activity_iri, node_iri, project_iri, render_turtle,
    render_turtle_with_limit, ExportEdge, ExportNode, ExportSnapshot, ExportSource,
};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::path::Path;
use std::time::{Duration as StdDuration, Instant};
use tempfile::tempdir;

fn parse(turtle: &str) -> Vec<Triple> {
    TurtleParser::new()
        .for_slice(turtle.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .expect("renderer output must be accepted by an independent Turtle parser")
}

fn has_iri_object(triples: &[Triple], subject: &str, predicate: &str, object: &str) -> bool {
    triples.iter().any(|triple| {
        matches!(&triple.subject, NamedOrBlankNode::NamedNode(node) if node.as_str() == subject)
            && triple.predicate.as_str() == predicate
            && matches!(&triple.object, Term::NamedNode(node) if node.as_str() == object)
    })
}

fn literal_object<'a>(
    triples: &'a [Triple],
    subject: &str,
    predicate: &str,
) -> Option<&'a oxrdf::Literal> {
    triples.iter().find_map(|triple| {
        let subject_matches = matches!(
            &triple.subject,
            NamedOrBlankNode::NamedNode(node) if node.as_str() == subject
        );
        if subject_matches && triple.predicate.as_str() == predicate {
            if let Term::Literal(literal) = &triple.object {
                return Some(literal);
            }
        }
        None
    })
}

fn fixture(project_root: &Path, project_id: &str) -> ExportSnapshot {
    let source_path = project_root.join("input/data.csv");
    std::fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    std::fs::write(&source_path, "amount,3\n").unwrap();
    let root_a = ExportNode {
        id: "root/a with space".into(),
        node_type: NodeType::Root,
        value: ValueData::Number(3.0),
        source_doc_id: Some(1),
        line_number: Some(1),
        is_stale: false,
    };
    let root_b = ExportNode {
        id: "root-b".into(),
        node_type: NodeType::Root,
        value: ValueData::Date(Utc.with_ymd_and_hms(2026, 9, 24, 12, 0, 0).unwrap()),
        source_doc_id: None,
        line_number: None,
        is_stale: false,
    };
    let duration = ExportNode {
        id: "duration-zero".into(),
        node_type: NodeType::Root,
        value: ValueData::Duration(Duration::zero()),
        source_doc_id: None,
        line_number: None,
        is_stale: true,
    };
    let parent_ids = vec![root_a.id.clone(), root_b.id.clone()];
    let child_id =
        TrackedNode::generate_derived_id(&parent_ids, "EXPRESSION", &ValueData::Number(7.0));
    let child = ExportNode {
        id: child_id.clone(),
        node_type: NodeType::Derived,
        value: ValueData::Number(7.0),
        source_doc_id: None,
        line_number: None,
        is_stale: false,
    };
    ExportSnapshot {
        project_id: project_id.into(),
        nodes: vec![root_a.clone(), root_b.clone(), duration, child],
        sources: vec![ExportSource {
            id: 1,
            file_path: "input/data.csv".into(),
        }],
        edges: vec![
            ExportEdge {
                id: 1,
                parent_node_id: root_a.id.clone(),
                child_node_id: child_id.clone(),
                operation_type: "EXPRESSION".into(),
                expression: Some("a + b # \"quoted\"\nélan".into()),
            },
            ExportEdge {
                id: 2,
                parent_node_id: root_b.id,
                child_node_id: child_id,
                operation_type: "EXPRESSION".into(),
                expression: Some("a + b # \"quoted\"\nélan".into()),
            },
        ],
        captures: vec![rgt::export::turtle::CaptureAssociation {
            node_id: root_a.id,
            agent_name: "cursor".into(),
        }],
    }
}

#[test]
fn empty_project_is_a_valid_prefix_only_turtle_document() {
    let root = tempdir().unwrap();
    let snapshot = ExportSnapshot {
        project_id: "0123456789abcdef0123456789abcdef".into(),
        nodes: Vec::new(),
        sources: Vec::new(),
        edges: Vec::new(),
        captures: Vec::new(),
    };
    let output = render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + StdDuration::from_secs(1),
    )
    .unwrap();
    assert!(output.turtle.starts_with("@prefix prov:"));
    assert!(parse(&output.turtle).is_empty());
}

#[test]
fn parsed_graph_preserves_typed_values_lineage_usage_capture_and_safe_paths() {
    let root = tempdir().unwrap();
    let token = "0123456789abcdef0123456789abcdef";
    let snapshot = fixture(root.path(), token);
    let output = render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + StdDuration::from_secs(1),
    )
    .unwrap();
    let triples = parse(&output.turtle);
    let project = project_iri(token).unwrap();
    let source = format!("{project}/source/1");
    let root_a = node_iri(token, "root/a with space").unwrap();
    let root_b = node_iri(token, "root-b").unwrap();
    let duration = node_iri(token, "duration-zero").unwrap();
    let child_id = snapshot
        .nodes
        .iter()
        .find(|node| node.node_type == NodeType::Derived)
        .unwrap()
        .id
        .clone();
    let child = node_iri(token, &child_id).unwrap();
    let activity = format!(
        "{project}/activity/derive/{}",
        rgt::export::turtle::encode_iri_component(&child_id)
    );
    let usage_0 = format!(
        "{project}/usage/derive/{}/0",
        rgt::export::turtle::encode_iri_component(&child_id)
    );
    let usage_1 = format!(
        "{project}/usage/derive/{}/1",
        rgt::export::turtle::encode_iri_component(&child_id)
    );

    assert!(has_iri_object(
        &triples,
        &root_a,
        "http://www.w3.org/ns/prov#wasDerivedFrom",
        &source
    ));
    assert!(has_iri_object(
        &triples,
        &child,
        "http://www.w3.org/ns/prov#wasDerivedFrom",
        &root_a
    ));
    assert!(has_iri_object(
        &triples,
        &child,
        "http://www.w3.org/ns/prov#wasDerivedFrom",
        &root_b,
    ));
    assert!(has_iri_object(
        &triples,
        &activity,
        "http://www.w3.org/ns/prov#used",
        &root_a
    ));
    assert!(has_iri_object(
        &triples,
        &usage_0,
        "http://www.w3.org/ns/prov#entity",
        &root_a
    ));
    assert!(has_iri_object(
        &triples,
        &child,
        "http://www.w3.org/ns/prov#wasGeneratedBy",
        &activity
    ));
    assert!(has_iri_object(
        &triples,
        &activity,
        "http://www.w3.org/ns/prov#qualifiedUsage",
        &usage_0
    ));
    assert!(has_iri_object(
        &triples,
        &usage_1,
        "http://www.w3.org/ns/prov#entity",
        &root_b
    ));
    assert_eq!(
        literal_object(
            &triples,
            &root_a,
            &format!("{}localNodeId", rgt::export::turtle::VOCAB)
        )
        .unwrap()
        .value(),
        "root/a with space"
    );
    assert_eq!(
        literal_object(
            &triples,
            &activity,
            &format!("{}expression", rgt::export::turtle::VOCAB)
        )
        .unwrap()
        .value(),
        "a + b # \"quoted\"\nélan"
    );
    let number = literal_object(
        &triples,
        &root_a,
        &format!("{}value", rgt::export::turtle::VOCAB),
    )
    .unwrap();
    assert_eq!(
        number.datatype().as_str(),
        "http://www.w3.org/2001/XMLSchema#double"
    );
    assert_eq!(number.value(), "3.0");
    let stale = literal_object(
        &triples,
        &duration,
        &format!("{}isStale", rgt::export::turtle::VOCAB),
    )
    .unwrap();
    assert_eq!(
        stale.datatype().as_str(),
        "http://www.w3.org/2001/XMLSchema#boolean"
    );
    assert_eq!(stale.value(), "true");
    let line = literal_object(
        &triples,
        &root_a,
        &format!("{}lineNumber", rgt::export::turtle::VOCAB),
    )
    .unwrap();
    assert_eq!(
        line.datatype().as_str(),
        "http://www.w3.org/2001/XMLSchema#integer"
    );
    assert_eq!(line.value(), "1");
    let date = literal_object(
        &triples,
        &root_b,
        &format!("{}value", rgt::export::turtle::VOCAB),
    )
    .unwrap();
    assert_eq!(
        date.datatype().as_str(),
        "http://www.w3.org/2001/XMLSchema#dateTime"
    );
    let duration_literal = literal_object(
        &triples,
        &duration,
        &format!("{}value", rgt::export::turtle::VOCAB),
    )
    .unwrap();
    assert_eq!(
        duration_literal.datatype().as_str(),
        "http://www.w3.org/2001/XMLSchema#duration"
    );
    assert_eq!(duration_literal.value(), "PT0S");
    assert_eq!(
        literal_object(
            &triples,
            &source,
            &format!("{}relativePath", rgt::export::turtle::VOCAB)
        )
        .unwrap()
        .value(),
        "input/data.csv"
    );
    assert!(!output
        .turtle
        .contains(root.path().to_string_lossy().as_ref()));

    let capture = capture_activity_iri(token, "root/a with space", "cursor").unwrap();
    let agent = agent_iri("cursor");
    assert!(has_iri_object(
        &triples,
        &capture,
        "http://www.w3.org/ns/prov#used",
        &root_a
    ));
    assert!(has_iri_object(
        &triples,
        &capture,
        "http://www.w3.org/ns/prov#wasAssociatedWith",
        &agent
    ));
    assert!(triples.iter().any(|triple| {
        matches!(&triple.subject, NamedOrBlankNode::NamedNode(node) if node.as_str() == agent)
            && triple.predicate.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
            && matches!(&triple.object, Term::NamedNode(node) if node.as_str() == "http://www.w3.org/ns/prov#Agent")
    }));
    assert_eq!(
        literal_object(
            &triples,
            &agent,
            &format!("{}agentName", rgt::export::turtle::VOCAB)
        )
        .unwrap()
        .value(),
        "cursor"
    );
}

#[test]
fn ambiguous_edges_keep_direct_links_without_claiming_an_activity() {
    let root = tempdir().unwrap();
    let token = "0123456789abcdef0123456789abcdef";
    let mut snapshot = fixture(root.path(), token);
    snapshot.edges[1].expression = Some("a - b".into());
    let output = render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + StdDuration::from_secs(1),
    )
    .unwrap();
    let child_id = snapshot
        .nodes
        .iter()
        .find(|node| node.node_type == NodeType::Derived)
        .unwrap()
        .id
        .clone();
    let child = node_iri(token, &child_id).unwrap();
    let root_a = node_iri(token, "root/a with space").unwrap();
    let triples = parse(&output.turtle);
    assert!(has_iri_object(
        &triples,
        &child,
        "http://www.w3.org/ns/prov#wasDerivedFrom",
        &root_a
    ));
    assert!(!triples
        .iter()
        .any(|triple| { triple.predicate.as_str() == "http://www.w3.org/ns/prov#wasGeneratedBy" }));
    assert_eq!(output.ambiguous_children, vec![child_id]);
}

#[test]
fn absolute_source_paths_require_the_explicit_option_and_byte_limit_is_preflighted() {
    let root = tempdir().unwrap();
    let token = "0123456789abcdef0123456789abcdef";
    let snapshot = fixture(root.path(), token);
    let deadline = Instant::now() + StdDuration::from_secs(1);
    let default = render_turtle(&snapshot, root.path(), false, deadline).unwrap();
    assert!(!default
        .turtle
        .contains(root.path().to_string_lossy().as_ref()));
    let full = render_turtle(&snapshot, root.path(), true, deadline).unwrap();
    assert!(full.turtle.contains(
        root.path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    ));
    assert!(!parse(&full.turtle).is_empty());

    let error = render_turtle_with_limit(&snapshot, root.path(), false, 32, deadline).unwrap_err();
    assert!(error.contains("32-byte output limit"), "{error}");
}

#[test]
fn graph_cli_exports_turtle_and_preserves_existing_graph_formats() {
    let project = tempdir().unwrap();
    std::fs::write(project.path().join("data.csv"), "Revenue,120.50\n").unwrap();
    let record = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["record", "data.csv"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        record.status.success(),
        "{}",
        String::from_utf8_lossy(&record.stderr)
    );

    let turtle = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        turtle.status.success(),
        "{}",
        String::from_utf8_lossy(&turtle.stderr)
    );
    let parsed = TurtleParser::new()
        .for_slice(&turtle.stdout)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!parsed.is_empty());
    assert!(!String::from_utf8_lossy(&turtle.stdout)
        .contains(project.path().to_string_lossy().as_ref()));

    let full_paths = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "-f", "ttl", "--include-absolute-paths"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(full_paths.status.success());
    assert!(String::from_utf8_lossy(&full_paths.stdout).contains(
        project
            .path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    ));

    let invalid_option = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "text", "--include-absolute-paths"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert_eq!(invalid_option.status.code(), Some(2));
    assert!(invalid_option.stdout.is_empty());

    let text = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "text"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let mermaid = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "mermaid"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let dot = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "dot"])
        .current_dir(project.path())
        .output()
        .unwrap();
    let unknown = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "unknown-format"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&text.stdout).starts_with("=== RGT Provenance DAG Tree ==="));
    assert!(String::from_utf8_lossy(&mermaid.stdout).starts_with("graph TD\n"));
    assert!(String::from_utf8_lossy(&dot.stdout).starts_with("digraph RGT {\n"));
    assert_eq!(
        text.stdout, unknown.stdout,
        "unknown format keeps the text fallback"
    );
}

#[test]
fn malformed_duration_cli_error_has_no_turtle_stdout() {
    let project = tempdir().unwrap();
    {
        let db = DbStore::open_in_project(project.path()).unwrap();
        db.conn()
            .execute(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,duration_secs,is_stale,created_at,updated_at)
                 VALUES ('bad-duration','ROOT','DURATION',?1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                [i64::MAX],
            )
            .unwrap();
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duration"), "{stderr}");
    assert!(stderr.contains("out of range"), "{stderr}");
}

#[test]
fn derived_value_without_edges_warns_and_emits_no_activity_claims() {
    let project = tempdir().unwrap();
    {
        let db = DbStore::open_in_project(project.path()).unwrap();
        db.conn()
            .execute(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES ('orphan-derived','DERIVED','NUMBER',7.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "--format", "ttl"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed = parse(std::str::from_utf8(&output.stdout).unwrap());
    assert!(!parsed.iter().any(|triple| {
        triple.predicate.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
            && matches!(&triple.object, Term::NamedNode(node) if node.as_str() == "http://www.w3.org/ns/prov#Activity")
    }));
    assert!(!parsed
        .iter()
        .any(|triple| { triple.predicate.as_str() == "http://www.w3.org/ns/prov#wasGeneratedBy" }));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        stderr.matches("omitted derivation activity").count(),
        1,
        "{stderr}"
    );
    assert!(stderr.contains("orphan-derived"), "{stderr}");
}

#[test]
fn over_limit_cli_export_fails_before_writing_any_stdout() {
    let project = tempdir().unwrap();
    {
        let db = rgt::store::DbStore::open_in_project(project.path()).unwrap();
        db.conn().execute_batch("BEGIN").unwrap();
        let mut insert = db
            .conn()
            .prepare(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES (?1,'ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            )
            .unwrap();
        for index in 0..=rgt::export::turtle::MAX_VALUES {
            insert.execute([format!("limit-{index}")]).unwrap();
        }
        drop(insert);
        db.conn().execute_batch("COMMIT").unwrap();
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(["graph", "-f", "ttl"])
        .current_dir(project.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("at most 10000 recorded values"));
}
