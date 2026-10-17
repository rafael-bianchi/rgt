# Tasks: Harness Hook Payload and Shell Extraction Hardening

**Input**: Feature specification `specs/048-harness-hook-payload-hardening/spec.md` and plan `specs/048-harness-hook-payload-hardening/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US5] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US1] Add test in `tests/contract/test_hooks_contract.rs` verifying that OpenCode capture succeeds for a file with a trailing newline.
- [x] T002 [US2] Add unit tests in `tests/unit/hook_parsers.rs` asserting that `default_dialect` parses Claude Code nested `{ "type": "text", "file": { "content": "..." } }` payload.
- [x] T003 [US3] Add unit tests in `tests/unit/hook_parsers.rs` asserting that Codex CLI `execute_command` / `Bash` is qualified when reading a file.
- [x] T004 [US4] Add unit tests in `tests/unit/hook_editors.rs` asserting that `shell_tokens` preserves Windows backslash paths like `C:\Users\foo\bin\rgt.exe`.
- [x] T005 [US5] Add test in `tests/contract/test_pdf_capture_contract.rs` asserting that Claude Code `additionalContext` is suppressed when disk snapshot fails.

---

## Phase 2: Implementation

- [x] T006 [US1] In `src/hooks/mod.rs`, update `matching_file_snapshot` to accept snapshots differing only by a single trailing `\n` or `\r\n`, and update `opencode_native_read_dialect` in `src/hooks/parser.rs` for `line_count` flexibility.
- [x] T007 [US2] In `src/hooks/parser.rs`, update `default_dialect` to extract text from nested `file.content` and `text` fields.
- [x] T008 [US3] In `src/hooks/parser.rs`, update `is_qualifying_read_event` to qualify Codex CLI shell commands (`Bash`, `bash`, `execute_command`, `ExecuteCommand`, `exec`).
- [x] T009 [US4] In `src/hooks/editor.rs`, update `shell_tokens` to treat unquoted `\` as an escape only when followed by whitespace, quotes, or another backslash.
- [x] T010 [US5] In `src/hooks/mod.rs`, update `handle_pdf_capture` to skip `additionalContext` stdout when `path.is_some() && !recorded`.

---

## Phase 3: Verification & Parity

- [x] T011 Run the complete test suite (`cargo test </dev/null`) and verify all tests pass.
- [x] T012 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
