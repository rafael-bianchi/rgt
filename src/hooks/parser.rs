use crate::types::ValueData;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Locale-aware number parsing mode (FR-002). `Auto` is the default and infers
/// the concrete format once per file (FR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberFormat {
    /// `.` decimal, `,` thousands.
    Us,
    /// `,` decimal, `.` thousands.
    Eu,
    /// Infer from content: rightmost separator is the decimal when both `.`
    /// and `,` appear in a number; a single separator falls back to `Us`.
    #[default]
    Auto,
}

impl NumberFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            NumberFormat::Us => "us",
            NumberFormat::Eu => "eu",
            NumberFormat::Auto => "auto",
        }
    }
}

impl FromStr for NumberFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "us" => Ok(NumberFormat::Us),
            "eu" => Ok(NumberFormat::Eu),
            "auto" => Ok(NumberFormat::Auto),
            _ => Err(format!(
                "invalid --number-format '{}': expected one of us, eu, auto",
                s
            )),
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct ExtractedValue {
    pub value: ValueData,
    pub line_number: Option<u32>,
    /// 0-based index of this value within its line's emitted values (dates
    /// first, then non-masked numbers). Disambiguates identical values on the
    /// same line (FR-002).
    pub occurrence: u32,
}

/// Result of a content scan: the extracted values plus the count of number
/// candidates rejected as malformed under the active format (FR-005).
#[derive(Debug, Clone, Default)]
pub struct ExtractionResult {
    pub values: Vec<ExtractedValue>,
    pub skipped_malformed: usize,
}

impl From<ExtractionResult> for Vec<ExtractedValue> {
    fn from(r: ExtractionResult) -> Self {
        r.values
    }
}

/// Normalized capture model consumed by `handle_passive_hook_event`.
#[derive(Debug, Clone, Default)]
pub struct NormalizedCapture {
    /// Legacy alias retained for existing callers; equals `read_target`.
    pub path: Option<String>,
    pub read_target: Option<String>,
    pub phase: CapturePhase,
    pub outcome_evidence: OutcomeEvidence,
    /// Full host-returned text; this is never populated by reading the file.
    pub completed_content: Option<String>,
    /// 1-based original lines corresponding to `completed_content`.
    pub source_line_mapping: Option<Vec<u32>>,
    pub content_complete: bool,
    /// Legacy alias retained for current extraction code.
    #[allow(dead_code)]
    pub content: Option<String>,
    /// Base64 PDF envelope bytes (spec 033). When present, the hook decodes and
    /// locally extracts the PDF instead of falling back to a disk read.
    pub pdf_base64: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CapturePhase {
    Before,
    After,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutcomeEvidence {
    ExplicitSuccess,
    CompletedResult,
    Failed,
    Canceled,
    #[default]
    Unknown,
}

impl NormalizedCapture {
    /// Checks whether the event provides a completed full result. The capture
    /// path still has to match the returned bytes against the current snapshot.
    pub fn is_value_capture_eligible(&self) -> bool {
        self.phase == CapturePhase::After
            && matches!(
                self.outcome_evidence,
                OutcomeEvidence::ExplicitSuccess | OutcomeEvidence::CompletedResult
            )
            && self.content_complete
            && (self
                .completed_content
                .as_ref()
                .is_some_and(|_| self.source_line_mapping.is_some())
                || self.pdf_base64.is_some())
    }
}

/// Parses a hook event JSON payload from stdin into a structured `HookPayload`.
pub fn parse_hook_payload(json_str: &str) -> Result<HookPayload, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Extracts numeric values and ISO-8601 dates from text content, line by line.
///
/// Returns the extracted values with their source line numbers (1-based) and a
/// per-line occurrence index, plus a count of number candidates rejected as
/// malformed under `format` (FR-005). Recognized ISO-8601 dates emit exactly
/// one `Date` each, and their digit components are masked from the number scan
/// so no spurious `Number` nodes leak from a date's year/month/day.
///
/// Number parsing is locale-aware per `format` (FR-002): a leading `-`/`+` and
/// an accounting `(N)` pair are part of the number (FR-001), and separators are
/// normalized against the format's decimal/thousands convention. `Auto` infers
/// the concrete format once per file (FR-004).
pub fn extract_values_from_content(content: &str, format: NumberFormat) -> ExtractionResult {
    let mut result = ExtractionResult::default();
    let iso_date_regex = Regex::new(
        r"\b(\d{4}-\d{2}-\d{2}(?:[Tt]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:[Zz]|[+-]\d{2}:?\d{2})?)?)\b",
    )
    .unwrap();
    let compact_offset_regex = Regex::new(r"([+-]\d{2})(\d{2})$").unwrap();
    let url_regex = Regex::new(r"https?://\S+").unwrap();
    let time_regex = Regex::new(r"\b\d{1,2}:\d{2}(?::\d{2}(?:\.\d+)?)?\b").unwrap();
    let slash_date_regex = Regex::new(r"\b\d{1,2}/\d{1,2}/\d{2,4}\b").unwrap();
    let uuid_hex_regex = Regex::new(r"\b[0-9a-fA-F]{3,}(?:-[0-9a-fA-F]{3,}){3,}\b").unwrap();
    let hex_literal_regex = Regex::new(r"\b0x[0-9a-fA-F]+\b").unwrap();
    let v_version_regex = Regex::new(r"\bv\d+(?:\.\d+)+\b").unwrap();
    let ipv4_regex = Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap();

    // FR-004: resolve `Auto` once per file, then reuse the concrete format.
    let format = match format {
        NumberFormat::Auto => resolve_auto_format(content),
        other => other,
    };
    let number_regex = Regex::new(r"(\d[\d.,]*(?:[eE][+-]?\d+)?)").unwrap();

    for (idx, line) in content.lines().enumerate() {
        let line_num = (idx + 1) as u32;
        let mut line_occurrences: std::collections::HashMap<
            (crate::types::value::ValueKind, String),
            u32,
        > = std::collections::HashMap::new();

        // Recognize dates and mask spans so the number scan skips URLs, timestamps,
        // slash dates, UUIDs, version strings, IPv4 addresses, and date components.
        let mut mask_spans: Vec<(usize, usize)> = Vec::new();
        for cap in iso_date_regex.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                let s = m.as_str();
                mask_spans.push((m.start(), m.end()));
                let dt = if s.contains(['T', 't']) {
                    let s_norm = if let Some(cap_offset) = compact_offset_regex.captures(s) {
                        let full = cap_offset.get(0).unwrap().as_str();
                        let prefix = &s[..s.len() - full.len()];
                        format!("{}{}:{}", prefix, &cap_offset[1], &cap_offset[2])
                    } else {
                        s.to_string()
                    };
                    DateTime::parse_from_rfc3339(&s_norm)
                        .map(|d| d.with_timezone(&Utc))
                        .or_else(|_| {
                            DateTime::parse_from_rfc3339(&format!("{}Z", s_norm))
                                .map(|d| d.with_timezone(&Utc))
                        })
                        .ok()
                } else {
                    DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", s))
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                };

                if let Some(valid_dt) = dt {
                    let val = ValueData::Date(valid_dt);
                    let key = (val.kind(), val.to_string_repr());
                    let occurrence = *line_occurrences.get(&key).unwrap_or(&0);
                    line_occurrences.insert(key, occurrence + 1);
                    result.values.push(ExtractedValue {
                        value: val,
                        line_number: Some(line_num),
                        occurrence,
                    });
                } else {
                    result.skipped_malformed += 1;
                }
            }
        }

        for m in url_regex.find_iter(line) {
            mask_spans.push((m.start(), m.end()));
        }
        for m in time_regex.find_iter(line) {
            mask_spans.push((m.start(), m.end()));
        }
        for m in slash_date_regex.find_iter(line) {
            mask_spans.push((m.start(), m.end()));
        }
        for m in uuid_hex_regex.find_iter(line) {
            let s = m.as_str();
            if s.contains(|c: char| c.is_ascii_alphabetic()) {
                mask_spans.push((m.start(), m.end()));
            }
        }
        for m in hex_literal_regex.find_iter(line) {
            mask_spans.push((m.start(), m.end()));
        }
        for m in v_version_regex.find_iter(line) {
            mask_spans.push((m.start(), m.end()));
        }
        for m in ipv4_regex.find_iter(line) {
            let parts: Vec<&str> = m.as_str().split('.').collect();
            if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
                mask_spans.push((m.start(), m.end()));
            }
        }

