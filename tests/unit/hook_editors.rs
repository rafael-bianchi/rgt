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

#[test]
fn jsonc_merge_preserves_foreign_absolute_hook_executable() {
    let input = r#"{
  "hooks": {
    "PostToolUse": [
      { "matcher": "Read", "hooks": [{ "type": "command", "command": "\"/opt/vendor path/read-hook\" hook post" }] }
    ]
  }
}"#;
    let (out, changed) = jsonc_merge_hook_entries(input, &claude_desired()).unwrap();
    assert!(changed);
    assert!(out.contains("/opt/vendor path/read-hook"));
    assert_eq!(out.matches("rgt hook post").count(), 1);
}

#[test]
fn jsonc_merge_rejects_mixed_rgt_and_foreign_commands_in_one_group() {
    let input = r#"{
  "hooks": {
    "PostToolUse": [
      { "matcher": "Read", "hooks": [
        { "type": "command", "command": "rgt hook post" },
        { "type": "command", "command": "user audit-hook" }
      ] }
    ]
  }
}"#;
    let error = jsonc_merge_hook_entries(input, &claude_desired()).unwrap_err();
    assert!(error.reason.contains("mixed"), "unexpected error: {error}");
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

#[test]
fn toml_ensure_array_value_handles_inline_plugins_without_losing_other_values() {
    let current = "# user comment\nplugins = { enabled = [\"git\", \"rgt\"], theme = \"dark\" }\n";
    let (unchanged, changed) =
        toml_ensure_array_value(current, &["plugins", "enabled"], "rgt").unwrap();
    assert!(!changed);
    assert_eq!(unchanged, current);

    let missing = "# user comment\nplugins = { enabled = [\"git\"], theme = \"dark\" }\n";
    let (updated, changed) =
        toml_ensure_array_value(missing, &["plugins", "enabled"], "rgt").unwrap();
    assert!(changed);
    assert!(updated.contains("# user comment"));
    let doc: toml_edit::DocumentMut = updated.parse().unwrap();
    let inline = doc["plugins"]
        .as_value()
        .unwrap()
        .as_inline_table()
        .unwrap();
    assert_eq!(
        inline.get("theme").and_then(toml_edit::Value::as_str),
        Some("dark")
    );
    let enabled = inline.get("enabled").unwrap().as_array().unwrap();
    assert_eq!(
        enabled
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>(),
        vec!["git", "rgt"]
    );

    let missing_entry = "# user comment\nplugins = { theme = \"dark\" }\n";
    let (updated, changed) =
        toml_ensure_array_value(missing_entry, &["plugins", "enabled"], "rgt").unwrap();
    assert!(changed);
    let doc: toml_edit::DocumentMut = updated.parse().unwrap();
    let inline = doc["plugins"]
        .as_value()
        .unwrap()
        .as_inline_table()
        .unwrap();
    assert_eq!(
        inline.get("theme").and_then(toml_edit::Value::as_str),
        Some("dark")
    );
    let enabled = inline.get("enabled").unwrap().as_array().unwrap();
    assert_eq!(
        enabled
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>(),
        vec!["rgt"]
    );
}

#[test]
fn toml_ensure_array_value_rejects_non_string_enablement() {
    for text in [
        "[plugins]\nenabled = [\"rgt\", 42]\n",
        "plugins = { enabled = [\"rgt\", 42] }\n",
        "[plugins]\nenabled = \"rgt\"\n",
        "plugins = { enabled = \"rgt\" }\n",
    ] {
        let error = toml_ensure_array_value(text, &["plugins", "enabled"], "rgt").unwrap_err();
        assert!(error.to_string().contains("enabled"), "{error}");
    }
}

// Gemini PostToolUse command ensure
#[test]
fn toml_ensure_table_entry_sets_gemini_command() {
    let input = "# comment\n[PostToolUse]\ncommand = \"other\"\n";
    assert!(toml_ensure_table_entry(
        input,
        &["PostToolUse"],
        "command",
        "rgt hook post --agent gemini",
        "gemini"
    )
    .is_err());
    let (out, changed) = toml_ensure_table_entry(
        "",
        &["PostToolUse"],
        "command",
        "rgt hook post --agent gemini",
        "gemini",
    )
    .unwrap();
    assert!(changed);
    assert!(out.contains("rgt hook post --agent gemini"));
}

#[test]
fn toml_ensure_table_entry_migrates_only_an_rgt_owned_command() {
    let legacy = "# keep\n[PostToolUse]\ncommand = \"rgt hook post --agent gemini\"\n";
    let (out, changed) = toml_ensure_table_entry(
        legacy,
        &["PostToolUse"],
        "command",
        "\"/new/path/rgt\" hook post --agent gemini --rgt-managed",
        "gemini",
    )
    .unwrap();
    assert!(changed);
    assert!(out.contains("# keep"));
    assert!(out.contains("--rgt-managed"));

    let foreign = "[PostToolUse]\ncommand = \"/opt/vendor/hook hook post --agent gemini\"\n";
    assert!(toml_ensure_table_entry(
        foreign,
        &["PostToolUse"],
        "command",
        "rgt hook post --agent gemini",
        "gemini",
    )
    .is_err());
}

#[test]
fn vibe_migration_updates_one_owned_hook_and_removes_owned_duplicates() {
    let input = "# user comment\n[[pre_tool]]\nmatch = \"shell\"\ncommand = \"echo keep\"\n\n[[pre_tool]]\ncommand = \"rgt hook post --agent vibe\"\n\n[[pre_tool]]\ncommand = \"rgt hook post --agent vibe\"\n";
    let (out, changed) = toml_ensure_vibe_pre_tool(
        input,
        "\"/new/path/rgt\" hook pre --agent vibe --rgt-managed",
    )
    .unwrap();
    assert!(changed);
    assert!(out.contains("# user comment"));
    assert!(out.contains("echo keep"));
    assert_eq!(out.matches("--rgt-managed").count(), 1);
    assert_eq!(out.matches("--agent vibe").count(), 1);
    assert!(out.contains("hook pre --agent vibe"));
    let (same, changed) = toml_ensure_vibe_pre_tool(
        &out,
        "\"/new/path/rgt\" hook pre --agent vibe --rgt-managed",
    )
    .unwrap();
    assert!(!changed);
    assert_eq!(same, out);
}

// Vibe [[pre_tool]] array of tables
#[test]
fn toml_ensure_vibe_pre_tool_adds_entry() {
    let input = "[[pre_tool]]\nmatch = \"bash\"\ncommand = \"something else\"\n";
    let (out, changed) = toml_ensure_vibe_pre_tool(input, "rgt hook pre --agent vibe").unwrap();
    assert!(changed);
    assert!(out.contains("rgt hook pre --agent vibe"));
    assert!(out.contains("\"something else\""));
    assert!(out.contains("match = \"bash\""));
    assert!(out.contains("strict = false"));
    let _: toml_edit::DocumentMut = out.parse().expect("must parse as TOML");
}

// (f) text_replace_block preserves content below the end marker
#[test]
fn text_replace_block_preserves_content_below_block() {
    let block = "\n## RGT Integration\ninstructions\n<!-- /RGT Integration -->";
    let input = "line1\n\n## RGT Integration\nold instructions\n<!-- /RGT Integration -->\n## My personal notes\nkeep me\n";
    let (out, changed) = text_replace_block(
        input,
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
