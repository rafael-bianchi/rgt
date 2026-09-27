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
    let iso_date_regex =
        Regex::new(r"\b(\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)?)\b").unwrap();
    // FR-004: resolve `Auto` once per file, then reuse the concrete format.
    let format = match format {
        NumberFormat::Auto => resolve_auto_format(content),
        other => other,
    };
    let (decimal, thousands) = match format {
        NumberFormat::Us => ('.', ','),
        NumberFormat::Eu => (',', '.'),
        NumberFormat::Auto => unreachable!("Auto resolved above"),
    };
    let number_regex = Regex::new(r"\b(\d[\d.,]*\d|\d)\b").unwrap();

    for (idx, line) in content.lines().enumerate() {
        let line_num = (idx + 1) as u32;
        let mut occurrence: u32 = 0;

        // Recognize dates first and record their byte spans, so the number scan
        // can skip (mask) everything inside a recognized date.
        let mut date_spans: Vec<(usize, usize)> = Vec::new();
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
                    date_spans.push((m.start(), m.end()));
                    result.values.push(ExtractedValue {
                        value: ValueData::Date(valid_dt),
                        line_number: Some(line_num),
                        occurrence,
                    });
                    occurrence += 1;
                }
            }
        }

        for cap in number_regex.captures_iter(line) {
            if let Some(m) = cap.get(1) {
                // Skip any number match that lies inside a recognized date.
                if date_spans
                    .iter()
                    .any(|(s, e)| m.start() >= *s && m.end() <= *e)
                {
                    continue;
                }
                // FR-001: attach an immediately-adjacent leading `-`/`+` and
                // accounting `(...)` pair (only when not preceded by a word
                // char, so `abc-123` stays `123`, not `-123`).
                let (prefix, suffix) = candidate_context(line, m.start(), m.end());
                match normalize_number(&prefix, m.as_str(), &suffix, decimal, thousands) {
                    Some(num) => {
                        result.values.push(ExtractedValue {
                            value: ValueData::Number(num),
                            line_number: Some(line_num),
                            occurrence,
                        });
                        occurrence += 1;
                    }
                    None => result.skipped_malformed += 1,
                }
            }
        }
    }

    result
}

/// Expands a numeric core span into its optional sign/parentheses context:
/// returns `(prefix, suffix)` strings. The prefix may be a leading `-`/`+`
/// and/or an opening `(`; the suffix is a closing `)` when an opening `(` was
/// attached. Attachment is suppressed when the context is preceded by a word
/// character, so `abc-123` and `abc(500)` do not take on a sign.
fn candidate_context(line: &str, start: usize, end: usize) -> (String, String) {
    let bytes = line.as_bytes();
    let max_run = start.min(2);
    let mut run_len = 0;
    while run_len < max_run && matches!(bytes[start - run_len - 1], b'-' | b'+' | b'(') {
        run_len += 1;
    }
    let prefix_bytes = &bytes[start - run_len..start];
    let preceded_by_word = start > run_len
        && (bytes[start - run_len - 1].is_ascii_alphanumeric()
            || bytes[start - run_len - 1] == b'_');
    let prefix = if preceded_by_word {
        String::new()
    } else {
        String::from_utf8_lossy(prefix_bytes).into_owned()
    };
    let mut suffix = String::new();
    if end < bytes.len() && bytes[end] == b')' && prefix.contains('(') {
        suffix.push(')');
    }
    (prefix, suffix)
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
    if negative {
        value = -value;
    }
    Some(value)
}

/// Infers a concrete format for `Auto` by scanning the content once (FR-004):
/// the first number candidate containing both `.` and `,` determines the
/// decimal as its rightmost separator; when no candidate contains both, the
/// format falls back to `Us`.
fn resolve_auto_format(content: &str) -> NumberFormat {
    let candidate_regex = Regex::new(r"\d[\d.,]*\d").unwrap();
    for line in content.lines() {
        for cap in candidate_regex.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                let s = m.as_str();
                let has_dot = s.contains('.');
                let has_comma = s.contains(',');
                if has_dot && has_comma {
                    if let Some(idx) = s.rfind(['.', ',']) {
                        return if s.as_bytes()[idx] == b'.' {
                            NumberFormat::Us
                        } else {
                            NumberFormat::Eu
                        };
                    }
                }
            }
        }
    }
    NumberFormat::Us
}

