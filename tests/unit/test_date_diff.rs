#[cfg(test)]
mod tests {
    use chrono::{DateTime, Duration, Utc};
    use rgt::types::ValueData;

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
}
