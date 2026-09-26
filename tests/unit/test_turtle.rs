use rgt::export::turtle::{
    agent_iri, capture_activity_iri, classify_derivation, derivation_activity_iri,
    encode_iri_component, escape_literal, format_name, load_snapshot, node_iri, project_iri,
    qualified_usage_iri, render_turtle, render_turtle_with_limit, source_iri, source_location,
    write_bytes_before_deadline, xsd_duration_lexical, ExportEdge, ExportNode, ExportSnapshot,
    MAX_VALUES,
};
use rgt::store::DbStore;
use rgt::types::{NodeType, TrackedNode, ValueData};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::{io, sync::mpsc};

#[test]
fn turtle_export_module_is_registered() {
    assert_eq!(format_name(), "ttl");
}

#[test]
fn empty_store_produces_an_empty_snapshot() {
    let db = DbStore::open_in_memory().unwrap();
    let snapshot = load_snapshot(db.conn(), Instant::now() + Duration::from_secs(1)).unwrap();
    assert_eq!(snapshot.nodes.len(), 0);
    assert_eq!(snapshot.sources.len(), 0);
    assert_eq!(snapshot.edges.len(), 0);
    assert_eq!(snapshot.captures.len(), 0);
    assert_eq!(snapshot.project_id.len(), 32);
}

