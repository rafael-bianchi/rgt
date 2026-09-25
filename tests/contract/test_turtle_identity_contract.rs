use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;
use rgt::export::turtle::{node_iri, render_turtle, ExportNode, ExportSnapshot};
use rgt::types::{NodeType, ValueData};
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn parse(text: &str) -> Vec<Triple> {
    TurtleParser::new()
        .for_slice(text.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn export(token: &str) -> String {
    let snapshot = ExportSnapshot {
        project_id: token.into(),
        nodes: vec![ExportNode {
            id: "matching-local-id".into(),
            node_type: NodeType::Root,
            value: ValueData::Number(12.0),
            source_doc_id: None,
            line_number: Some(2),
            is_stale: false,
        }],
        sources: vec![],
        edges: vec![],
        captures: vec![],
    };
    let root = tempdir().unwrap();
    render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap()
    .turtle
}

#[test]
fn rdf_union_keeps_matching_local_ids_distinct_between_projects() {
    let token_a = "0123456789abcdef0123456789abcdef";
    let token_b = "fedcba9876543210fedcba9876543210";
    let turtle_a = export(token_a);
    let turtle_b = export(token_b);
    let triples = parse(&format!("{turtle_a}\n{turtle_b}"));
    let value_a = node_iri(token_a, "matching-local-id").unwrap();
    let value_b = node_iri(token_b, "matching-local-id").unwrap();
    assert_ne!(value_a, value_b);

    for value in [&value_a, &value_b] {
        assert!(triples.iter().any(|triple| {
            matches!(&triple.subject, NamedOrBlankNode::NamedNode(node) if node.as_str() == value)
                && triple.predicate.as_str()
                    == "https://github.com/rafael-bianchi/rgt/vocab#localNodeId"
                && matches!(&triple.object, Term::Literal(literal) if literal.value() == "matching-local-id")
        }));
    }
    assert!(!triples
        .iter()
        .any(|triple| { triple.predicate.as_str() == "http://www.w3.org/2002/07/owl#sameAs" }));
}
