use crate::types::ValueData;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookToolInput {
    pub path: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookToolResponse {
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPayload {
    pub event: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<HookToolInput>,
    pub tool_response: Option<HookToolResponse>,
}

pub struct ExtractedValue {
    pub value: ValueData,
    pub line_number: Option<u32>,
}

/// Parses a hook event JSON payload from stdin into a structured `HookPayload`.
pub fn parse_hook_payload(json_str: &str) -> Result<HookPayload, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Extracts numeric values and ISO-8601 dates from text content, line by line.
///
/// Returns the extracted values with their source line numbers (1-based).
pub fn extract_values_from_content(content: &str) -> Vec<ExtractedValue> {
    let mut results = Vec::new();
    let iso_date_regex =
        Regex::new(r"\b(\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)?)\b").unwrap();
    let number_regex = Regex::new(r"\b(\d+(?:\.\d+)?)\b").unwrap();

    for (idx, line) in content.lines().enumerate() {
        let line_num = (idx + 1) as u32;

        for cap in iso_date_regex.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                let s = m.as_str();
                let dt = if s.contains('T') {
                    DateTime::parse_from_rfc3339(s)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                } else {
                    DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", s))
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                };

                if let Some(valid_dt) = dt {
                    results.push(ExtractedValue {
                        value: ValueData::Date(valid_dt),
                        line_number: Some(line_num),
                    });
                }
            }
        }

        for cap in number_regex.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                if let Ok(num) = m.as_str().parse::<f64>() {
                    results.push(ExtractedValue {
                        value: ValueData::Number(num),
                        line_number: Some(line_num),
                    });
                }
            }
        }
    }

    results
}
