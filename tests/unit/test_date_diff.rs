#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
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
    }
}
