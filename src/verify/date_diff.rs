use crate::types::ValueData;

/// Verifies that `date2 - date1` (from two Date parents) equals the claimed result.
///
/// # Arguments
/// - `parents`: Exactly two `ValueData::Date` nodes (`parent[0]` = date1, `parent[1]` = date2).
/// - `result_seconds`: The caller-claimed duration in seconds.
///
/// # Returns
/// `Ok(())` if the computed difference matches. `Err(message)` if the parent count is
/// wrong, either parent is not a Date, or the duration does not match.
pub fn verify_date_diff(parents: &[ValueData], result_seconds: i64) -> Result<(), String> {
    if parents.len() != 2 {
        return Err(format!(
            "DATE_DIFF requires exactly 2 Date parents, got {}",
            parents.len()
        ));
    }

    let date1 = match &parents[0] {
        ValueData::Date(d) => *d,
        other => {
            return Err(format!(
                "DATE_DIFF parent[0] must be a Date, got {:?}",
                other.kind()
            ))
        }
    };

    let date2 = match &parents[1] {
        ValueData::Date(d) => *d,
        other => {
            return Err(format!(
                "DATE_DIFF parent[1] must be a Date, got {:?}",
                other.kind()
            ))
        }
    };

    let dur = ValueData::date_diff(&date1, &date2);
    let computed_seconds = dur.num_seconds();

    // FR-006: enforce the `date2 - date1` convention. A negative duration is
    // the observable signal of a swapped `--parents` order — reject it with a
    // convention-naming error rather than passing the opposite sign silently.
    if computed_seconds < 0 {
        return Err(format!(
            "DATE_DIFF computed date2 - date1 = {}s (negative); parents may be swapped — convention is --parents <date1>,<date2>",
            computed_seconds
        ));
    }

    if computed_seconds == result_seconds {
        Ok(())
    } else {
        Err(format!(
            "expected {}s, got {}s",
            computed_seconds, result_seconds
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_date_diff_match() {
        let d1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        assert!(verify_date_diff(&parents, 864_000).is_ok());
    }

    #[test]
    fn test_date_diff_mismatch() {
        let d1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        assert!(verify_date_diff(&parents, 864_001).is_err());
    }

    #[test]
    fn test_date_diff_negative_rejected_as_swapped() {
        // d2 before d1 ⇒ negative duration — a swapped-order signal (FR-006).
        let d1 = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        let err = verify_date_diff(&parents, -1209600).unwrap_err();
        assert!(err.contains("date2 - date1"), "{}", err);
        assert!(err.contains("swapped"), "{}", err);
    }

    #[test]
    fn test_date_diff_zero_is_ok() {
        let d = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d), ValueData::Date(d)];
        assert!(verify_date_diff(&parents, 0).is_ok());
    }
}
