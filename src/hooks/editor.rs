//! Format-preserving configuration editors used by the installer.
//!
//! Every edit here guarantees that bytes RGT does not manage are preserved
//! verbatim (spec FR-001/SC-001): JSON edits are text splices computed from the
//! `jsonc-parser` AST's `Ranged` spans, TOML edits go through `toml_edit`'s
//! document model (comments/formatting/order preserved), and text/rules blocks
//! are replaced only between paired markers. Unparsable or unmergeable input
//! returns [`ConfigEditError`] — never a silent "treat as empty" fallback
//! (spec FR-003).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
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

/// Reports whether a JSONC artifact currently defines at least one nonempty
/// array under its top-level `hooks` object. Used to preserve Windsurf's
/// documented active-file precedence when choosing the workspace artifact.
pub fn jsonc_has_active_hooks(text: &str) -> Result<bool, ConfigEditError> {
    let source = if text.trim().is_empty() { "{}" } else { text };
    let parsed = parse_to_ast(source, &CollectOptions::default(), &ParseOptions::default())
        .map_err(|e| ConfigEditError::msg(format!("unable to parse JSON/JSONC: {e}")))?;
    let Some(value) = parsed.value else {
        return Err(ConfigEditError::msg("config file contains no JSON value"));
    };
    let Value::Object(root) = value else {
        return Err(ConfigEditError::msg(
            "config file root must be a JSON object",
        ));
    };
    let Some(hooks_prop) = root.get("hooks") else {
        return Ok(false);
    };
    let Value::Object(hooks) = &hooks_prop.value else {
        return Err(ConfigEditError::msg("`hooks` must be an object"));
    };
    Ok(hooks.properties.iter().any(
        |property| matches!(&property.value, Value::Array(array) if !array.elements.is_empty()),
    ))
}

/// Extracts native command callbacks only from the host's hook subtree. This
/// gives `rgt doctor` the parsed registration surface instead of broad text
/// substring matches that could be satisfied by comments or unrelated fields.
pub fn jsonc_hook_commands(text: &str) -> Result<Vec<String>, ConfigEditError> {
    Ok(jsonc_hook_callbacks(text)?
        .into_iter()
        .map(|callback| callback.command)
        .collect())
}

/// One command callback and the host structure that owns it. The fields let
/// `rgt doctor` validate the host event and callback-group schema separately
/// from the RGT CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonHookCallback {
    pub hook_path: String,
    pub event: String,
    pub group_index: usize,
    pub callback_index: usize,
    pub callback_count: usize,
    pub nested_callback_group: bool,
    pub callback_type: Option<String>,
    pub command: String,
}