        let mut masked_line = line.as_bytes().to_vec();
        for (start, end) in &mask_spans {
            if *start < masked_line.len() && *end <= masked_line.len() {
                for b in &mut masked_line[*start..*end] {
                    *b = b' ';
                }
            }
        }
        let masked_line_str = std::str::from_utf8(&masked_line).unwrap_or(line);

        for cap in number_regex.captures_iter(masked_line_str) {
            if let Some(m) = cap.get(1) {
                // Must not be preceded by a word character (e.g. `foo123`), unless it's a sign/context handled later
                if m.start() > 0
                    && line[..m.start()]
                        .chars()
                        .last()
                        .map(|c| c.is_alphanumeric() || c == '_')
                        .unwrap_or(false)
                {
                    continue;
                }

                // Skip any number match that overlaps with a masked span.
                if mask_spans
                    .iter()
                    .any(|(s, e)| m.start() < *e && m.end() > *s)
                {
                    continue;
                }

                // Trim trailing commas or dots (e.g. from sentence period or trailing punctuation)
                let trimmed = m.as_str().trim_end_matches(['.', ',']);
                if trimmed.is_empty() {
                    continue;
                }
                let end = m.start() + trimmed.len();

                // Reject hex/UUID/hash/ordinal tokens (e.g. `96a3adc0`, `7e3a9f1`, `0x1a2b`, `1st`, `4faf`)
                // while still permitting measurement/time/computing unit suffixes (`12.5kg`, `99.9ms`, `100GB`).
                let after = &line[end..];
                if let Some(first_after) = after.chars().next() {
                    if first_after.is_alphanumeric() || first_after == '_' {
                        let word_suffix: String = after
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        if !is_allowed_unit_suffix(&word_suffix) || trimmed.contains(['e', 'E']) {
                            continue;
                        }
                    }
                }
                // Reject UUID/hex middle segments like `-81e6-` in `96a3adc0-f76e-4faf-81e6-5b0578c2c46d`
                if trimmed.contains(['e', 'E']) && !trimmed.contains('.') {
                    let before_hyphen_word = line[..m.start()]
                        .strip_suffix('-')
                        .and_then(|b| b.chars().last())
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
                    if before_hyphen_word {
                        continue;
                    }
                }

                // Attach sign, currency context, accounting parens
                let (prefix, suffix, has_currency, preceded_by_comma) =
                    candidate_context_ext(line, m.start(), end);
                let has_line_whitespace = line.contains([' ', '\t']);

                match process_candidate(
                    &prefix,
                    trimmed,
                    &suffix,
                    format,
                    has_currency,
                    preceded_by_comma,
                    has_line_whitespace,
                ) {
                    ProcessedCandidate::Single(num) => {
                        let val = ValueData::Number(num);
                        let key = (val.kind(), val.to_string_repr());
                        let occurrence = *line_occurrences.get(&key).unwrap_or(&0);
                        line_occurrences.insert(key, occurrence + 1);
                        result.values.push(ExtractedValue {
                            value: val,
                            line_number: Some(line_num),
                            occurrence,
                        });
                    }
                    ProcessedCandidate::Split(nums) => {
                        for num in nums {
                            let val = ValueData::Number(num);
                            let key = (val.kind(), val.to_string_repr());
                            let occurrence = *line_occurrences.get(&key).unwrap_or(&0);
                            line_occurrences.insert(key, occurrence + 1);
                            result.values.push(ExtractedValue {
                                value: val,
                                line_number: Some(line_num),
                                occurrence,
                            });
                        }
                    }
                    ProcessedCandidate::Malformed => {
                        result.skipped_malformed += 1;
                    }
                }
            }
        }
    }

    result
}

