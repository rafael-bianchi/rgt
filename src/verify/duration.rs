use crate::types::{exact_duration_seconds, ValueData};
use std::collections::HashSet;

/// Computes a typed Duration aggregate from canonical whole seconds.
pub fn compute_duration_aggregate(operation: &str, parents: &[ValueData]) -> Result<i64, String> {
    if !matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        return Err(format!("unsupported Duration aggregate '{operation}'"));
    }
    if !(2..=super::MAX_PARENTS).contains(&parents.len()) {
        return Err(format!(
            "{operation} requires 2 to {} Duration parents, got {}",
            super::MAX_PARENTS,
            parents.len()
        ));
    }

    let mut total = 0_i128;
    for (index, parent) in parents.iter().enumerate() {
        let seconds = match parent {
            ValueData::Duration(duration) => exact_duration_seconds(duration)
                .map_err(|error| format!("{operation} parent[{index}] {error}"))?,
            other => {
                return Err(format!(
                    "{operation} parent[{index}] must be a Duration, got {:?}",
                    other.kind()
                ))
            }
        };
        total = total
            .checked_add(seconds as i128)
            .ok_or_else(|| format!("{operation} seconds overflowed"))?;
    }

    let result = if operation == "DURATION_AVG" {
        let count = parents.len() as i128;
        if total % count != 0 {
            return Err(format!(
                "DURATION_AVG result is not a whole number of seconds (sum {total} across {} parents)",
                parents.len()
            ));
        }
        total / count
    } else {
        total
    };
    let seconds = i64::try_from(result)
        .map_err(|_| format!("{operation} result is outside the supported seconds range"))?;
    if chrono::Duration::try_seconds(seconds).is_none() {
        return Err(format!(
            "{operation} result seconds '{seconds}' are outside the supported Duration range"
        ));
    }
    Ok(seconds)
}

/// Enforces graph-representable operands for duration aggregates.
pub fn validate_duration_parent_ids(operation: &str, parent_ids: &[String]) -> Result<(), String> {
    if !matches!(operation, "DURATION_SUM" | "DURATION_AVG") {
        return Err(format!("unsupported Duration aggregate '{operation}'"));
    }
    if !(2..=super::MAX_PARENTS).contains(&parent_ids.len()) {
        return Err(format!(
            "{operation} requires 2 to {} distinct Duration parents, got {}",
            super::MAX_PARENTS,
            parent_ids.len()
        ));
    }
    let mut seen = HashSet::with_capacity(parent_ids.len());
    for parent_id in parent_ids {
        if !seen.insert(parent_id) {
            return Err(format!(
                "{operation} does not accept repeated parent ID '{parent_id}' because the graph stores one edge per parent"
            ));
        }
    }
    Ok(())
}
