# Spec 048: Harness Hook Payload and Shell Extraction Hardening

## Overview
This specification hardens tool payload parsing and shell extraction across agent integrations (Claude Code, OpenCode, Codex CLI, and Windows paths) to eliminate silent drops and extraction flaws identified in the critical code review.

## Requirements

### FR-001: OpenCode Trailing Newline Tolerant Snapshot & Line Count
- When OpenCode's `read` tool strips the trailing `\n` or `\r\n` from a file, `matching_file_snapshot` must accept the snapshot as matching if `current` differs from `expected_bytes` only by that single trailing newline.
- The recorded BLAKE3 hash must remain that of `current` (the full on-disk content).
- In `opencode_native_read_dialect`, line count validation must accept `line_count == total_lines` or `line_count + 1 == total_lines` so that trailing newline files are not discarded.

### FR-002: Claude Code Nested Payload Extraction
- In `default_dialect`, extract content from `tool_response` when formatted as:
  - top-level `"content"`
  - nested `{"file": {"content": "..."}}`
  - typed text `{"type": "text", "file": {"content": "..."}}`
  - top-level `"text"`
- Native Claude Code `Read` tool responses with nested file envelopes must be capturable.

### FR-003: Codex CLI Shell Read Event Qualification
- In `is_qualifying_read_event`, qualify shell read commands for agent `codex` when `tool_name` is `Bash`, `bash`, `execute_command`, `ExecuteCommand`, or `exec` and a target path was extracted from the command.
- This ensures `command_dialect` for Codex CLI is not dead code.

### FR-004: Windows Path Separators in Shell Tokenizer
- In `shell_tokens` (`src/hooks/editor.rs`), backslashes (`\`) outside of quotes must not escape following alphanumeric/path characters (e.g., `C:\Users\...\rgt.exe`).
- An unquoted backslash only acts as an escape when followed by whitespace, quote characters (`'`, `"`), or another backslash (`\`).

### FR-005: PDF Additional Context Gating
- In `handle_pdf_capture` (`src/hooks/mod.rs`), Claude Code stdout `additionalContext` injection must be suppressed if a target path was provided but `record_pdf_values` returned `false` (e.g. disk snapshot verification failure).
