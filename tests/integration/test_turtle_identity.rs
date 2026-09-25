use rgt::export::turtle::{node_iri, render_turtle, ExportNode, ExportSnapshot};
use rgt::store::{
    queries::{get_or_create_rdf_project_id, get_setting, set_setting},
    DbStore,
};
use rgt::types::{NodeType, ValueData};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn project_token_is_128_bit_lowercase_hex_and_stable_after_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("store.db");
    let first = {
        let db = DbStore::open(&path).unwrap();
        let stored = get_setting(db.conn(), "rdf_project_id").unwrap();
        assert!(
            stored.is_some(),
            "store open must initialize project identity"
        );
        get_or_create_rdf_project_id(db.conn()).unwrap()
    };
    let second = {
        let db = DbStore::open(&path).unwrap();
        get_or_create_rdf_project_id(db.conn()).unwrap()
    };

    assert_eq!(first, second);
    assert_eq!(first.len(), 32);
    assert!(first
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}

#[test]
fn concurrent_first_openers_keep_one_project_token() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("store.db");
    let mut threads = Vec::new();
    for _ in 0..8 {
        let path: PathBuf = path.clone();
        threads.push(std::thread::spawn(move || {
            let db = DbStore::open(path).unwrap();
            get_or_create_rdf_project_id(db.conn()).unwrap()
        }));
    }
    let tokens: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert!(tokens.iter().all(|token| token == &tokens[0]));
}

#[test]
fn malformed_project_token_is_rejected() {
    let db = DbStore::open_in_memory().unwrap();
    set_setting(db.conn(), "rdf_project_id", "not-a-token").unwrap();
    let error = get_or_create_rdf_project_id(db.conn()).unwrap_err();
    assert!(error.to_string().contains("invalid rdf_project_id"));
}

#[test]
fn malformed_project_token_prevents_store_reopen() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("store.db");
    {
        let db = DbStore::open(&path).unwrap();
        set_setting(db.conn(), "rdf_project_id", "not-a-token").unwrap();
    }
    let error = match DbStore::open(&path) {
        Ok(_) => panic!("store open accepted malformed rdf_project_id"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("invalid rdf_project_id"));
}

#[test]
fn legacy_store_without_project_token_gets_one_on_open() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("store.db");
    {
        let db = DbStore::open(&path).unwrap();
        db.conn()
            .execute(
                "DELETE FROM project_settings WHERE key='rdf_project_id'",
                [],
            )
            .unwrap();
    }
    let db = DbStore::open(&path).unwrap();
    let token = get_setting(db.conn(), "rdf_project_id").unwrap();
    assert!(
        token.is_some(),
        "legacy store open must create project identity"
    );
}

#[test]
fn independent_stores_get_distinct_tokens_and_copies_keep_the_original_token() {
    let dir = tempdir().unwrap();
    let path_a = dir.path().join("a.db");
    let path_b = dir.path().join("b.db");
    let token_a = {
        let db = DbStore::open(&path_a).unwrap();
        get_setting(db.conn(), "rdf_project_id").unwrap().unwrap()
    };
    let token_b = {
        let db = DbStore::open(&path_b).unwrap();
        get_setting(db.conn(), "rdf_project_id").unwrap().unwrap()
    };
    assert_ne!(token_a, token_b);

    let copy_path = dir.path().join("a-copy.db");
    fs::copy(&path_a, &copy_path).unwrap();
    let copied_token =
        get_or_create_rdf_project_id(DbStore::open(&copy_path).unwrap().conn()).unwrap();
    assert_eq!(copied_token, token_a);
    assert_eq!(
        node_iri(&token_a, "same-node").unwrap(),
        node_iri(&copied_token, "same-node").unwrap()
    );
}

#[test]
fn repeat_exports_are_byte_stable_and_historical_ids_remain_local_identity() {
    let root = tempdir().unwrap();
    let token = "0123456789abcdef0123456789abcdef";
    let historical = "node_drv_0123456789ab";
    let snapshot = ExportSnapshot {
        project_id: token.into(),
        nodes: vec![ExportNode {
            id: historical.into(),
            node_type: NodeType::Derived,
            value: ValueData::Number(1.0),
            source_doc_id: None,
            line_number: None,
            is_stale: false,
        }],
        sources: vec![],
        edges: vec![],
        captures: vec![],
    };
    let first = render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    let second = render_turtle(
        &snapshot,
        root.path(),
        false,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(first.turtle, second.turtle);
    assert!(first
        .turtle
        .contains(&format!("rgt:localNodeId \"{historical}\"")));
    assert_eq!(
        node_iri(token, historical).unwrap(),
        node_iri(token, historical).unwrap()
    );
}
