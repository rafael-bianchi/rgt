#[cfg(test)]
mod tests {
    use rgt::hooks::parser::{extract_values_from_content, parse_hook_payload, NumberFormat};
    use rgt::types::ValueKind;

    #[test]
    fn test_parse_post_tool_use_payload() {
        let payload_json = r#"{
            "event": "PostToolUse",
            "tool_name": "ReadLocalFile",
            "tool_input": { "path": "docs/budget.md" },
            "tool_response": { "content": "Budget: 15000. Start Date: 2026-01-01" }
        }"#;

        let parsed = parse_hook_payload(payload_json).unwrap();
        assert_eq!(parsed.event.as_deref(), Some("PostToolUse"));
        assert_eq!(
            parsed.tool_input.as_ref().unwrap().path.as_deref(),
            Some("docs/budget.md")
        );

        let content = parsed
            .tool_response
            .as_ref()
            .unwrap()
            .content
            .as_deref()
            .unwrap();
        let extracted = extract_values_from_content(content, NumberFormat::Auto).values;
        assert!(!extracted.is_empty());
        assert!(extracted
            .iter()
            .any(|v| v.value.kind() == ValueKind::Number));
        assert!(extracted.iter().any(|v| v.value.kind() == ValueKind::Date));
    }
}
