use rgt::hooks::editor::{
    jsonc_merge_hook_entries, text_replace_block, toml_ensure_array_value, toml_ensure_table_entry,
    toml_ensure_vibe_pre_tool,
};
use serde_json::json;

fn rgt_group(command: &str) -> serde_json::Value {
    json!({
        "matcher": "",
        "hooks": [{ "type": "command", "command": command }]
    })
}

fn claude_desired() -> serde_json::Value {
    json!({
        "hooks": {
            "PostToolUse": [rgt_group("rgt hook post")],
            "PreToolUse": [rgt_group("rgt hook pre")],
        }
    })
}

// (a) JSONC merge preserves unrelated content byte-for-byte incl. comments/trailing commas
#[test]
fn jsonc_merge_preserves_foreign_groups_comments_and_trailing_commas() {
    let input = r#"{
  // RTK's own hook group — must survive
  "hooks": {
    "PostToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "rtk hook post" }] },
    ],
  },
  "keybindings": { "esc": "stop" },
}
"#;
    let (out, changed) = jsonc_merge_hook_entries(input, &claude_desired()).unwrap();
    assert!(changed);
    assert!(out.contains("// RTK's own hook group — must survive"));
    assert!(out.contains("\"matcher\": \"Bash\""));
    assert!(out.contains("\"rtk hook post\""));
    assert!(out.contains("\"keybindings\": { \"esc\": \"stop\" }"));
    assert!(out.contains("\"rgt hook post\""));
    assert!(out.contains("\"rgt hook pre\""));
    // untouched prefix byte-for-byte
    assert!(out.starts_with("{\n  // RTK's own hook group — must survive\n"));
    // output is valid JSONC (re-parse tolerantly)
    jsonc_parser::parse_to_value(&out, &Default::default()).expect("output must parse as JSONC");
}

// (a) idempotent: second run changes nothing
#[test]
fn jsonc_merge_is_idempotent_after_first_edit() {
    let input = r#"{
  // comment
  "hooks": {
    "PostToolUse": [ { "matcher": "", "hooks": [{ "type": "command", "command": "rgt hook post" }] } ],
  },
}
"#;
    let (out, changed) = jsonc_merge_hook_entries(input, &claude_desired()).unwrap();
    assert!(changed);
    let (out2, changed2) = jsonc_merge_hook_entries(&out, &claude_desired()).unwrap();
    assert!(!changed2, "second merge must be a no-op");
    assert_eq!(out, out2);
}

// (c) existing RGT entries are replaced, not duplicated
#[test]
fn jsonc_merge_replaces_existing_rgt_entries_without_duplication() {
    let input = r#"{
  "hooks": {
    "PostToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "user hook" }] },
      { "matcher": "", "hooks": [{ "type": "command", "command": "rgt hook post" }] },
    ],
  },
}
"#;
    let (out, _) = jsonc_merge_hook_entries(input, &claude_desired()).unwrap();
    assert_eq!(
        out.matches("rgt hook post").count(),
        1,
        "must not duplicate"
    );
    assert!(out.contains("user hook"));
    assert!(out.contains("rgt hook pre"));
}

// (b) genuinely invalid JSON is a hard error
#[test]
fn jsonc_merge_rejects_genuinely_invalid_json() {
    let input = r#"{ "hooks": {"PostToolUse": [} "#;
    assert!(jsonc_merge_hook_entries(input, &claude_desired()).is_err());
}

// (b) JSONC that ALSO contains invalid JSON elsewhere is rejected (no partial merge)
#[test]
fn jsonc_merge_rejects_jsonc_with_invalid_json_elsewhere() {
    let input = r#"{
  // ok comment
  "hooks": { "PostToolUse": [ { "matcher": "", "hooks": [{ "type": "command", "command": "rgt hook post" }] } ] },
  "broken": ,
}
"#;
    assert!(jsonc_merge_hook_entries(input, &claude_desired()).is_err());
}

// (h) unexpected `hooks` shape (object instead of array) is unmergeable
#[test]
fn jsonc_merge_rejects_wrong_hooks_shape() {
    let input = r#"{ "hooks": { "PostToolUse": {} } }"#;
    assert!(jsonc_merge_hook_entries(input, &claude_desired()).is_err());
}

// (b) empty/whitespace-only file is treated as absent and initialized
#[test]
fn jsonc_merge_initializes_empty_file() {
    let (out, changed) = jsonc_merge_hook_entries("  \n", &claude_desired()).unwrap();
    assert!(changed);
    assert!(out.contains("\"rgt hook post\""));
    jsonc_parser::parse_to_value(&out, &Default::default()).expect("must parse");
}

