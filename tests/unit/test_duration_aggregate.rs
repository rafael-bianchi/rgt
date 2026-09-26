use chrono::Duration;
use rgt::types::ValueData;
use rgt::verify::{compute_duration_aggregate, validate_duration_parent_ids};

fn durations(values: &[i64]) -> Vec<ValueData> {
    values
        .iter()
        .map(|seconds| ValueData::Duration(Duration::seconds(*seconds)))
        .collect()
}

#[test]
fn checked_sum_and_exact_average_keep_canonical_seconds() {
    let parents = durations(&[3_888_000, 2_700]);
    assert_eq!(
        compute_duration_aggregate("DURATION_SUM", &parents).unwrap(),
        3_890_700
    );
    assert_eq!(
        compute_duration_aggregate("DURATION_AVG", &parents).unwrap(),
        1_945_350
    );
    assert_eq!(
        compute_duration_aggregate("DURATION_AVG", &durations(&[1, 3])).unwrap(),
        2
    );
}

#[test]
fn signed_values_are_accepted_and_fractional_second_average_is_rejected() {
    assert_eq!(
        compute_duration_aggregate("DURATION_SUM", &durations(&[-900, 1_200])).unwrap(),
        300
    );
    assert_eq!(
        compute_duration_aggregate("DURATION_AVG", &durations(&[-900, 1_200])).unwrap(),
        150
    );
    assert!(
        compute_duration_aggregate("DURATION_AVG", &durations(&[1, 2]))
            .unwrap_err()
            .contains("whole number of seconds")
    );
}

#[test]
fn aggregate_checks_parent_count_kind_and_duplicate_ids() {
    assert!(compute_duration_aggregate("DURATION_SUM", &durations(&[1])).is_err());
    assert!(compute_duration_aggregate("DURATION_SUM", &durations(&vec![1; 27])).is_err());
    assert!(compute_duration_aggregate(
        "DURATION_SUM",
        &[
            ValueData::Duration(Duration::seconds(1)),
            ValueData::Number(2.0)
        ]
    )
    .is_err());
    assert!(validate_duration_parent_ids("DURATION_SUM", &["same".into(), "same".into()]).is_err());
    assert!(validate_duration_parent_ids("DURATION_AVG", &["a".into(), "b".into()]).is_ok());
}

#[test]
fn aggregate_rejects_results_outside_chrono_duration_range() {
    let max_seconds = Duration::MAX.num_seconds();
    assert!(
        compute_duration_aggregate("DURATION_SUM", &durations(&[max_seconds, max_seconds]))
            .is_err()
    );
}

#[test]
fn aggregate_rejects_positive_and_negative_fractional_duration_parents() {
    for fractional in [
        Duration::milliseconds(1_500),
        Duration::milliseconds(-1_500),
        Duration::milliseconds(500),
        Duration::milliseconds(-500),
    ] {
        let error = compute_duration_aggregate(
            "DURATION_SUM",
            &[
                ValueData::Duration(fractional),
                ValueData::Duration(Duration::zero()),
            ],
        )
        .unwrap_err();
        assert!(
            error.contains("whole number of seconds"),
            "unexpected error for {fractional:?}: {error}"
        );
    }
}

#[test]
fn unsupported_operation_is_rejected() {
    assert!(compute_duration_aggregate("DATE_DIFF", &durations(&[1, 2])).is_err());
}