#[test]
fn values_over_limit_fail_before_typed_value_decoding() {
    let db = DbStore::open_in_memory().unwrap();
    {
        let mut statement = db
            .conn()
            .prepare(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES (?1,'ROOT','NUMBER','malformed',0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            )
            .unwrap();
        for index in 0..=MAX_VALUES {
            statement.execute([format!("invalid-{index}")]).unwrap();
        }
    }
    let error = load_snapshot(db.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("at most 10000 recorded values"), "{error}");
}

#[test]
fn derivation_edges_over_limit_fail_during_bounded_preflight() {
    let db = DbStore::open_in_memory().unwrap();
    let conn = db.conn();
    conn.execute_batch("BEGIN").unwrap();
    {
        let mut node = conn
            .prepare(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
                 VALUES (?1,'ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            )
            .unwrap();
        for index in 0..180 {
            node.execute([format!("n{index:03}")]).unwrap();
        }
    }
    {
        let mut edge = conn
            .prepare(
                "INSERT INTO derivation_edges (parent_node_id,child_node_id,operation_type,expression)
                 VALUES (?1,?2,'EXPRESSION','a+b')",
            )
            .unwrap();
        let mut inserted = 0;
        for parent in 0..180 {
            for child in 0..180 {
                if parent == child {
                    continue;
                }
                edge.execute([format!("n{parent:03}"), format!("n{child:03}")])
                    .unwrap();
                inserted += 1;
                if inserted == 30_001 {
                    break;
                }
            }
            if inserted == 30_001 {
                break;
            }
        }
        assert_eq!(inserted, 30_001);
    }
    conn.execute_batch("COMMIT").unwrap();

    let error = load_snapshot(conn, Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("at most 30000 derivation edges"), "{error}");
}

#[test]
fn malformed_typed_values_are_hard_errors() {
    let db = DbStore::open_in_memory().unwrap();
    db.conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('bad-number','ROOT','NUMBER','not-a-number',0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let error = load_snapshot(db.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("invalid number"), "{error}");
}

#[test]
fn out_of_range_stored_durations_return_errors_without_panicking() {
    for seconds in [i64::MAX, i64::MIN] {
        let db = DbStore::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO tracked_nodes (id,node_type,value_kind,duration_secs,is_stale,created_at,updated_at)
                 VALUES ('bad-duration','ROOT','DURATION',?1,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                [seconds],
            )
            .unwrap();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            load_snapshot(db.conn(), Instant::now() + Duration::from_secs(1))
        }));
        let error = result
            .expect("decoding a stored duration must not panic")
            .unwrap_err();
        assert!(error.contains("duration"), "{error}");
        assert!(error.contains("out of range"), "{error}");
    }
}

#[test]
fn dangling_source_reference_is_a_hard_error() {
    let db = DbStore::open_in_memory().unwrap();
    db.conn()
        .execute_batch("PRAGMA foreign_keys = OFF")
        .unwrap();
    db.conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,source_doc_id,is_stale,created_at,updated_at)
             VALUES ('orphan-root','ROOT','NUMBER',1.0,777,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    let error = load_snapshot(db.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(
        error.contains("references missing source document"),
        "{error}"
    );
}

#[test]
fn capture_snapshot_rejects_unknown_agents_dangling_nodes_and_duplicate_pairs() {
    let unknown = DbStore::open_in_memory().unwrap();
    unknown
        .conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('known-node','ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    unknown
        .conn()
        .execute(
            "INSERT INTO capture_associations (node_id,agent_name) VALUES ('known-node','codex')",
            [],
        )
        .unwrap();
    let error = load_snapshot(unknown.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("unsupported agent 'codex'"), "{error}");

    let dangling = DbStore::open_in_memory().unwrap();
    dangling
        .conn()
        .execute_batch("PRAGMA foreign_keys=OFF")
        .unwrap();
    dangling
        .conn()
        .execute(
            "INSERT INTO capture_associations (node_id,agent_name) VALUES ('missing-node','cursor')",
            [],
        )
        .unwrap();
    let error =
        load_snapshot(dangling.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(
        error.contains("references missing value 'missing-node'"),
        "{error}"
    );

    let duplicate = DbStore::open_in_memory().unwrap();
    duplicate
        .conn()
        .execute(
            "INSERT INTO tracked_nodes (id,node_type,value_kind,number_val,is_stale,created_at,updated_at)
             VALUES ('duplicate-node','ROOT','NUMBER',1.0,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    duplicate
        .conn()
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
             DROP TABLE capture_associations;
             CREATE TABLE capture_associations (node_id TEXT, agent_name TEXT);
             INSERT INTO capture_associations VALUES ('duplicate-node','cursor');
             INSERT INTO capture_associations VALUES ('duplicate-node','cursor');",
        )
        .unwrap();
    let error =
        load_snapshot(duplicate.conn(), Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.contains("duplicate capture association"), "{error}");
}

#[test]
fn all_instance_iris_use_the_project_token_and_encode_components() {
    let token = "0123456789abcdef0123456789abcdef";
    let prefix = format!("https://github.com/rafael-bianchi/rgt/iri/v1/project/{token}");
    let node = "node/id with%space";
    let encoded = "node%2Fid%20with%25space";
    assert_eq!(project_iri(token).unwrap(), format!("{prefix}"));
    assert_eq!(
        node_iri(token, node).unwrap(),
        format!("{prefix}/node/{encoded}")
    );
    assert_eq!(source_iri(token, 7).unwrap(), format!("{prefix}/source/7"));
    assert_eq!(
        derivation_activity_iri(token, node).unwrap(),
        format!("{prefix}/activity/derive/{encoded}")
    );
    assert_eq!(
        qualified_usage_iri(token, node, 1).unwrap(),
        format!("{prefix}/usage/derive/{encoded}/1")
    );
    assert_eq!(
        capture_activity_iri(token, node, "claude-code").unwrap(),
        format!("{prefix}/activity/capture/{encoded}/claude-code")
    );
    assert_eq!(
        agent_iri("claude-code"),
        "https://github.com/rafael-bianchi/rgt/iri/v1/agent/claude-code"
    );
    assert_eq!(encode_iri_component("a/b %é"), "a%2Fb%20%25%C3%A9");
}

#[test]
fn turtle_literals_escape_controls_without_changing_unicode() {
    assert_eq!(
        escape_literal("quote \" slash \\\n\t é"),
        "quote \\\" slash \\\\\\n\\t é"
    );
}

#[test]
fn durations_use_xml_schema_duration_lexical_forms() {
    assert_eq!(xsd_duration_lexical(0), "PT0S");
    assert_eq!(xsd_duration_lexical(172_800), "P2D");
    assert_eq!(xsd_duration_lexical(3_665), "PT1H1M5S");
    assert_eq!(xsd_duration_lexical(-5), "-PT5S");
}

#[test]
fn source_paths_are_relative_only_after_canonical_containment() {
    let project = tempfile::tempdir().unwrap();
    let nested = project.path().join("data");
    std::fs::create_dir_all(&nested).unwrap();
    let file = nested.join("values.csv");
    std::fs::write(&file, "x,1\n").unwrap();
    let location = source_location("data/values.csv", project.path(), false);
    assert_eq!(location.relative_path.as_deref(), Some("data/values.csv"));
    assert!(location.absolute_path.is_none());
    let full = source_location("data/values.csv", project.path(), true);
    assert_eq!(full.relative_path.as_deref(), Some("data/values.csv"));
    assert_eq!(
        full.absolute_path.as_deref(),
        Some(file.canonicalize().unwrap().to_str().unwrap())
    );

    let outside = tempfile::NamedTempFile::new().unwrap();
    let private = source_location(outside.path().to_str().unwrap(), project.path(), false);
    assert!(private.relative_path.is_none());
    assert!(private.absolute_path.is_none());
    let missing_path = project.path().join("missing.csv");
    let opted_in = source_location(missing_path.to_str().unwrap(), project.path(), true);
    assert!(opted_in.relative_path.is_none());
    assert_eq!(
        opted_in.absolute_path.as_deref(),
        Some(missing_path.to_string_lossy().as_ref())
    );
}

#[cfg(unix)]
#[test]
fn source_path_symlink_cannot_escape_project_relative_classification() {
    use std::os::unix::fs::symlink;

    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("private.csv"), "value,123\n").unwrap();
    std::fs::create_dir_all(project.path().join("data")).unwrap();
    symlink(
        outside.path().join("private.csv"),
        project.path().join("data/escape.csv"),
    )
    .unwrap();
    let location = source_location("data/escape.csv", project.path(), false);
    assert!(location.relative_path.is_none());
    assert!(location.absolute_path.is_none());
}

fn node(id: String, node_type: NodeType, value: ValueData) -> ExportNode {
    ExportNode {
        id,
        node_type,
        value,
        source_doc_id: None,
        line_number: None,
        is_stale: false,
    }
}

#[test]
fn derivation_activity_requires_ordered_distinct_parents_and_matching_id() {
    let parent_ids = vec!["parent-a".to_string(), "parent-b".to_string()];
    let child_value = ValueData::Number(7.0);
    let child_id = TrackedNode::generate_derived_id(&parent_ids, "EXPRESSION", &child_value);
    let child = node(child_id.clone(), NodeType::Derived, child_value);
    let parents = HashMap::from([
        (
            parent_ids[0].clone(),
            node(
                parent_ids[0].clone(),
                NodeType::Root,
                ValueData::Number(3.0),
            ),
        ),
        (
            parent_ids[1].clone(),
            node(
                parent_ids[1].clone(),
                NodeType::Root,
                ValueData::Number(4.0),
            ),
        ),
    ]);
    let edges = vec![
        ExportEdge {
            id: 1,
            parent_node_id: parent_ids[0].clone(),
            child_node_id: child_id.clone(),
            operation_type: "EXPRESSION".into(),
            expression: Some("a + b".into()),
        },
        ExportEdge {
            id: 2,
            parent_node_id: parent_ids[1].clone(),
            child_node_id: child_id,
            operation_type: "EXPRESSION".into(),
            expression: Some("a + b".into()),
        },
    ];
    let activity = classify_derivation(&child, &edges, &parents).unwrap();
    assert_eq!(activity.parent_ids, parent_ids);
    assert_eq!(activity.operation_type, "EXPRESSION");
    assert_eq!(activity.expression.as_deref(), Some("a + b"));
}

#[test]
fn derivation_activity_accepts_historical_48_bit_id_and_rejects_ambiguous_edges() {
    let parent_ids = vec!["parent-a".to_string(), "parent-b".to_string()];
    let value = ValueData::Number(7.0);
    let full_id = TrackedNode::generate_derived_id(&parent_ids, "EXPRESSION", &value);
    let short_id = format!("{}{}", &full_id[..9], &full_id[9..21]);
    let child = node(short_id.clone(), NodeType::Derived, value.clone());
    let parents = HashMap::from([
        (
            parent_ids[0].clone(),
            node(
                parent_ids[0].clone(),
                NodeType::Root,
                ValueData::Number(3.0),
            ),
        ),
        (
            parent_ids[1].clone(),
            node(
                parent_ids[1].clone(),
                NodeType::Root,
                ValueData::Number(4.0),
            ),
        ),
    ]);
    let mut edges = parent_ids
        .iter()
        .enumerate()
        .map(|(index, parent)| ExportEdge {
            id: index as i64 + 1,
            parent_node_id: parent.clone(),
            child_node_id: short_id.clone(),
            operation_type: "EXPRESSION".into(),
            expression: Some("a + b".into()),
        })
        .collect::<Vec<_>>();
    assert!(classify_derivation(&child, &edges, &parents).is_some());
    edges[1].expression = Some("a - b".into());
    assert!(classify_derivation(&child, &edges, &parents).is_none());
    edges[1].expression = Some("a + b".into());
    edges[1].parent_node_id = edges[0].parent_node_id.clone();
    assert!(classify_derivation(&child, &edges, &parents).is_none());
}

fn duration_activity_case(
    operation: &str,
    parent_values: Vec<ValueData>,
    expression: Option<&str>,
) -> Option<rgt::export::turtle::DerivationActivity> {
    let parent_ids = (0..parent_values.len())
        .map(|index| format!("parent-{index}"))
        .collect::<Vec<_>>();
    let seconds = 60;
    let child_value = ValueData::Duration(chrono::Duration::seconds(seconds));
    let child_id = TrackedNode::generate_duration_derived_id(&parent_ids, operation, seconds)
        .unwrap_or_else(|_| TrackedNode::generate_derived_id(&parent_ids, operation, &child_value));
    let child = node(child_id.clone(), NodeType::Derived, child_value);
    let parents = parent_ids
        .iter()
        .cloned()
        .zip(parent_values)
        .map(|(id, value)| (id.clone(), node(id, NodeType::Root, value)))
        .collect::<HashMap<_, _>>();
    let edges = parent_ids
        .iter()
        .enumerate()
        .map(|(index, parent_id)| ExportEdge {
            id: index as i64 + 1,
            parent_node_id: parent_id.clone(),
            child_node_id: child_id.clone(),
            operation_type: operation.into(),
            expression: expression.map(str::to_string),
        })
        .collect::<Vec<_>>();
    classify_derivation(&child, &edges, &parents)
}

#[test]
fn duration_activity_requires_supported_operation_parent_evidence_and_no_expression() {
    use chrono::{TimeZone, Utc};

    let date = || ValueData::Date(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap());
    let duration = || ValueData::Duration(chrono::Duration::seconds(30));

    assert!(duration_activity_case("DATE_DIFF", vec![date(), date()], None).is_some());
    assert!(
        duration_activity_case("DATE_DIFF", vec![ValueData::Number(1.0), date()], None).is_none()
    );
    assert!(duration_activity_case("DATE_DIFF", vec![date()], None).is_none());
    assert!(duration_activity_case("DATE_DIFF", vec![date(), date(), date()], None).is_none());

    assert!(duration_activity_case("DURATION_SUM", vec![duration(), duration()], None).is_some());
    assert!(duration_activity_case("DURATION_SUM", vec![date(), duration()], None).is_none());
    assert!(duration_activity_case("DURATION_AVG", vec![duration()], None).is_none());
    assert!(
        duration_activity_case("DURATION_SUM", vec![duration(), duration()], Some("a + b"))
            .is_none()
    );
    assert!(
        duration_activity_case("EXPRESSION", vec![duration(), duration()], Some("a + b")).is_none()
    );
}

#[test]
fn turtle_keeps_direct_lineage_when_duration_activity_has_a_wrong_parent_kind() {
    let project = tempfile::tempdir().unwrap();
    let project_id = "0123456789abcdef0123456789abcdef";
    let parent_ids = vec!["number-parent".to_string(), "duration-parent".to_string()];
    let child_value = ValueData::Duration(chrono::Duration::seconds(60));
    let child_id =
        TrackedNode::generate_duration_derived_id(&parent_ids, "DURATION_SUM", 60).unwrap();
    let nodes = vec![
        node(
            parent_ids[0].clone(),
            NodeType::Root,
            ValueData::Number(30.0),
        ),
        node(
            parent_ids[1].clone(),
            NodeType::Root,
            ValueData::Duration(chrono::Duration::seconds(30)),
        ),
        node(child_id.clone(), NodeType::Derived, child_value),
    ];
    let edges = parent_ids
        .iter()
        .enumerate()
        .map(|(index, parent_id)| ExportEdge {
            id: index as i64 + 1,
            parent_node_id: parent_id.clone(),
            child_node_id: child_id.clone(),
            operation_type: "DURATION_SUM".into(),
            expression: None,
        })
        .collect();
    let output = render_turtle(
        &ExportSnapshot {
            project_id: project_id.into(),
            nodes,
            sources: vec![],
            edges,
            captures: vec![],
        },
        project.path(),
        false,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    let child_iri = node_iri(project_id, &child_id).unwrap();
    for parent_id in parent_ids {
        let parent_iri = node_iri(project_id, &parent_id).unwrap();
        assert!(output.turtle.contains(&format!(
            "<{child_iri}> prov:wasDerivedFrom <{parent_iri}> ."
        )));
    }
    assert!(!output.turtle.contains("prov:wasGeneratedBy"));
    assert_eq!(output.ambiguous_children, vec![child_id]);
}

#[test]
fn turtle_quota_accepts_the_exact_byte_boundary_and_rejects_one_byte_less() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = ExportSnapshot {
        project_id: "0123456789abcdef0123456789abcdef".into(),
        nodes: vec![],
        sources: vec![],
        edges: vec![],
        captures: vec![],
    };
    let deadline = Instant::now() + Duration::from_secs(1);
    let complete = render_turtle(&snapshot, temp.path(), false, deadline).unwrap();
    let exact = render_turtle_with_limit(
        &snapshot,
        temp.path(),
        false,
        complete.turtle.len(),
        deadline,
    )
    .unwrap();
    assert_eq!(exact.turtle.len(), complete.turtle.len());
    let error = render_turtle_with_limit(
        &snapshot,
        temp.path(),
        false,
        complete.turtle.len() - 1,
        deadline,
    )
    .unwrap_err();
    assert!(error.contains("output limit"), "{error}");
}

#[test]
fn expired_render_deadline_fails_before_output() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = ExportSnapshot {
        project_id: "0123456789abcdef0123456789abcdef".into(),
        nodes: vec![],
        sources: vec![],
        edges: vec![],
        captures: vec![],
    };
    let error = render_turtle(
        &snapshot,
        temp.path(),
        false,
        Instant::now() - Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(error.contains("deadline"), "{error}");
}

struct StalledWriter(mpsc::Receiver<()>);

impl io::Write for StalledWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.recv().unwrap();
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BrokenWriter;

impl io::Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn final_writer_reports_broken_pipe_and_times_out_without_joining_a_stall() {
    let broken = write_bytes_before_deadline(
        BrokenWriter,
        b"complete turtle".to_vec(),
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(broken.contains("closed"), "{broken}");

    let (release, receiver) = mpsc::channel();
    let timed_out = write_bytes_before_deadline(
        StalledWriter(receiver),
        b"blocked turtle".to_vec(),
        Instant::now() + Duration::from_millis(50),
    )
    .unwrap_err();
    release.send(()).unwrap();
    assert!(timed_out.contains("deadline"), "{timed_out}");
}
