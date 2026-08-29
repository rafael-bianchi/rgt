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
    /// 0-based index of this value within its line's emitted values (dates
    /// first, then non-masked numbers). Disambiguates identical values on the
    /// same line (FR-002).
    pub occurrence: u32,
}

/// Normalized capture model consumed by `handle_passive_hook_event`.
#[derive(Debug, Clone, Default)]
pub struct NormalizedCapture {
    pub path: Option<String>,
    pub content: Option<String>,
}

/// Parses a hook event JSON payload from stdin into a structured `HookPayload`.
pub fn parse_hook_payload(json_str: &str) -> Result<HookPayload, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Extracts numeric values and ISO-8601 dates from text content, line by line.
///
/// Returns the extracted values with their source line numbers (1-based) and a
/// per-line occurrence index (FR-002). Recognized ISO-8601 dates emit exactly
/// one `Date` each, and their digit components are masked from the number scan
/// so no spurious `Number` nodes leak from a date's year/month/day (FR-001).
pub fn extract_values_from_content(content: &str) -> Vec<ExtractedValue> {
    let mut results = Vec::new();
    let iso_date_regex =
        Regex::new(r"\b(\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)?)\b").unwrap();
    let number_regex = Regex::new(r"\b(\d+(?:\.\d+)?)\b").unwrap();

    for (idx, line) in content.lines().enumerate() {
        let line_num = (idx + 1) as u32;
        let mut occurrence: u32 = 0;

        // FR-001: recognize dates first and record their byte spans, so the
        // number scan can skip (mask) everything inside a recognized date.
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
                    results.push(ExtractedValue {
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
                if let Ok(num) = m.as_str().parse::<f64>() {
                    results.push(ExtractedValue {
                        value: ValueData::Number(num),
                        line_number: Some(line_num),
                        occurrence,
                    });
                    occurrence += 1;
                }
            }
        }
    }

    results
}

/// Normalizes an agent's native stdin event JSON into the internal
/// `{path, content}` capture model. Returns `None` when the event is malformed
/// or does not reference a capturable file, so callers can fail open.
///
/// `agent` selects the stdin dialect; `None`/`claude-code`/`cursor` use the
/// default Claude Code compatible shape (existing behavior).
pub fn normalize_agent_event(agent: Option<&str>, stdin: &str) -> Option<NormalizedCapture> {
    let value: serde_json::Value = serde_json::from_str(stdin).ok()?;
    match agent {
        // FR-001: try the Read-shaped dialect first; a Bash tool call has no
        // `path`/`content`, so fall back to command-based extraction.
        None | Some("claude-code") | Some("cursor") => match default_dialect(&value) {
            Some(c) if c.path.is_some() => Some(c),
            _ => command_dialect(&value),
        },
        Some("copilot") => copilot_dialect(&value),
        Some("gemini") | Some("vibe") | Some("opencode") | Some("pi") | Some("hermes") => {
            command_dialect(&value)
        }
        Some(_) => None,
    }
}

/// Default dialect: `tool_input.path`/`file_path` + `tool_response.content`
/// (reuses the structured Claude Code compatible payload shape).
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
    Some(NormalizedCapture { path, content })
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
                return Some(NormalizedCapture {
                    path: Some(path.to_string()),
                    content: None,
                });
            }
            if let Some(cmd) = args.get("command").and_then(|c| c.as_str()) {
                return Some(NormalizedCapture {
                    path: extract_path_from_command(cmd),
                    content: None,
                });
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
    Some(NormalizedCapture { path, content })
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