fn is_allowed_unit_suffix(suffix: &str) -> bool {
    if !suffix.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    matches!(
        suffix.to_ascii_lowercase().as_str(),
        "s" | "ms"
            | "us"
            | "ns"
            | "m"
            | "min"
            | "mins"
            | "h"
            | "hr"
            | "hrs"
            | "d"
            | "w"
            | "wk"
            | "wks"
            | "y"
            | "yr"
            | "yrs"
            | "g"
            | "kg"
            | "mg"
            | "ug"
            | "lb"
            | "lbs"
            | "oz"
            | "km"
            | "cm"
            | "mm"
            | "um"
            | "nm"
            | "mi"
            | "ft"
            | "in"
            | "yd"
            | "l"
            | "ml"
            | "b"
            | "kb"
            | "mb"
            | "gb"
            | "tb"
            | "pb"
            | "kib"
            | "mib"
            | "gib"
            | "tib"
            | "hz"
            | "khz"
            | "mhz"
            | "ghz"
            | "px"
            | "pt"
            | "em"
            | "rem"
            | "vw"
            | "vh"
            | "v"
            | "kv"
            | "mv"
            | "kw"
            | "mw"
            | "ma"
            | "c"
            | "k"
    )
}

enum ProcessedCandidate {
    Single(f64),
    Split(Vec<f64>),
    Malformed,
}

fn is_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() == 4 {
        parts.iter().all(|p| p.parse::<u8>().is_ok())
    } else {
        false
    }
}

fn process_candidate(
    prefix: &str,
    raw: &str,
    suffix: &str,
    format: NumberFormat,
    has_currency: bool,
    preceded_by_comma: bool,
    has_line_whitespace: bool,
) -> ProcessedCandidate {
    if is_ipv4(raw) {
        return ProcessedCandidate::Malformed;
    }

    let (decimal, thousands) = match format {
        NumberFormat::Us => ('.', ','),
        NumberFormat::Eu => (',', '.'),
        NumberFormat::Auto => unreachable!("Auto resolved before candidate processing"),
    };

    // Scientific notation check (only when not a comma/dot-separated multi-token row)
    if raw.contains(['e', 'E']) && !raw.contains(thousands) {
        let has_minus = prefix.starts_with('-');
        let has_parens = prefix.contains('(') && suffix.contains(')');
        let negative = has_minus ^ has_parens;

        let s = if decimal != '.' {
            raw.replace(decimal, ".")
        } else {
            raw.to_string()
        };
        return match s.parse::<f64>() {
            Ok(mut val) if val.is_finite() => {
                if negative {
                    val = -val;
                }
                if val == 0.0 && val.is_sign_negative() {
                    val = 0.0;
                }
                ProcessedCandidate::Single(val)
            }
            _ => ProcessedCandidate::Malformed,
        };
    }

    if decimal == '.' {
        // US format convention: '.' is decimal, ',' is thousands
        if raw.contains(',') {
            // Check if valid single thousands-grouped number
            let is_valid_single = if preceded_by_comma {
                false
            } else if let Some(dot_idx) = raw.find('.') {
                let after_dot = &raw[dot_idx + 1..];
                if after_dot.contains('.')
                    || after_dot.contains(',')
                    || after_dot.is_empty()
                    || !after_dot.chars().all(|c| c.is_ascii_digit())
                {
                    false
                } else {
                    let int_part = &raw[..dot_idx];
                    let groups: Vec<&str> = int_part.split(',').collect();
                    (1..=3).contains(&groups[0].len())
                        && !groups[0].starts_with('0')
                        && groups[0].chars().all(|c| c.is_ascii_digit())
                        && groups[1..]
                            .iter()
                            .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
                }
            } else {
                let groups: Vec<&str> = raw.split(',').collect();
                let valid_thousands_shape = groups.len() >= 2
                    && (1..=3).contains(&groups[0].len())
                    && !groups[0].starts_with('0')
                    && groups[0].chars().all(|c| c.is_ascii_digit())
                    && groups[1..]
                        .iter()
                        .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()));
                if !valid_thousands_shape {
                    false
                } else if groups.len() == 2 {
                    true
                } else {
                    // For 3+ groups without a decimal point (e.g. `1,000,000` vs `10,200,300`),
                    // treat as a single number when in currency/prose context or when any
                    // thousands group is `"000"`; treat bare whitespace-free CSV rows as split.
                    has_currency || has_line_whitespace || groups[1..].contains(&"000")
                }
            };

            if is_valid_single {
                return match normalize_number(prefix, raw, suffix, decimal, thousands) {
                    Some(num) => ProcessedCandidate::Single(num),
                    None => ProcessedCandidate::Malformed,
                };
            }

            // Conflicting EU-formatted number (e.g. 1.234,56 or 1.234.567,89) must not be split
            let eu_fmt_regex = Regex::new(r"^\d{1,3}(\.\d{3})+,\d+$").unwrap();
            if eu_fmt_regex.is_match(raw) {
                return ProcessedCandidate::Malformed;
            }

            // In prose or currency context, non-thousands single comma tokens (e.g. 12,34 in Price: 12,34 EUR)
            // must be rejected as Malformed rather than split into CSV items.
            if (has_line_whitespace || has_currency) && !raw.contains('.') {
                let pieces: Vec<&str> = raw.split(',').collect();
                if pieces.len() == 2 && pieces[1].len() != 3 {
                    return ProcessedCandidate::Malformed;
                }
            }

            // Not a valid single formatted number: check if unspaced CSV row
            let pieces: Vec<&str> = raw.split(',').collect();
            let mut parsed_pieces = Vec::new();
            let has_minus = prefix.starts_with('-');
            let has_parens = prefix.contains('(') && suffix.contains(')');
            let negative = has_minus ^ has_parens;

            for (i, p) in pieces.iter().enumerate() {
                if p.is_empty() {
                    return ProcessedCandidate::Malformed;
                }
                match p.parse::<f64>() {
                    Ok(mut val) if val.is_finite() => {
                        if i == 0 && negative {
                            val = -val;
                        }
                        if val == 0.0 && val.is_sign_negative() {
                            val = 0.0;
                        }
                        parsed_pieces.push(val);
                    }
                    _ => return ProcessedCandidate::Malformed,
                }
            }

            return ProcessedCandidate::Split(parsed_pieces);
        }
    } else {
        // EU format convention: ',' is decimal, '.' is thousands
        if raw.contains('.') {
            // Check if valid single thousands-grouped number
            let is_valid_single = if let Some(comma_idx) = raw.find(',') {
                let after_comma = &raw[comma_idx + 1..];
                if after_comma.contains('.')
                    || after_comma.contains(',')
                    || after_comma.is_empty()
                    || !after_comma.chars().all(|c| c.is_ascii_digit())
                {
                    false
                } else {
                    let int_part = &raw[..comma_idx];
                    let groups: Vec<&str> = int_part.split('.').collect();
                    (1..=3).contains(&groups[0].len())
                        && !groups[0].starts_with('0')
                        && groups[0].chars().all(|c| c.is_ascii_digit())
                        && groups[1..]
                            .iter()
                            .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()))
                }
            } else {
                let groups: Vec<&str> = raw.split('.').collect();
                let valid_thousands_shape = groups.len() >= 2
                    && (1..=3).contains(&groups[0].len())
                    && !groups[0].starts_with('0')
                    && groups[0].chars().all(|c| c.is_ascii_digit())
                    && groups[1..]
                        .iter()
                        .all(|g| g.len() == 3 && g.chars().all(|c| c.is_ascii_digit()));
                if !valid_thousands_shape {
                    false
                } else if groups.len() == 2 {
                    true
                } else {
                    // For 3+ groups without a decimal comma (e.g. `1.000.000` vs `1.100.200`),
                    // treat as a single number only when in currency context or when any
                    // thousands group is `"000"`; reject version strings like `1.100.200`.
                    has_currency || groups[1..].contains(&"000")
                }
            };

            if is_valid_single {
                return match normalize_number(prefix, raw, suffix, decimal, thousands) {
                    Some(num) => ProcessedCandidate::Single(num),
                    None => ProcessedCandidate::Malformed,
                };
            }

            // Conflicting US-formatted number (e.g. 1,234.56 or 1,234,567.89) must not be split
            let us_fmt_regex = Regex::new(r"^\d{1,3}(,\d{3})+\.\d+$").unwrap();
            if us_fmt_regex.is_match(raw) {
                return ProcessedCandidate::Malformed;
            }
        }
    }

    match normalize_number(prefix, raw, suffix, decimal, thousands) {
        Some(num) => ProcessedCandidate::Single(num),
        None => ProcessedCandidate::Malformed,
    }
}