// (d) TOML array ensure preserves comments/formatting, never duplicates [plugins]
#[test]
fn toml_ensure_array_value_preserves_comments_and_never_duplicates_table() {
    let input = "# user comment\n[plugins]\nenabled = [  # keep me\n  \"git\",\n]\n";
    let (out, changed) = toml_ensure_array_value(input, &["plugins", "enabled"], "rgt").unwrap();
    assert!(changed);
    assert!(out.contains("# user comment"));
    assert!(out.contains("# keep me"));
    assert!(out.contains("\"git\""));
    assert!(out.contains("\"rgt\""));
    assert_eq!(out.matches("[plugins]").count(), 1, "no duplicate table");
    // re-parse valid
    let _: toml_edit::DocumentMut = out.parse().expect("must parse as TOML");
    // idempotent
    let (out2, changed2) = toml_ensure_array_value(&out, &["plugins", "enabled"], "rgt").unwrap();
    assert!(!changed2);
    assert_eq!(out, out2);
}

// (e) malformed TOML is a hard error
#[test]
fn toml_ensure_array_value_rejects_malformed_toml() {
    let input = "[plugins\nenabled = [\n";
    assert!(toml_ensure_array_value(input, &["plugins", "enabled"], "rgt").is_err());
}

// Gemini PostToolUse command ensure
#[test]
fn toml_ensure_table_entry_sets_gemini_command() {
    let input = "# comment\n[PostToolUse]\ncommand = \"other\"\n";
    assert!(toml_ensure_table_entry(
        input,
        &["PostToolUse"],
        "command",
        "rgt hook post --agent gemini"
    )
    .is_err());
    let (out, changed) = toml_ensure_table_entry(
        "",
        &["PostToolUse"],
        "command",
        "rgt hook post --agent gemini",
    )
    .unwrap();
    assert!(changed);
    assert!(out.contains("rgt hook post --agent gemini"));
}

// Vibe [[pre_tool]] array of tables
#[test]
fn toml_ensure_vibe_pre_tool_adds_entry() {
    let input = "[[pre_tool]]\nmatch = \"bash\"\ncommand = \"something else\"\n";
    let (out, changed) = toml_ensure_vibe_pre_tool(input, "rgt hook post --agent vibe").unwrap();
    assert!(changed);
    assert!(out.contains("rgt hook post --agent vibe"));
    assert!(out.contains("\"something else\""));
    assert!(out.contains("match = \"bash\""));
    assert!(out.contains("strict = false"));
    let _: toml_edit::DocumentMut = out.parse().expect("must parse as TOML");
}

// (f) text_replace_block preserves content below the end marker
#[test]
fn text_replace_block_preserves_content_below_block() {
    let block = "\n## RGT Integration\ninstructions\n<!-- /RGT Integration -->";
    let input = format!("line1\n{block}\n## My personal notes\nkeep me\n");
    let (out, changed) = text_replace_block(
        &input,
        "## RGT Integration",
        "<!-- /RGT Integration -->",
        block,
    )
    .unwrap();
    assert!(changed);
    assert!(
        out.contains("## My personal notes\nkeep me\n"),
        "notes below must survive"
    );
    assert!(out.contains("line1\n"));
}

// (f) text_replace_block errors on a legacy block (open marker, no end marker)
#[test]
fn text_replace_block_rejects_legacy_block_without_end_marker() {
    let input = "## RGT Integration\ninstructions\nuser notes below\n";
    let err = text_replace_block(
        input,
        "## RGT Integration",
        "<!-- /RGT Integration -->",
        "\n## RGT Integration\nnew\n<!-- /RGT Integration -->",
    )
    .unwrap_err();
    assert!(err.reason.contains("end marker"));
}

// (g) marker mid-line / inside a comment is NOT treated as an RGT block
#[test]
fn text_replace_block_ignores_midline_and_commented_markers() {
    let input = "some code ## RGT Integration\n<!-- ## RGT Integration -->\n";
    let block = "\n## RGT Integration\ninstructions\n<!-- /RGT Integration -->";
    let (out, changed) = text_replace_block(
        input,
        "## RGT Integration",
        "<!-- /RGT Integration -->",
        block,
    )
    .unwrap();
    assert!(changed);
    // original lines untouched
    assert!(out.contains("some code ## RGT Integration\n"));
    assert!(out.contains("<!-- ## RGT Integration -->\n"));
    // new block appended, not spliced mid-line
    assert!(out.ends_with("<!-- /RGT Integration -->\n"));
}
