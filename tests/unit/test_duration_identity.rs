use chrono::Duration;
use rgt::types::{TrackedNode, ValueData};

fn duration_id(parents: &[&str], operation: &str, seconds: i64) -> Result<String, String> {
    let owned = parents
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    TrackedNode::generate_duration_derived_id(&owned, operation, seconds)
}

#[test]
fn identity_includes_operation_and_exact_signed_seconds() {
    let base = duration_id(&["date-a", "date-b"], "DATE_DIFF", 3_888_000).unwrap();
    assert_ne!(
        base,
        duration_id(&["date-a", "date-b"], "DATE_DIFF", 3_888_001).unwrap()
    );
    assert_ne!(
        base,
        duration_id(&["date-a", "date-b"], "DURATION_SUM", 3_888_000).unwrap()
    );
    assert_ne!(
        duration_id(&["duration-a", "duration-b"], "DURATION_SUM", -6).unwrap(),
        duration_id(&["duration-a", "duration-b"], "DURATION_SUM", 6).unwrap()
    );
    assert!(base.starts_with("node_drv_"));
    assert_eq!(base.len(), "node_drv_".len() + 32);
}

#[test]
fn date_parent_order_is_significant_but_aggregate_order_is_canonical() {
    assert_ne!(
        duration_id(&["date-a", "date-b"], "DATE_DIFF", 1).unwrap(),
        duration_id(&["date-b", "date-a"], "DATE_DIFF", 1).unwrap()
    );
    for operation in ["DURATION_SUM", "DURATION_AVG"] {
        assert_eq!(
            duration_id(&["duration-a", "duration-b"], operation, 7).unwrap(),
            duration_id(&["duration-b", "duration-a"], operation, 7).unwrap()
        );
    }
}

#[test]
fn aggregate_identity_rejects_duplicate_parents_and_length_delimits_ids() {
    assert!(duration_id(&["same", "same"], "DURATION_SUM", 2).is_err());
    assert_ne!(
        duration_id(&["ab", "c"], "DURATION_SUM", 2).unwrap(),
        duration_id(&["a", "bc"], "DURATION_SUM", 2).unwrap()
    );
}

#[test]
fn human_display_collisions_do_not_collapse_distinct_seconds() {
    let one_day = ValueData::Duration(Duration::seconds(86_400));
    let one_day_and_one_second = ValueData::Duration(Duration::seconds(86_401));
    assert_eq!(
        one_day.to_string_repr(),
        one_day_and_one_second.to_string_repr()
    );
    assert_eq!(one_day.to_string_repr(), "1d");
    assert_ne!(
        duration_id(&["parent-a", "parent-b"], "DURATION_SUM", 86_400).unwrap(),
        duration_id(&["parent-a", "parent-b"], "DURATION_SUM", 86_401).unwrap()
    );
}