/// Expands a numeric core span into its optional sign/parentheses context:
/// returns `(prefix, suffix, has_currency, preceded_by_comma)`.
fn candidate_context_ext(line: &str, start: usize, end: usize) -> (String, String, bool, bool) {
    let is_currency = |c: char| matches!(c, '$' | '€' | '£' | '¥');
    let before = &line[..start];

    // Trim single preceding currency symbol if present
    let mut has_currency = false;
    let before_trimmed_curr = if let Some(last_char) = before.chars().last() {
        if is_currency(last_char) {
            has_currency = true;
            &before[..before.len() - last_char.len_utf8()]
        } else {
            before
        }
    } else {
        before
    };

    let sign_chars: Vec<char> = before_trimmed_curr
        .chars()
        .rev()
        .take_while(|&c| matches!(c, '-' | '+' | '('))
        .collect();

    let sign_byte_len: usize = sign_chars.iter().map(|c| c.len_utf8()).sum();
    let rem = &before_trimmed_curr[..before_trimmed_curr.len() - sign_byte_len];
    if !has_currency && rem.chars().last().is_some_and(is_currency) {
        has_currency = true;
    }
    let preceded_by_comma = rem.ends_with(',');
    let preceded_by_word = rem
        .chars()
        .last()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    let preceded_by_expr_closer = rem
        .chars()
        .last()
        .map(|c| matches!(c, ')' | ']'))
        .unwrap_or(false);

    let prefix = if preceded_by_word || preceded_by_expr_closer {
        String::new()
    } else if sign_chars.iter().filter(|&&c| c == '-').count() >= 2 {
        // Multi-hyphen CLI flags (e.g. --100 or ---50) are not unary minus
        String::new()
    } else {
        sign_chars.into_iter().rev().collect()
    };

    let mut suffix = String::new();
    if end < line.len() && prefix.contains('(') && line[end..].starts_with(')') {
        suffix.push(')');
    }

    (prefix, suffix, has_currency, preceded_by_comma)
}

/// Normalizes a captured number candidate into an `f64` under the active
/// separator convention, or `None` when the candidate is malformed (FR-005).
///
/// Accounting parentheses and a leading `-` denote a negative; `-(N)` (a minus
/// immediately before parentheses) yields a positive magnitude. Separators are
/// stripped with validation: at most one decimal separator and every thousands
/// group must be exactly 3 digits.
fn normalize_number(
    prefix: &str,
    num: &str,
    suffix: &str,
    decimal: char,
    thousands: char,
) -> Option<f64> {
    let has_minus = prefix.starts_with('-');
    let has_parens = prefix.contains('(') && suffix.contains(')');
    let negative = has_minus ^ has_parens;

    let mut cleaned = String::new();
    let mut decimal_seen = false;
    let chars: Vec<char> = num.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == decimal {
            if decimal_seen {
                return None;
            }
            decimal_seen = true;
            cleaned.push('.');
            i += 1;
        } else if c == thousands {
            if decimal_seen {
                return None;
            }
            // The group of digits after a thousands separator must be exactly 3.
            let mut count = 0;
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                count += 1;
                j += 1;
            }
            if count != 3 {
                return None;
            }
            i += 1;
        } else if c.is_ascii_digit() {
            cleaned.push(c);
            i += 1;
        } else {
            return None;
        }
    }

    let mut value = cleaned.parse::<f64>().ok()?;
    if !value.is_finite() {
        return None;
    }
    if negative {
        value = -value;
    }
    if value == 0.0 && value.is_sign_negative() {
        value = 0.0;
    }
    Some(value)
}

/// Infers a concrete format for `Auto` by scanning the content once (FR-004):
/// the first number candidate containing both `.` and `,` determines the
/// decimal as its rightmost separator; when no candidate contains both, the
/// format falls back to `Us`.
fn resolve_auto_format(content: &str) -> NumberFormat {
    let candidate_regex = Regex::new(r"\d[\d.,]*\d").unwrap();
    let us_fmt_regex = Regex::new(r"^\d{1,3}(,\d{3})+\.\d+$").unwrap();
    let eu_fmt_regex = Regex::new(r"^\d{1,3}(\.\d{3})+,\d+$").unwrap();

    for line in content.lines() {
        for cap in candidate_regex.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                let s = m.as_str();
                if s.contains('.') && s.contains(',') {
                    if us_fmt_regex.is_match(s) {
                        return NumberFormat::Us;
                    }
                    if eu_fmt_regex.is_match(s) {
                        return NumberFormat::Eu;
                    }
                }
            }
        }
    }

    // Secondary scan: detect unambiguous European decimal tokens in prose (e.g. `Price: 12,34 EUR`)
    let eu_decimal_regex = Regex::new(r"\b\d+,\d{1,2}\b").unwrap();
    for line in content.lines() {
        if line.contains([' ', '\t']) && eu_decimal_regex.is_match(line) {
            return NumberFormat::Eu;
        }
    }

    NumberFormat::Us
}

