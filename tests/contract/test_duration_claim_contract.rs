use chrono::{Duration, TimeZone, Utc};
use rgt::store::queries::{insert_tracked_node, list_all_nodes};
use rgt::store::DbStore;
use rgt::types::{DurationClaim, DurationUnit, NodeType, TrackedNode, ValueData};
use rgt::verify::{verify, verify_temporal_claim};
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let db = DbStore::open_in_project(directory.path()).unwrap();
    let values = [
        (
            "date-a",
            ValueData::Date(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
        ),
        (
            "date-b",
            ValueData::Date(Utc.with_ymd_and_hms(2026, 2, 15, 0, 0, 0).unwrap()),
        ),
        (
            "duration-a",
            ValueData::Duration(Duration::seconds(604_800)),
        ),
        ("duration-b", ValueData::Duration(Duration::seconds(0))),
        ("number-a", ValueData::Number(2.0)),
        ("number-b", ValueData::Number(3.0)),
    ];
    for (id, value) in values {
        let now = Utc::now();
        insert_tracked_node(
            db.conn(),
            &TrackedNode {
                id: id.into(),
                node_type: NodeType::Root,
                value_kind: value.kind(),
                value,
                source_doc_id: None,
                line_number: None,
                is_stale: false,
                stale_reason: None,
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
    }
    drop(db);
    directory
}

fn run(directory: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rgt"))
        .args(args)
        .current_dir(directory.path())
        .output()
        .unwrap()
}

fn node_count(directory: &TempDir) -> usize {
    list_all_nodes(DbStore::open_in_project(directory.path()).unwrap().conn())
        .unwrap()
        .len()
}

#[test]
fn verify_accepts_day_claims_seconds_and_scientific_notation_silently() {
    let directory = fixture();
    for result in ["45", "4.5e1"] {
        let output = run(
            &directory,
            &[
                "verify",
                "--parents",
                "date-a,date-b",
                "--operation",
                "DATE_DIFF",
                "--result",
                result,
                "--result-unit",
                "days",
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
    let seconds = run(
        &directory,
        &[
            "verify",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "3888000",
        ],
    );
    assert!(
        seconds.status.success(),
        "{}",
        String::from_utf8_lossy(&seconds.stderr)
    );
    assert_eq!(node_count(&directory), 6);
}

#[test]
fn verify_uses_exact_claim_units_for_duration_aggregates_without_writes() {
    let directory = fixture();
    let before = node_count(&directory);
    for (operation, result, unit) in [
        ("DURATION_SUM", "1", "weeks"),
        ("DURATION_AVG", "5040", "minutes"),
    ] {
        let output = run(
            &directory,
            &[
                "verify",
                "--parents",
                "duration-a,duration-b",
                "--operation",
                operation,
                "--result",
                result,
                "--result-unit",
                unit,
            ],
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
    assert_eq!(node_count(&directory), before);
}

#[test]
fn malformed_claims_and_wrong_claims_have_distinct_exit_codes_and_no_writes() {
    let directory = fixture();
    let before = node_count(&directory);
    let wrong = run(
        &directory,
        &[
            "verify",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "44",
            "--result-unit",
            "days",
        ],
    );
    assert_eq!(wrong.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("claim unit days"));

    let malformed = run(
        &directory,
        &[
            "verify",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "0.01",
            "--result-unit",
            "minutes",
        ],
    );
    assert_eq!(malformed.status.code(), Some(2));
    assert_eq!(node_count(&directory), before);
}

#[test]
fn verify_rejects_display_unit_and_preserves_expression_number_behavior() {
    let directory = fixture();
    let invalid = run(
        &directory,
        &[
            "verify",
            "--parents",
            "date-a,date-b",
            "--operation",
            "DATE_DIFF",
            "--result",
            "45",
            "--unit",
            "days",
        ],
    );
    assert_eq!(invalid.status.code(), Some(2));

    let expression = run(
        &directory,
        &[
            "verify",
            "--parents",
            "number-a,number-b",
            "--operation",
            "EXPRESSION",
            "--expression",
            "a + b",
            "--result",
            "5",
        ],
    );
    assert!(
        expression.status.success(),
        "{}",
        String::from_utf8_lossy(&expression.stderr)
    );
    assert!(expression.stdout.is_empty());
}

#[test]
fn legacy_expression_with_temporal_parents_warns_that_its_result_is_a_number() {
    let directory = fixture();
    let duration = run(
        &directory,
        &[
            "derive",
            "--parents",
            "duration-a",
            "--operation",
            "EXPRESSION",
            "--expression",
            "a + 1",
            "--result",
            "604801",
        ],
    );
    assert!(
        duration.status.success(),
        "{}",
        String::from_utf8_lossy(&duration.stderr)
    );
    let duration_warning = String::from_utf8_lossy(&duration.stderr);
    assert!(duration_warning.contains("Duration parents are interpreted as elapsed seconds"));
    assert!(duration_warning.contains("result is a Number"));

    let date = run(
        &directory,
        &[
            "verify",
            "--parents",
            "date-a,date-b",
            "--operation",
            "EXPRESSION",
            "--expression",
            "a - a",
            "--result",
            "0",
        ],
    );
    assert!(
        date.status.success(),
        "{}",
        String::from_utf8_lossy(&date.stderr)
    );
    let date_warning = String::from_utf8_lossy(&date.stderr);
    assert!(date_warning.contains("Date parents are interpreted as Unix timestamp seconds"));
    assert!(date_warning.contains("result is a Number"));
}

#[test]
fn public_f64_verify_rejects_fractional_and_rounded_temporal_claims() {
    let date_a = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let date_b = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
    let parents = vec![ValueData::Date(date_a), ValueData::Date(date_b)];

    for claim in [864_000.5_f64, "864000.00000000001".parse::<f64>().unwrap()] {
        let error = verify(&parents, "DATE_DIFF", None, claim).unwrap_err();
        assert!(error.contains("exact temporal claim API"), "{error}");
    }
}

#[test]
fn public_exact_temporal_claim_api_preserves_decimal_precision() {
    let date_a = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let date_b = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
    let parents = vec![ValueData::Date(date_a), ValueData::Date(date_b)];

    let exact = DurationClaim::parse("10", DurationUnit::Days).unwrap();
    assert!(verify_temporal_claim(&parents, "DATE_DIFF", &exact).is_ok());

    let fractional = DurationClaim::parse("864000.00000000001", DurationUnit::Seconds);
    assert!(fractional.is_err());
}

#[test]
fn temporal_help_documents_fixed_units_and_storage_semantics() {
    for command in ["derive", "verify", "query"] {
        let output = Command::new(env!("CARGO_BIN_EXE_rgt"))
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        for promise in [
            "seconds",
            "minutes",
            "hours",
            "days",
            "weeks",
            "whole seconds",
            "months and years",
            "not stored",
        ] {
            assert!(
                help.contains(promise),
                "{command} --help is missing {promise:?}:\n{help}"
            );
        }
        if command != "query" {
            assert!(
                help.contains("EXPRESSION") && help.contains("warning"),
                "{command} --help must explain the legacy temporal EXPRESSION warning:\n{help}"
            );
        }
    }
}