/// Extracts JSON/JSONC callback commands together with their exact top-level
/// hook path, event key, and callback-group location.
pub fn jsonc_hook_callbacks(text: &str) -> Result<Vec<JsonHookCallback>, ConfigEditError> {
    let source = if text.trim().is_empty() { "{}" } else { text };
    let parsed = parse_to_ast(source, &CollectOptions::default(), &ParseOptions::default())
        .map_err(|e| ConfigEditError::msg(format!("unable to parse JSON/JSONC: {e}")))?;
    let Some(Value::Object(root)) = parsed.value else {
        return Err(ConfigEditError::msg(
            "config file root must be a JSON object",
        ));
    };
    let mut callbacks = Vec::new();
    for property in &root.properties {
        if property.name.as_str() == "hooks" || property.name.as_str().ends_with(".hooks") {
            let Value::Object(events) = &property.value else {
                continue;
            };
            for event in &events.properties {
                let Value::Array(groups) = &event.value else {
                    continue;
                };
                for (group_index, group) in groups.elements.iter().enumerate() {
                    let Value::Object(group) = group else {
                        continue;
                    };
                    let nested = group.get_array("hooks");
                    let direct = group.get_string("command");
                    let callback_count = nested.map_or(usize::from(direct.is_some()), |array| {
                        array.elements.len() + usize::from(direct.is_some())
                    });
                    if let Some(command) = direct {
                        callbacks.push(JsonHookCallback {
                            hook_path: property.name.as_str().to_string(),
                            event: event.name.as_str().to_string(),
                            group_index,
                            callback_index: 0,
                            callback_count,
                            nested_callback_group: false,
                            callback_type: None,
                            command: command.value.to_string(),
                        });
                    }
                    if let Some(nested) = nested {
                        for (callback_index, callback) in nested.elements.iter().enumerate() {
                            let Value::Object(callback) = callback else {
                                continue;
                            };
                            let Some(command) = callback.get_string("command") else {
                                continue;
                            };
                            callbacks.push(JsonHookCallback {
                                hook_path: property.name.as_str().to_string(),
                                event: event.name.as_str().to_string(),
                                group_index,
                                callback_index,
                                callback_count,
                                nested_callback_group: true,
                                callback_type: callback
                                    .get_string("type")
                                    .map(|kind| kind.value.to_string()),
                                command: command.value.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(callbacks)
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

    let mut rgt_idx = Vec::new();
    for (i, element) in existing.elements.iter().enumerate() {
        if element_has_rgt_command(element)? {
            rgt_idx.push(i);
        }
    }

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

/// Whether a JSON array element is an RGT-managed hook entry. A group is only
/// owned by RGT when every callback in it is an RGT command; mixed callback
/// groups are ambiguous because replacing the group could delete user hooks.
fn element_has_rgt_command(el: &Value<'_>) -> Result<bool, ConfigEditError> {
    let Value::Object(obj) = el else {
        return Ok(false);
    };
    let mut commands = Vec::new();
    let mut callback_count = 0;
    if let Some(cmd) = obj.get_string("command") {
        callback_count += 1;
        commands.push(cmd.value.to_string());
    }
    if let Some(hooks) = obj.get_array("hooks") {
        for h in &hooks.elements {
            callback_count += 1;
            if let Value::Object(hobj) = h {
                if let Some(cmd) = hobj.get_string("command") {
                    commands.push(cmd.value.to_string());
                }
            }
        }
    }
    let rgt_count = commands.iter().filter(|cmd| is_rgt_command(cmd)).count();
    if rgt_count == 0 {
        return Ok(false);
    }
    if rgt_count != commands.len() || callback_count != commands.len() {
        return Err(ConfigEditError::msg(
            "hook group has mixed RGT and non-RGT callbacks; refusing an unsafe rewrite",
        ));
    }
    Ok(true)
}

/// True when a command is explicitly marked as RGT-managed or invokes the RGT
/// executable by its known basename. Absolute paths are accepted only when
/// their basename is `rgt`/`rgt.exe`; arbitrary absolute executables are user
/// owned. The hidden marker handles test harnesses and renamed installations.
pub fn is_rgt_command(cmd: &str) -> bool {
    let Some((executable, args)) = command_executable_and_args(cmd) else {
        return false;
    };
    let tokens = args;
    let Some(event_index) = tokens
        .windows(2)
        .position(|pair| pair[0] == "hook" && matches!(pair[1].as_str(), "pre" | "post"))
    else {
        return false;
    };
    let _ = event_index;
    let managed = tokens.iter().any(|token| token == "--rgt-managed");
    let base = executable.rsplit(['/', '\\']).next().unwrap_or(&executable);
    managed || base.eq_ignore_ascii_case("rgt") || base.eq_ignore_ascii_case("rgt.exe")
}

pub fn is_rgt_hook_command_for_agent(cmd: &str, agent: &str) -> bool {
    if !is_rgt_command(cmd) {
        return false;
    }
    let Some((_, tokens)) = command_executable_and_args(cmd) else {
        return false;
    };
    tokens
        .windows(2)
        .any(|pair| pair[0] == "--agent" && pair[1] == agent)
        || tokens
            .iter()
            .any(|token| token == &format!("--agent={agent}"))
}

/// Returns the canonical agent argument and event for a marked/recognized
/// direct hook registration, including the encoded PowerShell wrapper emitted
/// by the installer on Windows.
pub fn rgt_hook_identity(cmd: &str) -> Option<(String, String)> {
    let (_, tokens) = command_executable_and_args(cmd)?;
    let event_index = tokens
        .windows(2)
        .position(|pair| pair[0] == "hook" && matches!(pair[1].as_str(), "pre" | "post"))?;
    let event = tokens.get(event_index + 1)?.clone();
    let agent_index = tokens.iter().position(|token| token == "--agent")?;
    let agent = tokens.get(agent_index + 1)?.clone();
    Some((agent, event))
}

/// Checks whether the actual RGT target can be found locally. For encoded
/// Windows registrations this checks the executable inside the PowerShell
/// wrapper, never merely `powershell.exe`.
pub fn hook_command_executable_resolvable(cmd: &str) -> bool {
    let Some((executable, _)) = command_executable_and_args(cmd) else {
        return false;
    };
    if Path::new(&executable).is_absolute() {
        return Path::new(&executable).is_file();
    }
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| {
            let candidate = directory.join(&executable);
            candidate.is_file()
                || (cfg!(windows) && directory.join(format!("{executable}.exe")).is_file())
        })
    })
}

fn command_executable_and_args(command: &str) -> Option<(String, Vec<String>)> {
    let tokens = if command.starts_with("( _rgt_tmp=$(mktemp -d ") {
        let (_, remainder) = command.split_once("; trap 'rm -rf \"$_rgt_tmp\"' 0; ")?;
        let (invocation, _) = remainder.split_once(" >\"$_rgt_tmp/stdout\"")?;
        shell_tokens(invocation)?
    } else {
        shell_tokens(command)?
    };
    let executable = tokens.first()?.clone();
    if executable.eq_ignore_ascii_case("powershell.exe")
        || executable.eq_ignore_ascii_case("powershell")
        || executable.eq_ignore_ascii_case("pwsh.exe")
        || executable.eq_ignore_ascii_case("pwsh")
    {
        let encoded_index = tokens.iter().position(|token| {
            token.eq_ignore_ascii_case("-EncodedCommand")
                || token.eq_ignore_ascii_case("-e")
                || token.eq_ignore_ascii_case("-enc")
        })?;
        let script = decode_powershell(&tokens[encoded_index + 1])?;
        let executable = powershell_assignment(&script, "$p.StartInfo.FileName=")?;
        let arguments = powershell_assignment(&script, "$p.StartInfo.Arguments=")?;
        Some((executable, shell_tokens(&arguments)?))
    } else {
        Some((executable, tokens.into_iter().skip(1).collect()))
    }
}

fn shell_tokens(command: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut started = false;
    for character in command.chars() {
        if escaped {
            token.push(character);
            escaped = false;
            started = true;
            continue;
        }
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('"'), '\\') => escaped = true,
            (Some(_), _) => token.push(character),
            (None, '\\') => escaped = true,
            (None, '\'' | '"') => {
                quote = Some(character);
                started = true;
            }
            (None, value) if value.is_whitespace() => {
                if started {
                    tokens.push(std::mem::take(&mut token));
                    started = false;
                }
            }
            (None, value) => {
                token.push(value);
                started = true;
            }
        }
    }
    if escaped || quote.is_some() {
        return None;
    }
    if started {
        tokens.push(token);
    }
    Some(tokens)
}

fn decode_powershell(encoded: &str) -> Option<String> {
    let bytes = BASE64.decode(encoded).ok()?;
    if bytes.len() % 2 != 0 {
        return None;
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok()
}

fn powershell_assignment(script: &str, prefix: &str) -> Option<String> {
    let value = script.get(script.find(prefix)? + prefix.len()..)?;
    let mut characters = value.chars();
    if characters.next()? != '\'' {
        return None;
    }
    let mut output = String::new();
    loop {
        match characters.next()? {
            '\'' => {
                if characters.clone().next() == Some('\'') {
                    characters.next();
                    output.push('\'');
                } else {
                    return Some(output);
                }
            }
            character => output.push(character),
        }
    }
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

    let ensure_value = |arr: &mut toml_edit::Array| -> Result<bool, ConfigEditError> {
        if arr.iter().any(|entry| entry.as_str().is_none()) {
            return Err(ConfigEditError::msg(format!(
                "TOML key `{last}` contains non-string entries; ownership is ambiguous, repair the array before reinitializing RGT"
            )));
        }
        if arr.iter().any(|entry| entry.as_str() == Some(value)) {
            Ok(false)
        } else {
            arr.push(value);
            Ok(true)
        }
    };

    let inline_parent = rest.len() == 1
        && doc
            .get(rest[0])
            .is_some_and(|item| item.as_inline_table().is_some());
    let changed = if inline_parent {
        let inline = doc
            .as_table_mut()
            .get_mut(rest[0])
            .and_then(toml_edit::Item::as_inline_table_mut)
            .ok_or_else(|| ConfigEditError::msg("TOML inline table could not be edited"))?;
        if !inline.contains_key(last) {
            inline.insert(*last, toml_edit::Value::Array(toml_edit::Array::new()));
        }
        let arr = inline
            .get_mut(last)
            .and_then(toml_edit::Value::as_array_mut)
            .ok_or_else(|| {
                ConfigEditError::msg(format!(
                    "TOML key `{last}` is not an array; ownership is ambiguous, repair the value before reinitializing RGT"
                ))
            })?;
        ensure_value(arr)?
    } else {
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
            .ok_or_else(|| {
                ConfigEditError::msg(format!(
                    "TOML key `{last}` is not an array; ownership is ambiguous, repair the value before reinitializing RGT"
                ))
            })?;
        ensure_value(arr)?
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
    agent: &str,
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
            Some(existing) if is_rgt_hook_command_for_agent(existing, agent) => {
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
/// `rgt hook pre --agent vibe`, creating it with `match = "bash"`,
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
            let owned: Vec<usize> = aot
                .iter()
                .enumerate()
                .filter(|(_, table)| {
                    table
                        .get("command")
                        .and_then(|v| v.as_str())
                        .is_some_and(|command| is_rgt_hook_command_for_agent(command, "vibe"))
                })
                .map(|(index, _)| index)
                .collect();
            if let Some(&first) = owned.first() {
                let mut changed = false;
                if let Some(table) = aot.get_mut(first) {
                    if table.get("command").and_then(|value| value.as_str()) != Some(command) {
                        table["command"] = toml_edit::value(command);
                        changed = true;
                    }
                }
                for index in owned.into_iter().skip(1).rev() {
                    aot.remove(index);
                    changed = true;
                }
                changed
            } else {
                let mut t = toml_edit::Table::new();
                t["match"] = toml_edit::value("bash");
                t["strict"] = toml_edit::value(false);
                t["command"] = toml_edit::value(command);
                aot.push(t);
                true
            }
        } else if let Some(arr) = item.as_array_mut() {
            let owned: Vec<usize> = arr
                .iter()
                .enumerate()
                .filter(|(_, v)| {
                    v.as_inline_table()
                        .and_then(|t| t.get("command"))
                        .and_then(|v| v.as_str())
                        .is_some_and(|command| is_rgt_hook_command_for_agent(command, "vibe"))
                })
                .map(|(index, _)| index)
                .collect();
            if let Some(&first) = owned.first() {
                let mut changed = false;
                if let Some(table) = arr.get_mut(first).and_then(|v| v.as_inline_table_mut()) {
                    if table.get("command").and_then(|value| value.as_str()) != Some(command) {
                        table["command"] = toml_edit::Value::from(command);
                        changed = true;
                    }
                }
                for index in owned.into_iter().skip(1).rev() {
                    arr.remove(index);
                    changed = true;
                }
                changed
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
    // `with_instruction` blocks begin with one newline for concatenation, but
    // that newline sits before the start marker and is outside this slice.
    // Ignore that boundary whitespace so a repeated init stays a true no-op.
    if existing.trim() == new_block.trim() {
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