/// Unwraps line-numbered tool output (such as Claude Code's cat -n style
/// `^\s*\d+[→\t]` and OpenCode's `<file>\n00001| ...` or `1: ...`) to raw
/// source text, returning the unwrapped text and a mapping of 1-based line numbers.
pub fn unwrap_line_numbered_content(raw: &str) -> (String, Vec<u32>) {
    let raw_trimmed = raw.trim();
    if raw_trimmed.is_empty() {
        return (raw.to_string(), Vec::new());
    }

    let mut lines_to_process: Vec<&str> = raw.lines().collect();
    if lines_to_process.first().is_some_and(|l| {
        let t = l.trim();
        (t.starts_with("<file") || t == "<content>") && t.ends_with('>')
    }) {
        lines_to_process.remove(0);
    }
    if lines_to_process.last().is_some_and(|l| {
        let t = l.trim();
        t == "</file>" || t == "</content>"
    }) {
        lines_to_process.pop();
    }

    let claude_re = Regex::new(r"^\s*(\d+)[\u{2192}\t](.*)$").unwrap();
    let pipe_re = Regex::new(r"^\s*(\d+)\|(?: (.*)|(.*))$").unwrap();
    let colon_re = Regex::new(r"^\s*(\d+):\s?(.*)$").unwrap();

    let mut is_claude = true;
    let mut is_pipe = true;
    let mut is_colon = true;
    let mut non_empty_count = 0;

    for line in &lines_to_process {
        if line.trim().is_empty() {
            continue;
        }
        non_empty_count += 1;
        if !claude_re.is_match(line) {
            is_claude = false;
        }
        if !pipe_re.is_match(line) {
            is_pipe = false;
        }
        if !colon_re.is_match(line) {
            is_colon = false;
        }
    }

    let active_re = if non_empty_count > 0 && is_claude {
        Some(&claude_re)
    } else if non_empty_count > 0 && is_pipe {
        Some(&pipe_re)
    } else if non_empty_count > 0 && is_colon {
        Some(&colon_re)
    } else {
        None
    };

    if let Some(re) = active_re {
        let mut unwrapped_lines = Vec::new();
        let mut line_mapping = Vec::new();
        for line in lines_to_process {
            if let Some(caps) = re.captures(line) {
                let line_num: u32 = caps
                    .get(1)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                unwrapped_lines.push(body.to_string());
                line_mapping.push(line_num);
            } else if line.trim().is_empty() {
                unwrapped_lines.push(String::new());
                let next_num = line_mapping.last().copied().unwrap_or(0) + 1;
                line_mapping.push(next_num);
            } else {
                unwrapped_lines.push(line.to_string());
                let next_num = line_mapping.last().copied().unwrap_or(0) + 1;
                line_mapping.push(next_num);
            }
        }
        let mut unwrapped = unwrapped_lines.join("\n");
        if raw.ends_with('\n') {
            unwrapped.push('\n');
        }
        (unwrapped, line_mapping)
    } else {
        let total_lines = raw.lines().count().max(1) as u32;
        (raw.to_string(), (1..=total_lines).collect())
    }
}

/// Normalizes relative and absolute paths for comparison (FR-001).
pub fn paths_match(p1: &str, p2: &str) -> bool {
    fn clean(s: &str) -> &str {
        s.strip_prefix("./")
            .or_else(|| s.strip_prefix(".\\"))
            .unwrap_or(s)
    }
    let c1 = clean(p1);
    let c2 = clean(p2);
    if c1 == c2 {
        return true;
    }
    if c1.ends_with(&format!("/{c2}")) || c1.ends_with(&format!("\\{c2}")) {
        return true;
    }
    if c2.ends_with(&format!("/{c1}")) || c2.ends_with(&format!("\\{c1}")) {
        return true;
    }
    std::path::Path::new(c1) == std::path::Path::new(c2)
}

