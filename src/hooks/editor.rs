//! Format-preserving configuration editors used by the installer.
//!
//! Every edit here guarantees that bytes RGT does not manage are preserved
//! verbatim (spec FR-001/SC-001): JSON edits are text splices computed from the
//! `jsonc-parser` AST's `Ranged` spans, TOML edits go through `toml_edit`'s
//! document model (comments/formatting/order preserved), and text/rules blocks
//! are replaced only between paired markers. Unparsable or unmergeable input
//! returns [`ConfigEditError`] — never a silent "treat as empty" fallback
//! (spec FR-003).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jsonc_parser::ast::{Array, Object, Value};
use jsonc_parser::common::Ranged;
use jsonc_parser::{parse_to_ast, CollectOptions, ParseOptions};
use serde_json::Map as JsonMap;

/// An error produced while reading, parsing, or editing a configuration file.
#[derive(Debug, Clone)]
pub struct ConfigEditError {
    /// The config file path, when known.
    pub path: Option<String>,
    /// Human-readable reason, user-recoverable (what / which file / how to fix).
    pub reason: String,
    /// Path of a backup written before an attempted edit, when one exists.
    pub backup_path: Option<String>,
}

impl ConfigEditError {
    pub fn new(path: Option<String>, reason: impl Into<String>) -> Self {
        Self {
            path,
            reason: reason.into(),
            backup_path: None,
        }
    }

    /// Error without a file-path context (pure text-level failures).
    pub fn msg(reason: impl Into<String>) -> Self {
        Self::new(None, reason)
    }
}

impl std::fmt::Display for ConfigEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{}: {}", p, self.reason),
            None => write!(f, "{}", self.reason),
        }
    }
}

impl std::error::Error for ConfigEditError {}

/// Reads a config file; a missing file yields the empty string (caller decides
/// whether that means "fresh"). Read/IO failures are loud errors (FR-003).
pub fn read_config(path: &Path) -> Result<String, ConfigEditError> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| {
        ConfigEditError::new(
            Some(path.display().to_string()),
            format!("failed to read config file: {e}"),
        )
    })
}

/// Writes `content` to `path`, creating parent directories as needed.
pub fn write_config(path: &Path, content: &str) -> Result<(), ConfigEditError> {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, content).map_err(|e| {
        ConfigEditError::new(
            Some(path.display().to_string()),
            format!("failed to write config file: {e}"),
        )
    })
}

/// Writes `content` to `path`, first creating a `<path>.rgt.bak` backup when the
/// file already exists (FR-004). Failure messages include the backup path.
pub fn write_config_with_backup(path: &Path, content: &str) -> Result<(), ConfigEditError> {
    let backup = backup_before_write(path).map_err(|e| {
        ConfigEditError::new(
            Some(path.display().to_string()),
            format!("failed to back up config file before writing: {e}"),
        )
    })?;
    if let Err(mut e) = write_config(path, content) {
        e.backup_path = backup.as_ref().map(|b| b.display().to_string());
        return Err(e);
    }
    Ok(())
}

/// Copies an existing file to `<path>.rgt.bak` before any write that modifies
/// it. Returns the backup path, or `None` when the file does not exist yet.
/// Backups are never auto-deleted (spec assumption; SC-006).
pub fn backup_before_write(path: &Path) -> io::Result<Option<PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let backup = PathBuf::from(format!("{}.rgt.bak", path.display()));
    fs::copy(path, &backup)?;
    Ok(Some(backup))
}

// ---------------------------------------------------------------------------
// JSONC: merge RGT hook entries into a JSON/JSONC document, byte-for-byte
// ---------------------------------------------------------------------------

