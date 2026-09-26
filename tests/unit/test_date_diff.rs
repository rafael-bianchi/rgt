#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};
    use rgt::types::ValueData;
    use rgt::verify::compute_date_diff_seconds;

    #[test]
    fn test_date_difference_calculation() {
        let d1 = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let d2 = DateTime::parse_from_rfc3339("2026-01-11T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let dur = ValueData::date_diff(&d1, &d2);
        assert_eq!(dur.num_days(), 10);
        assert_eq!(dur.num_seconds(), 864_000);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "10d");
    }

    #[test]
    fn test_duration_to_string_exact_day() {
        let dur = Duration::seconds(14 * 86_400);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "14d");
    }

    #[test]
    fn test_duration_to_string_hour_range() {
        let dur = Duration::seconds(2 * 3_600 + 15 * 60 + 30);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "2h 15m 30s");
    }

    #[test]
    fn test_duration_to_string_middle_zero_preservation() {
        let dur = Duration::seconds(3_600 + 5);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "1h 0m 5s");
    }

    #[test]
    fn test_duration_to_string_minute_range() {
        let dur = Duration::seconds(5 * 60 + 30);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "5m 30s");
    }

    #[test]
    fn test_duration_to_string_seconds_only() {
        let dur = Duration::seconds(45);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "45s");
    }

    #[test]
    fn test_duration_to_string_zero() {
        let dur = Duration::seconds(0);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "0s");
    }

    #[test]
    fn test_duration_to_string_large_days() {
        let dur = Duration::seconds(400 * 86_400);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "400d");
    }

    #[test]
    fn test_duration_to_string_negative_multi_day() {
        let dur = Duration::seconds(-5 * 86_400);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "-5d");
    }

    #[test]
    fn test_duration_to_string_negative_seconds() {
        let dur = Duration::seconds(-30);
        assert_eq!(ValueData::Duration(dur).to_string_repr(), "-30s");
    }

    fn parsed(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn date_diff_computes_exact_whole_seconds_and_zero() {
        let parents = vec![
            ValueData::Date(parsed("2026-01-01T00:00:00Z")),
            ValueData::Date(parsed("2026-02-15T00:00:00Z")),
        ];
        assert_eq!(compute_date_diff_seconds(&parents).unwrap(), 3_888_000);

        let same = vec![parents[0].clone(), parents[0].clone()];
        assert_eq!(compute_date_diff_seconds(&same).unwrap(), 0);
    }

    #[test]
    fn date_diff_rejects_positive_and_negative_fractional_seconds() {
        let positive = vec![
            ValueData::Date(parsed("2026-01-01T00:00:00.000000001Z")),
            ValueData::Date(parsed("2026-01-01T00:00:00.000000002Z")),
        ];
        assert!(compute_date_diff_seconds(&positive)
            .unwrap_err()
            .contains("whole number of seconds"));

        let negative_fraction = vec![
            ValueData::Date(parsed("2026-01-01T00:00:00.000000002Z")),
            ValueData::Date(parsed("2026-01-01T00:00:00.000000001Z")),
        ];
        assert!(compute_date_diff_seconds(&negative_fraction)
            .unwrap_err()
            .contains("negative"));
    }

    #[test]
    fn date_diff_rejects_reversed_order_and_non_date_parents() {
        let reversed = vec![
            ValueData::Date(parsed("2026-01-02T00:00:00Z")),
            ValueData::Date(parsed("2026-01-01T00:00:00Z")),
        ];
        assert!(compute_date_diff_seconds(&reversed)
            .unwrap_err()
            .contains("negative"));
        assert!(compute_date_diff_seconds(&[ValueData::Number(1.0)]).is_err());
    }
}
