use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValueKind {
    Number,
    Date,
    Duration,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueKind::Number => write!(f, "NUMBER"),
            ValueKind::Date => write!(f, "DATE"),
            ValueKind::Duration => write!(f, "DURATION"),
        }
    }
}

impl std::str::FromStr for ValueKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NUMBER" => Ok(ValueKind::Number),
            "DATE" => Ok(ValueKind::Date),
            "DURATION" => Ok(ValueKind::Duration),
            _ => Err(format!("Unknown value kind: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValueData {
    Number(f64),
    Date(DateTime<Utc>),
    Duration(Duration),
}

impl ValueData {
    pub fn kind(&self) -> ValueKind {
        match self {
            ValueData::Number(_) => ValueKind::Number,
            ValueData::Date(_) => ValueKind::Date,
            ValueData::Duration(_) => ValueKind::Duration,
        }
    }

    pub fn to_string_repr(&self) -> String {
        match self {
            ValueData::Number(n) => n.to_string(),
            ValueData::Date(d) => d.to_rfc3339(),
            ValueData::Duration(dur) => {
                let total_secs = dur.num_seconds();
                if total_secs == 0 {
                    return "0s".to_string();
                }
                let sign = if total_secs < 0 { "-" } else { "" };
                let abs_secs = total_secs.unsigned_abs() as i64;
                let days = abs_secs / 86_400;
                let hours = (abs_secs % 86_400) / 3_600;
                let mins = (abs_secs % 3_600) / 60;
                let secs = abs_secs % 60;
                if days > 0 {
                    if hours == 0 && mins == 0 {
                        format!("{}{}d", sign, days)
                    } else if mins == 0 {
                        format!("{}{}d {}h", sign, days, hours)
                    } else {
                        format!("{}{}d {}h {}m", sign, days, hours, mins)
                    }
                } else if hours > 0 {
                    if mins == 0 && secs == 0 {
                        format!("{}{}h", sign, hours)
                    } else if secs == 0 {
                        format!("{}{}h {}m", sign, hours, mins)
                    } else {
                        format!("{}{}h {}m {}s", sign, hours, mins, secs)
                    }
                } else if mins > 0 {
                    if secs == 0 {
                        format!("{}{}m", sign, mins)
                    } else {
                        format!("{}{}m {}s", sign, mins, secs)
                    }
                } else {
                    format!("{}{}s", sign, secs)
                }
            }
        }
    }

    /// Computes the difference between two dates as a `chrono::Duration`.
    ///
    /// **Year/month precision limitation**: `chrono::Duration` has no `num_years()`
    /// or `num_months()` methods because years and months are variable-length
    /// calendar units (leap years, month length variations). Callers needing
    /// year/month display should use calendar-aware methods like
    /// `chrono::NaiveDate::years_since()` on the original `Date` nodes rather
    /// than converting the derived `Duration` node.
    #[allow(dead_code)]
    pub fn date_diff(date1: &DateTime<Utc>, date2: &DateTime<Utc>) -> Duration {
        *date2 - *date1
    }
}

/// Returns a Duration's canonical whole seconds, rejecting subsecond values
/// instead of truncating them through `num_seconds()`.
pub(crate) fn exact_duration_seconds(duration: &Duration) -> Result<i64, String> {
    let seconds = duration.num_seconds();
    if *duration != Duration::seconds(seconds) {
        return Err("Duration must represent a whole number of seconds".into());
    }
    Ok(seconds)
}