/// Ensures the nested structure described by `desired` exists inside the JSONC
/// `text`, merging RGT entries into existing hook arrays and preserving all
/// untouched bytes (comments, formatting, foreign hook groups) verbatim.
///
/// `desired` is a JSON object describing the full target subtree, e.g.
/// `{"hooks": {"PostToolUse": [<rgt-group>], "PreToolUse": [<rgt-group>]}}`.
///
/// Returns the edited text and whether anything changed.
pub fn jsonc_merge_hook_entries(
    text: &str,
    desired: &serde_json::Value,
) -> Result<(String, bool), ConfigEditError> {
    let Some(desired_obj) = desired.as_object() else {
        return Err(ConfigEditError::msg(
            "desired RGT structure must be a JSON object",
        ));
    };
    if desired_obj.is_empty() {
        return Ok((text.to_string(), false));
    }

    // Empty/whitespace-only files are "effectively absent" (spec assumption):
    // treat them as an empty object so the desired structure is created.
    let source = if text.trim().is_empty() { "{}" } else { text };

    let parsed = parse_to_ast(source, &CollectOptions::default(), &ParseOptions::default())
        .map_err(|e| {
            ConfigEditError::msg(format!(
                "unable to parse JSON/JSONC (comments and trailing commas are supported): {e}"
            ))
        })?;
    let Some(value) = parsed.value else {
        return Err(ConfigEditError::msg("config file contains no JSON value"));
    };
    let Value::Object(root) = value else {
        return Err(ConfigEditError::msg(
            "config file root must be a JSON object",
        ));
    };

    let mut edits = Vec::new();
    merge_into(&root, desired_obj, source, &mut edits)?;

    if edits.is_empty() {
        return Ok((text.to_string(), false));
    }
    let edited = apply_edits(source, edits);
    Ok((edited.clone(), edited != source))
}

/// One text splice: replace `text[start..end]` with `replacement`.
#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    end: usize,
    replacement: String,
}

fn apply_edits(text: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
    let mut out = text.to_string();
    for e in edits {
        debug_assert!(e.start <= e.end && e.end <= out.len());
        out.replace_range(e.start..e.end, &e.replacement);
    }
    out
}