/// Normalizes an agent's native stdin event JSON into the internal
/// `{path, content}` capture model. Returns `None` when the event is malformed
/// or does not reference a capturable file, so callers can fail open.
///
/// `agent` selects the stdin dialect; `None`/`claude-code`/`cursor` use the
/// default Claude Code compatible shape (existing behavior).
pub fn normalize_agent_event(agent: Option<&str>, stdin: &str) -> Option<NormalizedCapture> {
    let stdin = stdin.strip_prefix('\u{feff}').unwrap_or(stdin);
    let value: serde_json::Value = serde_json::from_str(stdin).ok()?;
    let mut capture = match agent {
        // FR-001: try the Read-shaped dialect first; a Bash tool call has no
        // `path`/`content`, so fall back to command-based extraction.
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

    // Until a documented or native event contract is recorded, these parsed
    // compatibility shapes are diagnostic candidates only. They cannot
    // authorize graph writes even if a synthetic payload happens to contain
    // fields resembling a complete result.
    if !matches!(
        agent,
        None | Some("claude-code") | Some("codex") | Some("opencode")
    ) {
        capture.content_complete = false;
    }
    if agent == Some("opencode") && opencode_native_read_dialect(&value).is_none() {
        capture.content_complete = false;
    }
    if matches!(agent, None | Some("claude-code") | Some("codex"))
        && !is_qualifying_read_event(agent, &value, &capture)
    {
        capture.content_complete = false;
    }
    Some(capture)
}

/// OpenCode's observed `read` result exposes the complete decoded file text
/// in `metadata.display.text`. Only a full-file, untruncated after-result is
/// eligible; the caller still verifies those bytes against the disk snapshot.
fn opencode_native_read_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    if !is_opencode_native_read(value) {
        return None;
    }
    let path = value.pointer("/args/filePath")?.as_str()?;
    let result = value.get("result")?;
    let metadata = result.get("metadata")?;
    let display = metadata.get("display")?;
    let text = display.get("text")?.as_str()?;
    let line_count = u32::try_from(text.lines().count()).ok()?;
    if path.is_empty()
        || result.get("output")?.as_str().is_none()
        || [value, result]
            .into_iter()
            .any(opencode_outcome_is_negative)
        || metadata.get("truncated")?.as_bool()?
        || display.get("truncated")?.as_bool()?
        || display.get("type")?.as_str()? != "file"
        || display.get("path")?.as_str()? != path
        || display.get("lineStart")?.as_u64()? != 1
        || display.get("lineEnd")?.as_u64()? != u64::from(line_count)
        || display.get("totalLines")?.as_u64()? != u64::from(line_count)
    {
        return None;
    }
    let content = text.to_string();
    Some(NormalizedCapture {
        path: Some(path.to_string()),
        read_target: Some(path.to_string()),
        phase: CapturePhase::After,
        outcome_evidence: OutcomeEvidence::CompletedResult,
        completed_content: Some(content.clone()),
        source_line_mapping: Some((1..=line_count).collect()),
        content_complete: true,
        content: Some(content),
        pdf_base64: None,
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
        .and_then(serde_json::Value::as_str);
    match (agent, tool) {
        (None | Some("claude-code"), Some("Read")) => capture.read_target.is_some(),
        (None | Some("claude-code"), Some("Bash")) => {
            let command = value
                .get("tool_input")
                .and_then(|input| input.get("command"))
                .and_then(serde_json::Value::as_str)
                .or_else(|| value.get("command").and_then(serde_json::Value::as_str));
            command.is_some_and(|command| extract_path_from_command(command).is_some())
                && capture.read_target.is_some()
        }
        (Some("codex"), Some("read_file" | "ReadFile" | "Read")) => capture.read_target.is_some(),
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
fn default_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    let payload = parse_hook_payload(&value.to_string()).ok()?;
    let path = payload
        .tool_input
        .as_ref()
        .and_then(|t| t.path.clone().or_else(|| t.file_path.clone()));
    let content = payload
        .tool_response
        .as_ref()
        .and_then(|r| r.content.clone());
    // Inspect the raw JSON, not the round-tripped struct: HookToolResponse only
    // carries `content`, so the envelope's `type`/`file` fields survive only on
    // the original value.
    let pdf_base64 = value.get("tool_response").and_then(pdf_envelope_base64);
    Some(normalized_capture(value, path, content, pdf_base64))
}

fn normalized_capture(
    value: &serde_json::Value,
    path: Option<String>,
    content: Option<String>,
    pdf_base64: Option<String>,
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
    let source_line_mapping = content
        .as_ref()
        .map(|text| (1..=text.lines().count().max(1) as u32).collect::<Vec<_>>());
    NormalizedCapture {
        read_target: path.clone(),
        path,
        phase,
        outcome_evidence,
        completed_content: content.clone(),
        source_line_mapping,
        content_complete,
        content,
        pdf_base64,
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
    if event != "post_read_code" {
        return Some(normalized_capture(value, None, None, None));
    }
    let path = value
        .get("tool_info")
        .and_then(|tool_info| tool_info.get("file_path"))
        .or_else(|| value.get("file_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Some(normalized_capture(value, path, None, None))
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
        .and_then(|b| b.as_str())
        .map(|s| s.to_string())
}

/// Copilot dual dialect: VS Code Chat (snake_case, default shape) or Copilot
/// CLI (camelCase with JSON-stringified `toolArgs`).
fn copilot_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    if value.get("tool_input").is_some() || value.get("tool_response").is_some() {
        return default_dialect(value);
    }

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
                ));
            }
            if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                return Some(normalized_capture(
                    value,
                    extract_path_from_command(cmd),
                    None,
                    None,
                ));
            }
        }
    }

    command_dialect(value)
}