fn extract_command_str(value: &serde_json::Value) -> Option<String> {
    let candidates = [
        value.pointer("/tool_input/command"),
        value.pointer("/tool_input/cmd"),
        value.pointer("/args/command"),
        value.pointer("/args/cmd"),
        value.get("command"),
        value.get("cmd"),
    ];
    for cand in candidates.into_iter().flatten() {
        if let Some(s) = cand.as_str() {
            return Some(s.to_string());
        }
        if let Some(arr) = cand.as_array() {
            let str_tokens: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            if str_tokens.len() >= 3
                && (str_tokens[0] == "bash" || str_tokens[0] == "sh")
                && str_tokens[1].starts_with('-')
                && str_tokens[1].contains('c')
            {
                return Some(str_tokens[2].to_string());
            }
            let joined = str_tokens
                .iter()
                .map(|t| {
                    if t.contains(' ') {
                        format!("\"{}\"", t)
                    } else {
                        t.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            if !joined.is_empty() {
                return Some(joined);
            }
        }
    }
    None
}

fn extract_cwd(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/tool_input/cwd")
        .or_else(|| value.pointer("/args/cwd"))
        .or_else(|| value.get("cwd"))
        .and_then(|c| c.as_str())
        .map(str::to_string)
}

/// Normalizes an agent's native stdin event JSON into the internal
/// `{path, content}` capture model. Returns `None` when the event is malformed
/// or does not reference a capturable file, so callers can fail open.
///
/// `agent` selects the stdin dialect; `None`/`claude-code`/`cursor` use the
/// default Claude Code compatible shape (existing behavior).
pub fn normalize_agent_event(agent: Option<&str>, stdin: &str) -> Option<NormalizedCapture> {
    normalize_agent_event_with_phase(agent, stdin, None)
}

/// Normalizes an agent's native stdin event JSON with optional fallback
/// `CapturePhase` from the CLI `event_type` argument.
pub fn normalize_agent_event_with_phase(
    agent: Option<&str>,
    stdin: &str,
    default_phase: Option<CapturePhase>,
) -> Option<NormalizedCapture> {
    let stdin = stdin.strip_prefix('\u{feff}').unwrap_or(stdin);
    let value: serde_json::Value = serde_json::from_str(stdin).ok()?;
    let mut capture = match agent {
        None | Some("claude-code") | Some("cursor") | Some("codex") => {
            match default_dialect(&value) {
                Some(c) if c.path.is_some() => Some(c),
                _ => command_dialect(&value),
            }
        }
        Some("windsurf") => windsurf_dialect(&value),
        Some("copilot") => copilot_dialect(&value),
        Some("opencode") => {
            opencode_native_read_dialect(&value).or_else(|| command_dialect(&value))
        }
        Some("gemini") | Some("vibe") | Some("pi") | Some("hermes") => command_dialect(&value),
        Some(_) => None,
    }?;

    if capture.phase == CapturePhase::Unknown {
        if let Some(phase) = default_phase {
            capture.phase = phase;
        }
    }

    if matches!(agent, None | Some("claude-code") | Some("codex"))
        && !is_qualifying_read_event(agent, &value, &capture)
    {
        capture.content_complete = false;
    }
    Some(capture)
}

/// OpenCode's observed `read` result exposes the complete decoded file text
/// in `metadata.display.text` or `result.output`. Only a full-file, untruncated after-result is
/// eligible; the caller still verifies those bytes against the disk snapshot.
fn opencode_native_read_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    if !is_opencode_native_read(value) {
        return None;
    }
    let path = value.pointer("/args/filePath")?.as_str()?;
    let result = value.get("result")?;
    let metadata = result.get("metadata")?;
    let display = metadata.get("display")?;
    let text = display.get("text").and_then(|t| t.as_str());
    let output = result.get("output").and_then(|o| o.as_str());
    let raw_text = text.or(output)?;

    let line_start = display.get("lineStart")?.as_u64()?;
    let line_end = display.get("lineEnd")?.as_u64()?;
    let total_lines = display.get("totalLines")?.as_u64()?;
    let (unwrapped_content, line_mapping) = unwrap_line_numbered_content(raw_text);
    let line_count = u32::try_from(unwrapped_content.lines().count()).ok()?;
    let split_line_count = u64::try_from(unwrapped_content.split('\n').count()).ok()?;
    let line_count_u64 = u64::from(line_count);
    let lines_match = line_count_u64 == total_lines
        || line_count_u64 + 1 == total_lines
        || split_line_count == total_lines
        || split_line_count + 1 == total_lines;

    let display_path = display.get("path")?.as_str()?;
    if path.is_empty()
        || [value, result]
            .into_iter()
            .any(opencode_outcome_is_negative)
        || metadata.get("truncated")?.as_bool()?
        || display.get("truncated")?.as_bool()?
        || display.get("type")?.as_str()? != "file"
        || !paths_match(display_path, path)
        || line_start != 1
        || line_end != total_lines
        || !lines_match
    {
        return None;
    }
    let cwd = extract_cwd(value);
    Some(NormalizedCapture {
        path: Some(path.to_string()),
        read_target: Some(path.to_string()),
        phase: CapturePhase::After,
        outcome_evidence: OutcomeEvidence::CompletedResult,
        completed_content: Some(unwrapped_content.clone()),
        source_line_mapping: Some(line_mapping),
        content_complete: true,
        content: Some(unwrapped_content),
        pdf_base64: None,
        cwd,
    })
}

fn opencode_outcome_is_negative(value: &serde_json::Value) -> bool {
    value.get("success").and_then(serde_json::Value::as_bool) == Some(false)
        || ["failed", "canceled", "cancelled", "is_error", "isError"]
            .into_iter()
            .any(|key| value.get(key).and_then(serde_json::Value::as_bool) == Some(true))
        || value
            .get("error")
            .is_some_and(|error| !error.is_null() && error != "")
}

fn is_opencode_native_read(value: &serde_json::Value) -> bool {
    value.get("event").and_then(serde_json::Value::as_str) == Some("tool.execute.after")
        && value.get("tool").and_then(serde_json::Value::as_str) == Some("read")
        && value
            .pointer("/args/filePath")
            .and_then(serde_json::Value::as_str)
            .is_some()
}

fn is_qualifying_read_event(
    agent: Option<&str>,
    value: &serde_json::Value,
    capture: &NormalizedCapture,
) -> bool {
    let tool = value
        .get("tool_name")
        .or_else(|| value.get("toolName"))
        .or_else(|| value.get("tool"))
        .and_then(serde_json::Value::as_str);
    match (agent, tool) {
        (None | Some("claude-code"), Some("Read")) => capture.read_target.is_some(),
        (None | Some("claude-code"), Some("Bash")) => {
            let command = extract_command_str(value);
            command.is_some_and(|command| extract_path_from_command(&command).is_some())
                && capture.read_target.is_some()
        }
        (Some("codex"), Some("read_file" | "ReadFile" | "Read")) => capture.read_target.is_some(),
        (
            Some("codex"),
            Some(
                "Bash" | "bash" | "execute_command" | "ExecuteCommand" | "exec" | "shell"
                | "local_shell" | "exec_command" | "container.exec",
            ),
        ) => {
            let command = extract_command_str(value);
            command.is_some_and(|command| extract_path_from_command(&command).is_some())
                && capture.read_target.is_some()
        }
        _ => false,
    }
}

/// Default dialect: `tool_input.path`/`file_path` + `tool_response.content`
/// (reuses the structured Claude Code compatible payload shape). When
/// `tool_response.content` is absent but the response carries the structured
/// binary envelope shape (`type == "pdf"` with a `file.base64` field — the
/// empirically captured shape for native PDF reads), the base64 is surfaced in
/// `pdf_base64` so the hook can decode and extract it locally (spec 033). The
/// shape is an undocumented internal detail, so any mismatch falls through with
/// the content-less capture unchanged (fail open).
fn extract_text_content(tool_response: &serde_json::Value) -> Option<String> {
    if let Some(c) = tool_response.get("content").and_then(|c| c.as_str()) {
        return Some(c.to_string());
    }
    if let Some(c) = tool_response
        .pointer("/file/content")
        .and_then(|c| c.as_str())
    {
        return Some(c.to_string());
    }
    if let Some(t) = tool_response.get("text").and_then(|t| t.as_str()) {
        return Some(t.to_string());
    }
    None
}

fn default_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    let payload = parse_hook_payload(&value.to_string()).ok()?;
    let path = payload
        .tool_input
        .as_ref()
        .and_then(|t| {
            t.path.clone().or_else(|| t.file_path.clone()).or_else(|| {
                value
                    .pointer("/tool_input/filePath")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
        })
        .or_else(|| {
            value
                .pointer("/tool_input/filePath")
                .or_else(|| value.pointer("/args/filePath"))
                .or_else(|| value.pointer("/args/path"))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        });
    let content = payload
        .tool_response
        .as_ref()
        .and_then(|r| r.content.clone())
        .or_else(|| value.get("tool_response").and_then(extract_text_content));
    let pdf_base64 = value.get("tool_response").and_then(pdf_envelope_base64);
    let cwd = extract_cwd(value);

    let (unwrapped_content, line_mapping) = match content {
        Some(ref text) => {
            let (unwrapped, mapping) = unwrap_line_numbered_content(text);
            (Some(unwrapped), Some(mapping))
        }
        None => (None, None),
    };

    Some(normalized_capture(
        value,
        path,
        unwrapped_content,
        line_mapping,
        pdf_base64,
        cwd,
    ))
}

fn normalized_capture(
    value: &serde_json::Value,
    path: Option<String>,
    content: Option<String>,
    source_line_mapping: Option<Vec<u32>>,
    pdf_base64: Option<String>,
    cwd: Option<String>,
) -> NormalizedCapture {
    let event = value
        .get("event")
        .or_else(|| value.get("hook_event_name"))
        .or_else(|| value.get("event_name"))
        .or_else(|| value.get("agent_action_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let phase = if event.contains("pre") || event.contains("before") {
        CapturePhase::Before
    } else if event.contains("post") || event.contains("after") {
        CapturePhase::After
    } else {
        CapturePhase::Unknown
    };
    let response = value.get("tool_response");
    let bool_at = |object: &serde_json::Value, key: &str| {
        object.get(key).and_then(serde_json::Value::as_bool) == Some(true)
    };
    let has_error = |object: &serde_json::Value| {
        object
            .get("error")
            .is_some_and(|error| !error.is_null() && error != "")
    };
    let canceled = bool_at(value, "canceled")
        || bool_at(value, "cancelled")
        || response.is_some_and(|r| bool_at(r, "canceled") || bool_at(r, "cancelled"));
    let failed = value.get("success").and_then(serde_json::Value::as_bool) == Some(false)
        || bool_at(value, "failed")
        || bool_at(value, "is_error")
        || bool_at(value, "isError")
        || has_error(value)
        || response.is_some_and(|r| {
            r.get("success").and_then(serde_json::Value::as_bool) == Some(false)
                || bool_at(r, "failed")
                || bool_at(r, "is_error")
                || bool_at(r, "isError")
                || has_error(r)
        });
    let content_complete = !failed
        && !canceled
        && !bool_at(value, "truncated")
        && !bool_at(value, "partial")
        && !bool_at(value, "filtered")
        && !response.is_some_and(|r| {
            bool_at(r, "truncated")
                || bool_at(r, "is_truncated")
                || bool_at(r, "partial")
                || bool_at(r, "filtered")
        });
    let outcome_evidence = if canceled {
        OutcomeEvidence::Canceled
    } else if failed {
        OutcomeEvidence::Failed
    } else if content.is_some() || pdf_base64.is_some() {
        OutcomeEvidence::CompletedResult
    } else if value.get("success").and_then(serde_json::Value::as_bool) == Some(true)
        || value
            .get("agent_action_name")
            .and_then(serde_json::Value::as_str)
            == Some("post_read_code")
    {
        OutcomeEvidence::ExplicitSuccess
    } else {
        OutcomeEvidence::Unknown
    };
    let line_mapping = source_line_mapping.or_else(|| {
        content
            .as_ref()
            .map(|text| (1..=text.lines().count().max(1) as u32).collect::<Vec<_>>())
    });
    NormalizedCapture {
        read_target: path.clone(),
        path,
        phase,
        outcome_evidence,
        completed_content: content.clone(),
        source_line_mapping: line_mapping,
        content_complete,
        content,
        pdf_base64,
        cwd,
    }
}

/// The documented Windsurf post-read event supplies only `file_path`. It is
/// normalized for local event visibility but remains ineligible for values.
fn windsurf_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    let event = value
        .get("agent_action_name")
        .or_else(|| value.get("event"))
        .or_else(|| value.get("hook_event_name"))
        .and_then(serde_json::Value::as_str)?;
    let cwd = extract_cwd(value);
    if event != "post_read_code" {
        return Some(normalized_capture(value, None, None, None, None, cwd));
    }
    let path = value
        .get("tool_info")
        .and_then(|tool_info| tool_info.get("file_path"))
        .or_else(|| value.get("file_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Some(normalized_capture(value, path, None, None, None, cwd))
}

/// Returns the base64 string of a PDF envelope inside a `tool_response` value
/// when the shape matches the empirically captured `{type: "pdf", file:
/// {filePath, base64, originalSize}}` (spec 033 §1). `None` otherwise.
fn pdf_envelope_base64(tool_response: &serde_json::Value) -> Option<String> {
    let is_pdf = tool_response.get("type").and_then(|t| t.as_str()) == Some("pdf");
    if !is_pdf {
        return None;
    }
    tool_response
        .get("file")
        .and_then(|f| f.get("base64"))
        .or_else(|| tool_response.get("base64"))
        .and_then(|b| b.as_str())
        .map(|s| s.to_string())
}

/// Copilot dual dialect: VS Code Chat (snake_case, default shape) or Copilot
/// CLI (camelCase with JSON-stringified `toolArgs`).
fn copilot_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    if value.get("tool_input").is_some() || value.get("tool_response").is_some() {
        return default_dialect(value);
    }
    let cwd = extract_cwd(value);

    if let Some(args_str) = value.get("toolArgs").and_then(|t| t.as_str()) {
        if let Ok(args) = serde_json::from_str::<serde_json::Value>(args_str) {
            if let Some(path) = args
                .get("path")
                .or_else(|| args.get("file_path"))
                .and_then(|p| p.as_str())
            {
                return Some(normalized_capture(
                    value,
                    Some(path.to_string()),
                    None,
                    None,
                    None,
                    cwd,
                ));
            }
            if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                return Some(normalized_capture(
                    value,
                    extract_path_from_command(cmd),
                    None,
                    None,
                    None,
                    cwd,
                ));
            }
        }
    }

    command_dialect(value)
}

/// Command-based dialect (gemini, vibe, opencode, pi, hermes, and the Bash
/// fallback for claude-code/cursor): the path is the first argument of a
/// read-style command in `tool_input.command` (or a top-level
/// `command`/`args.command`), and the content is `tool_response.stdout`/`output` when
/// non-empty. Non-read commands, empty commands, and malformed JSON yield a
/// no-op (`None` path).
fn command_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    let command = extract_command_str(value);
    let path = command.as_deref().and_then(extract_path_from_command);
    let cwd = extract_cwd(value);
    let content = value
        .pointer("/tool_response/stdout")
        .or_else(|| value.pointer("/tool_response/output"))
        .or_else(|| value.pointer("/tool_response/content"))
        .or_else(|| value.get("stdout"))
        .or_else(|| value.get("output"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let (unwrapped_content, line_mapping) = match content {
        Some(ref text) => {
            let (unwrapped, mapping) = unwrap_line_numbered_content(text);
            (Some(unwrapped), Some(mapping))
        }
        None => (None, None),
    };

    Some(normalized_capture(
        value,
        path,
        unwrapped_content,
        line_mapping,
        None,
        cwd,
    ))
}

fn is_env_assignment(tok: &str) -> bool {
    let mut parts = tok.splitn(2, '=');
    let Some(var) = parts.next() else {
        return false;
    };
    parts.next().is_some()
        && !var.is_empty()
        && var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !var.starts_with(|c: char| c.is_ascii_digit())
}

fn split_shell_segments(cmd: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' && !in_double {
            in_single = !in_single;
            current.push(c);
            i += 1;
        } else if c == '"' && !in_single {
            in_double = !in_double;
            current.push(c);
            i += 1;
        } else if !in_single && !in_double {
            if c == ';' || c == '|' {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    segments.push(std::mem::take(&mut current));
                    i += 2;
                    continue;
                }
                segments.push(std::mem::take(&mut current));
                i += 1;
            } else if c == '&' && i + 1 < chars.len() && chars[i + 1] == '&' {
                segments.push(std::mem::take(&mut current));
                i += 2;
            } else {
                current.push(c);
                i += 1;
            }
        } else {
            current.push(c);
            i += 1;
        }
    }
    if !current.trim().is_empty() {
        segments.push(current);
    }
    segments
}

fn resolve_path_with_cd(tok: &str, cd_dir: Option<&str>) -> Option<String> {
    let cleaned = tok.trim_matches('"').trim_matches('\'').to_string();
    if cleaned.is_empty() {
        return None;
    }
    if let Some(dir) = cd_dir {
        if !std::path::Path::new(&cleaned).is_absolute() {
            return Some(
                std::path::Path::new(dir)
                    .join(cleaned)
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    Some(cleaned)
}

/// Extracts a path argument of a read-style command (`cat`, `read`, `less`,
/// `head`, `tail`, `more`, `grep`, `bat`, `sed`, `python`, `python3`), skipping leading flags
/// and their numeric values. For `grep` the file is the LAST non-flag token (the
/// first is the search pattern). Non-read commands return `None`. When the
/// command is wrapped by `rtk` (RTK's auto-rewrite of `cat`/`read`), the
/// subcommand must be `read` (RTK's read subcommand) for a capture (FR-002).
pub fn extract_path_from_command(command: &str) -> Option<String> {
    const READ_STYLE: &[&str] = &[
        "cat", "read", "less", "head", "tail", "more", "grep", "bat", "sed", "python", "python3",
    ];

    let segments = split_shell_segments(command);
    let mut cd_dir: Option<String> = None;

    for segment in segments {
        let tokens = crate::hooks::editor::shell_tokens(&segment)
            .unwrap_or_else(|| segment.split_whitespace().map(String::from).collect());
        if tokens.is_empty() {
            continue;
        }

        if tokens[0] == "cd" && tokens.len() > 1 {
            cd_dir = Some(tokens[1].trim_matches('"').trim_matches('\'').to_string());
            continue;
        }

        let mut idx = 0;
        while idx < tokens.len() && is_env_assignment(&tokens[idx]) {
            idx += 1;
        }
        if idx >= tokens.len() {
            continue;
        }

        let first = &tokens[idx];
        let verb = std::path::Path::new(first)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(first.as_str());

        let (verb, advance) = if verb == "rtk" {
            if tokens.get(idx + 1).map(|s| s.as_str()) != Some("read") {
                continue;
            }
            ("read", 2)
        } else {
            (verb, 1)
        };

        if !READ_STYLE.contains(&verb) {
            continue;
        }

        let remaining = &tokens[idx + advance..];

        if verb == "python" || verb == "python3" {
            if remaining.iter().any(|t| t == "-c" || t == "-m") {
                return None;
            }
            for tok in remaining {
                if !tok.starts_with('-') {
                    return resolve_path_with_cd(tok, cd_dir.as_deref());
                }
            }
            continue;
        }

        if verb == "sed" {
            let mut target = None;
            let mut skip_next = false;
            for tok in remaining {
                if skip_next {
                    skip_next = false;
                    continue;
                }
                if tok == "-e" || tok == "-f" {
                    skip_next = true;
                    continue;
                }
                if tok.starts_with('-') {
                    continue;
                }
                target = Some(tok);
            }
            if let Some(tok) = target {
                return resolve_path_with_cd(tok, cd_dir.as_deref());
            }
            continue;
        }

        if verb == "grep" {
            let mut last_non_flag = None;
            for tok in remaining {
                if !tok.starts_with('-') {
                    last_non_flag = Some(tok);
                }
            }
            if let Some(tok) = last_non_flag {
                return resolve_path_with_cd(tok, cd_dir.as_deref());
            }
            continue;
        }

        let mut iter = remaining.iter().peekable();
        while let Some(tok) = iter.next() {
            if let Some(rest) = tok.strip_prefix('-') {
                if (matches!(rest, "n" | "c" | "m" | "l" | "C" | "B" | "A")
                    || tok == "--lines"
                    || tok == "--bytes")
                    && iter.peek().and_then(|v| v.parse::<u32>().ok()).is_some()
                {
                    iter.next();
                }
                continue;
            }
            return resolve_path_with_cd(tok, cd_dir.as_deref());
        }
    }
    None
}