/// Recursively merges the `desired` object into an existing JSON object,
/// collecting text edits (applied later, largest offset first).
fn merge_into(
    existing: &Object<'_>,
    desired: &JsonMap<String, serde_json::Value>,
    text: &str,
    edits: &mut Vec<Edit>,
) -> Result<(), ConfigEditError> {
    let mut missing = Vec::new();

    for (key, dv) in desired {
        match existing.get(key) {
            None => missing.push((key.clone(), dv)),
            Some(prop) => match (&prop.value, dv) {
                (Value::Object(child), serde_json::Value::Object(dc)) => {
                    merge_into(child, dc, text, edits)?;
                }
                (Value::Array(arr), serde_json::Value::Array(_)) => {
                    merge_array(arr, dv, text, edits)?;
                }
                (Value::Object(_), _) | (Value::Array(_), _) => {
                    return Err(ConfigEditError::msg(format!(
                        "key `{key}` exists as a container but the desired RGT structure needs a different type"
                    )));
                }
                (_, serde_json::Value::Object(_)) | (_, serde_json::Value::Array(_)) => {
                    return Err(ConfigEditError::msg(format!(
                        "key `{key}` exists as a scalar value; cannot merge a structure into it"
                    )));
                }
                // Existing scalar + desired scalar: leave the user's value as-is.
                (_, _) => {}
            },
        }
    }

    if !missing.is_empty() {
        // Reuse an existing trailing comma before `}`; otherwise separate with ", ".
        let sep = if existing.properties.is_empty() {
            ""
        } else {
            let last_prop_end = existing.properties.last().unwrap().end();
            let tail = &text[last_prop_end..existing.end() - 1];
            if tail.trim_start().starts_with(',') {
                ""
            } else {
                ", "
            }
        };
        let body = missing
            .iter()
            .map(|(k, v)| {
                format!(
                    "\"{k}\": {}",
                    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        edits.push(Edit {
            start: existing.end() - 1,
            end: existing.end() - 1,
            replacement: format!("{sep}{body}"),
        });
    }
    Ok(())
}

/// Merges RGT entries into an existing hook-group array: adds them when absent,
/// or replaces the contiguous run of existing `rgt hook` entries in place.
fn merge_array(
    existing: &Array<'_>,
    desired_arr: &serde_json::Value,
    text: &str,
    edits: &mut Vec<Edit>,
) -> Result<(), ConfigEditError> {
    let entries: Vec<serde_json::Value> = desired_arr.as_array().cloned().unwrap_or_default();
    if entries.is_empty() {
        return Ok(());
    }
    let desired_serialized = entries
        .iter()
        .map(|e| serde_json::to_string(e).unwrap_or_else(|_| "null".into()))
        .collect::<Vec<_>>()
        .join(", ");

    let rgt_idx: Vec<usize> = existing
        .elements
        .iter()
        .enumerate()
        .filter(|(_, el)| element_has_rgt_command(el))
        .map(|(i, _)| i)
        .collect();

    if rgt_idx.is_empty() {
        if existing.elements.is_empty() {
            edits.push(Edit {
                start: existing.end() - 1,
                end: existing.end() - 1,
                replacement: desired_serialized,
            });
        } else {
            let last = existing.elements.last().unwrap();
            let tail = &text[last.end()..existing.end() - 1];
            let replacement = if tail.trim_start().starts_with(',') {
                desired_serialized.clone()
            } else {
                format!(", {desired_serialized}")
            };
            edits.push(Edit {
                start: existing.end() - 1,
                end: existing.end() - 1,
                replacement,
            });
        }
        return Ok(());
    }

    for w in rgt_idx.windows(2) {
        if w[1] != w[0] + 1 {
            return Err(ConfigEditError::msg(
                "RGT hook entries are interleaved with other entries; refusing an unsafe rewrite",
            ));
        }
    }

    // Idempotency: if the existing RGT entries already carry the same commands,
    // leave the text untouched regardless of formatting/whitespace differences.
    let mut existing_cmds: Vec<String> = rgt_idx
        .iter()
        .flat_map(|&i| element_commands(&existing.elements[i]))
        .collect();
    let mut desired_cmds: Vec<String> = entries.iter().flat_map(entry_commands).collect();
    existing_cmds.sort();
    desired_cmds.sort();
    if existing_cmds == desired_cmds {
        return Ok(());
    }

    let first = &existing.elements[rgt_idx[0]];
    let last = &existing.elements[*rgt_idx.last().unwrap()];
    edits.push(Edit {
        start: first.start(),
        end: last.end(),
        replacement: desired_serialized,
    });
    Ok(())
}

/// Commands found in an existing AST hook-group element (top-level `command` or
/// `hooks[].command`).
fn element_commands(el: &Value<'_>) -> Vec<String> {
    let mut cmds = Vec::new();
    let Value::Object(obj) = el else {
        return cmds;
    };
    if let Some(cmd) = obj.get_string("command") {
        cmds.push(cmd.value.to_string());
    }
    if let Some(hooks) = obj.get_array("hooks") {
        for h in &hooks.elements {
            if let Value::Object(hobj) = h {
                if let Some(cmd) = hobj.get_string("command") {
                    cmds.push(cmd.value.to_string());
                }
            }
        }
    }
    cmds
}

/// Commands found in a desired (serde) hook-group entry.
fn entry_commands(e: &serde_json::Value) -> Vec<String> {
    let mut cmds = Vec::new();
    if let Some(cmd) = e.get("command").and_then(|c| c.as_str()) {
        cmds.push(cmd.to_string());
    }
    if let Some(hooks) = e.get("hooks").and_then(|h| h.as_array()) {
        for h in hooks {
            if let Some(cmd) = h.get("command").and_then(|c| c.as_str()) {
                cmds.push(cmd.to_string());
            }
        }
    }
    cmds
}

/// Whether a JSON array element is an RGT-managed hook entry (its command, or a
/// command nested under `hooks`, starts with `rgt hook`).
fn element_has_rgt_command(el: &Value<'_>) -> bool {
    let Value::Object(obj) = el else {
        return false;
    };
    if let Some(cmd) = obj.get_string("command") {
        if cmd.value.starts_with("rgt hook") {
            return true;
        }
    }
    if let Some(hooks) = obj.get_array("hooks") {
        for h in &hooks.elements {
            if let Value::Object(hobj) = h {
                if let Some(cmd) = hobj.get_string("command") {
                    if cmd.value.starts_with("rgt hook") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// TOML: format-preserving edits via toml_edit
// ---------------------------------------------------------------------------

fn parse_toml(text: &str) -> Result<toml_edit::DocumentMut, ConfigEditError> {
    text.parse()
        .map_err(|e| ConfigEditError::msg(format!("unable to parse TOML config: {e}")))
}

/// Ensures `value` is present in the TOML array at `path` (creating the path and
/// array as needed). Returns the edited text and whether anything changed.
pub fn toml_ensure_array_value(
    text: &str,
    path: &[&str],
    value: &str,
) -> Result<(String, bool), ConfigEditError> {
    let mut doc = parse_toml(text)?;
    let (last, rest) = path
        .split_last()
        .ok_or_else(|| ConfigEditError::msg("empty TOML path"))?;

    let changed = {
        let mut table = doc.as_table_mut();
        for seg in rest {
            let item = table.entry(seg).or_insert(toml_edit::table());
            table = item
                .as_table_mut()
                .ok_or_else(|| ConfigEditError::msg(format!("TOML key `{seg}` is not a table")))?;
        }
        let item = table
            .entry(last)
            .or_insert(toml_edit::Item::Value(toml_edit::Value::Array(
                toml_edit::Array::new(),
            )));
        let arr = item
            .as_array_mut()
            .ok_or_else(|| ConfigEditError::msg(format!("TOML key `{last}` is not an array")))?;
        if arr.iter().any(|v| v.as_str() == Some(value)) {
            false
        } else {
            arr.push(value);
            true
        }
    };

    Ok(if changed {
        (doc.to_string(), true)
    } else {
        (text.to_string(), false)
    })
}

/// Ensures `key = "value"` inside the TOML table at `path` (creating it as
/// needed). A pre-existing conflicting non-RGT value is a hard error (a table
/// cannot be duplicated safely). Returns the edited text and whether anything
/// changed.
pub fn toml_ensure_table_entry(
    text: &str,
    path: &[&str],
    key: &str,
    value: &str,
) -> Result<(String, bool), ConfigEditError> {
    let mut doc = parse_toml(text)?;
    let (last, rest) = path
        .split_last()
        .ok_or_else(|| ConfigEditError::msg("empty TOML path"))?;

    let changed = {
        let mut table = doc.as_table_mut();
        for seg in rest {
            let item = table.entry(seg).or_insert(toml_edit::table());
            table = item
                .as_table_mut()
                .ok_or_else(|| ConfigEditError::msg(format!("TOML key `{seg}` is not a table")))?;
        }
        let item = table.entry(last).or_insert(toml_edit::table());
        let t = item
            .as_table_mut()
            .ok_or_else(|| ConfigEditError::msg(format!("TOML key `{last}` is not a table")))?;
        match t.get(key).and_then(|v| v.as_str()) {
            Some(existing) if existing == value => false,
            Some(existing) if existing.contains("rgt hook") => {
                t[key] = toml_edit::value(value);
                true
            }
            Some(_) => {
                return Err(ConfigEditError::msg(format!(
                    "conflicting user value for `{key}` in `{last}`; refusing to overwrite it"
                )));
            }
            None => {
                t[key] = toml_edit::value(value);
                true
            }
        }
    };

    Ok(if changed {
        (doc.to_string(), true)
    } else {
        (text.to_string(), false)
    })
}

/// Ensures a Mistral Vibe `[[pre_tool]]` entry exists with `command` containing
/// `rgt hook post --agent vibe`, creating it with `match = "bash"`,
/// `strict = false`, and the canonical command when absent. Returns the edited
/// text and whether anything changed.
pub fn toml_ensure_vibe_pre_tool(
    text: &str,
    command: &str,
) -> Result<(String, bool), ConfigEditError> {
    let mut doc = parse_toml(text)?;

    let changed = {
        let table = doc.as_table_mut();
        if !table.contains_key("pre_tool") {
            table.insert(
                "pre_tool",
                toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
            );
        }
        let item = table
            .get_mut("pre_tool")
            .ok_or_else(|| ConfigEditError::msg("TOML key `pre_tool` not found"))?;

        if let Some(aot) = item.as_array_of_tables_mut() {
            if aot.iter().any(|t| {
                t.get("command")
                    .and_then(|v| v.as_str())
                    .map(|c| c.contains("rgt hook post --agent vibe"))
                    .unwrap_or(false)
            }) {
                false
            } else {
                let mut t = toml_edit::Table::new();
                t["match"] = toml_edit::value("bash");
                t["strict"] = toml_edit::value(false);
                t["command"] = toml_edit::value(command);
                aot.push(t);
                true
            }
        } else if let Some(arr) = item.as_array_mut() {
            if arr.iter().any(|v| {
                v.as_inline_table()
                    .and_then(|t| t.get("command"))
                    .and_then(|v| v.as_str())
                    .map(|c| c.contains("rgt hook post --agent vibe"))
                    .unwrap_or(false)
            }) {
                false
            } else {
                let mut t = toml_edit::InlineTable::new();
                t["match"] = toml_edit::Value::from("bash");
                t["strict"] = toml_edit::Value::from(false);
                t["command"] = toml_edit::Value::from(command);
                arr.push(t);
                true
            }
        } else {
            return Err(ConfigEditError::msg(
                "TOML key `pre_tool` is neither an array of tables nor an array",
            ));
        }
    };

    Ok(if changed {
        (doc.to_string(), true)
    } else {
        (text.to_string(), false)
    })
}

// ---------------------------------------------------------------------------
// Text/rules files: paired-marker block replacement
// ---------------------------------------------------------------------------

/// Replaces the RGT block in `text`, delimited by `open_marker` and `end_marker`
/// lines, with `block`. Content above and below the block is preserved
/// byte-for-byte. When `open_marker` is absent the block is appended at the end.
/// When `open_marker` is present but `end_marker` is missing (a legacy block),
/// this is a hard error: RGT must never guess a boundary (spec FR-006).
///
/// Returns the edited text and whether anything changed.
pub fn text_replace_block(
    text: &str,
    open_marker: &str,
    end_marker: &str,
    block: &str,
) -> Result<(String, bool), ConfigEditError> {
    let ranges = line_ranges(text);
    let line_starts_with = |i: usize, marker: &str| -> bool {
        let (s, e) = ranges[i];
        text[s..e].trim_start().starts_with(marker)
    };
    let open_idx = ranges
        .iter()
        .position(|_| true)
        .map(|_| 0)
        .and_then(|_| (0..ranges.len()).find(|&i| line_starts_with(i, open_marker)));

    let Some(oi) = open_idx else {
        // No marker: append the block at the end (FR-005 fresh write path).
        let mut out = text.trim_end().to_string();
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(block);
        if !block.ends_with('\n') {
            out.push('\n');
        }
        return Ok((out, true));
    };

    let ei = (oi..ranges.len()).find(|&i| line_starts_with(i, end_marker));

    let Some(ei) = ei else {
        return Err(ConfigEditError::msg(format!(
            "found the RGT marker `{open_marker}` but no end marker `{end_marker}`; refusing to guess the block boundary. Remove the legacy RGT block manually (or restore it) and re-run."
        )));
    };

    let (start_byte, _) = ranges[oi];
    let (_, end_byte) = ranges[ei];
    let existing = &text[start_byte..end_byte];
    let new_block = block.trim_end();
    if existing.trim_end() == new_block {
        return Ok((text.to_string(), false));
    }
    let edited = format!(
        "{}{}\n{}",
        &text[..start_byte],
        new_block,
        &text[end_byte..]
    );
    Ok((edited, true))
}

/// Byte ranges (start, end) of each line in `text`, where `end` includes the
/// trailing `\n` when present.
fn line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut pos = 0usize;
    for line in text.split_inclusive('\n') {
        ranges.push((pos, pos + line.len()));
        pos += line.len();
    }
    if ranges.is_empty() {
        ranges.push((0, 0));
    }
    ranges
}