/// Command-based dialect (gemini, vibe, opencode, pi, hermes, and the Bash
/// fallback for claude-code/cursor): the path is the first argument of a
/// read-style command in `tool_input.command` (or a top-level
/// `command`/`args.command`), and the content is `tool_response.stdout` when
/// non-empty. Non-read commands, empty commands, and malformed JSON yield a
/// no-op (`None` path).
fn command_dialect(value: &serde_json::Value) -> Option<NormalizedCapture> {
    let command = value
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .or_else(|| value.get("command"))
        .or_else(|| value.get("args").and_then(|a| a.get("command")))
        .and_then(|c| c.as_str());
    let path = command.and_then(extract_path_from_command);
    // FR-003: capture the Bash tool's stdout as content when non-empty;
    // otherwise leave it None so the disk-read fallback applies.
    let content = value
        .get("tool_response")
        .and_then(|t| t.get("stdout"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    Some(normalized_capture(value, path, content, None))
}

/// Extracts a path argument of a read-style command (`cat`, `read`, `less`,
/// `head`, `tail`, `more`, `grep`, `python`, `python3`), skipping leading flags
/// and their numeric values. For `grep` the file is the LAST non-flag token (the
/// first is the search pattern). Non-read commands return `None`. When the
/// command is wrapped by `rtk` (RTK's auto-rewrite of `cat`/`read`), the
/// subcommand must be `read` (RTK's read subcommand) for a capture (FR-002).
pub fn extract_path_from_command(command: &str) -> Option<String> {
    const READ_STYLE: &[&str] = &[
        "cat", "read", "less", "head", "tail", "more", "grep", "python", "python3",
    ];
    let mut tokens = command.split_whitespace();
    let first = tokens.next()?;
    let verb = if first == "rtk" {
        // RTK maps `cat`/`read` invocations to the `rtk read` subcommand; other
        // `rtk` subcommands are not reads.
        if tokens.next() != Some("read") {
            return None;
        }
        "read"
    } else {
        first
    };
    if !READ_STYLE.contains(&verb) {
        return None;
    }

    let mut last_non_flag: Option<String> = None;
    while let Some(tok) = tokens.next() {
        if let Some(rest) = tok.strip_prefix('-') {
            // A numeric-value flag (e.g. `head -n 3`) may consume the next
            // token when it is numeric; combined flags (`-3`, `-n3`) are
            // skipped outright.
            if matches!(rest, "n" | "c" | "m" | "l")
                && tokens
                    .clone()
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_some()
            {
                tokens.next();
            }
            continue;
        }
        let cleaned = tok.trim_matches('"').trim_matches('\'').to_string();
        if verb != "grep" {
            return Some(cleaned);
        }
        last_non_flag = Some(cleaned);
    }
    last_non_flag
}
