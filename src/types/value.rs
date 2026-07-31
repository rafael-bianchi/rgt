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
            ValueData::Duration(dur) => format!("{}s", dur.num_seconds()),
        }
    }

    #[allow(dead_code)]
    pub fn date_diff(date1: &DateTime<Utc>, date2: &DateTime<Utc>) -> Duration {
        *date2 - *date1
    }
}
