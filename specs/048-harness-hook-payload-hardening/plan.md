# Implementation Plan: Harness Hook Payload and Shell Extraction Hardening

## Proposed Changes

### 1. `src/hooks/mod.rs`
- In `matching_file_snapshot`:
  - Check whether `current == expected_bytes` OR `current` has a single trailing `\n` or `\r\n` compared to `expected_bytes`.
  - Maintain BLAKE3 hashing of `current` (the on-disk file).
- In `handle_pdf_capture`:
  - If `path.is_some() && !recorded`, do not emit `additionalContext` JSON.

### 2. `src/hooks/parser.rs`
- In `opencode_native_read_dialect`:
  - Allow `line_count == total_lines || line_count + 1 == total_lines`.
- In `default_dialect`:
  - Add helper `extract_text_content` inspecting `tool_response` for `file.content`, `text`, or `content`.
- In `is_qualifying_read_event`:
  - Support `Some("codex")` with `Bash | bash | execute_command | ExecuteCommand | exec` when command extracts a path.

### 3. `src/hooks/editor.rs`
- In `shell_tokens`:
  - When encountering unquoted `\`, peek next character. If it is whitespace, `'`, `"`, or `\`, treat as escape; otherwise preserve literal `\` as path separator.

### 4. Tests
- Add tests in `tests/unit/hook_parsers.rs` for Claude Code nested `Read` response and Codex CLI shell commands.
- Add tests in `tests/unit/hook_editors.rs` for Windows backslash paths in `shell_tokens`.
- Add tests in `tests/contract/test_pdf_capture_contract.rs` verifying no `additionalContext` on snapshot mismatch.
- Add tests in `tests/contract/test_hooks_contract.rs` or unit tests verifying OpenCode newline-terminated file capture.
