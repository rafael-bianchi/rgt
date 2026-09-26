use crate::types::{DurationClaim, ValueData};

/// Computes `date2 - date1` as an exact whole-second Duration.
pub fn compute_date_diff_seconds(parents: &[ValueData]) -> Result<i64, String> {
    if parents.len() != 2 {
        return Err(format!(
            "DATE_DIFF requires exactly 2 Date parents, got {}",
            parents.len()
        ));
    }
    let date1 = match &parents[0] {
        ValueData::Date(date) => date,
        other => {
            return Err(format!(
                "DATE_DIFF parent[0] must be a Date, got {:?}",
                other.kind()
            ))
        }
    };
    let date2 = match &parents[1] {
        ValueData::Date(date) => date,
        other => {
            return Err(format!(
                "DATE_DIFF parent[1] must be a Date, got {:?}",
                other.kind()
            ))
        }
    };

    if date2 < date1 {
        return Err(
            "DATE_DIFF computed date2 - date1 is negative; parents may be swapped — convention is --parents <date1>,<date2>"
                .into(),
        );
    }
    if date1.timestamp_subsec_nanos() != date2.timestamp_subsec_nanos() {
        return Err(
            "DATE_DIFF elapsed time includes a fractional second; only a whole number of seconds is supported"
                .into(),
        );
    }
    let seconds = date2
        .timestamp()
        .checked_sub(date1.timestamp())
        .ok_or_else(|| "DATE_DIFF elapsed seconds are outside the supported range".to_string())?;
    if chrono::Duration::try_seconds(seconds).is_none() {
        return Err(format!(
            "DATE_DIFF elapsed seconds '{seconds}' are outside the supported Duration range"
        ));
    }
    Ok(seconds)
}

/// Verifies that `date2 - date1` (from two Date parents) equals the claimed result.
///
/// # Arguments
/// - `parents`: Exactly two `ValueData::Date` nodes (`parent[0]` = date1, `parent[1]` = date2).
/// - `result_seconds`: The caller-claimed duration in seconds.
///
/// # Returns
/// `Ok(())` if the computed difference matches. `Err(message)` if the parent count is
/// wrong, either parent is not a Date, or the duration does not match.
pub fn verify_date_diff(parents: &[ValueData], claim: &DurationClaim) -> Result<(), String> {
    let computed_seconds = compute_date_diff_seconds(parents)?;
    claim.verify(computed_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DurationUnit;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_date_diff_match() {
        let d1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        let claim = DurationClaim::parse("864000", DurationUnit::Seconds).unwrap();
        assert!(verify_date_diff(&parents, &claim).is_ok());
    }

    #[test]
    fn test_date_diff_mismatch() {
        let d1 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 11, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        let claim = DurationClaim::parse("864001", DurationUnit::Seconds).unwrap();
        let error = verify_date_diff(&parents, &claim).unwrap_err();
        assert!(error.contains("expected 864000 seconds"));
    }

    #[test]
    fn test_date_diff_negative_rejected_as_swapped() {
        // d2 before d1 ⇒ negative duration — a swapped-order signal (FR-006).
        let d1 = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        let d2 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d1), ValueData::Date(d2)];
        let claim = DurationClaim::parse("-1209600", DurationUnit::Seconds).unwrap();
        let err = verify_date_diff(&parents, &claim).unwrap_err();
        assert!(err.contains("date2 - date1"), "{}", err);
        assert!(err.contains("swapped"), "{}", err);
    }

    #[test]
    fn test_date_diff_zero_is_ok() {
        let d = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let parents = vec![ValueData::Date(d), ValueData::Date(d)];
        let claim = DurationClaim::parse("0", DurationUnit::Seconds).unwrap();
        assert!(verify_date_diff(&parents, &claim).is_ok());
    }
}
