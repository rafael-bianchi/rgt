# Tasks: Non-Text Source Capture

**Input**: Design documents from `/specs/023-non-text-source-capture/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/hooks/glue/mod.rs`, `src/hooks/installer.rs`, `src/cli/record.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates)

- [x] T001 Confirm no new dependencies are required (the instruction is an embedded `const`; the record change uses `String::from_utf8` — see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared instruction text every in-scope agent's artifact depends on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T002 Add `pub const NON_TEXT_SOURCE_INSTRUCTION: &str` to `src/hooks/glue/mod.rs` with the findings §6.1 instruction text (scoped to PDF/Excel/images, explicitly stating plain-text/CSV/JSON/Markdown are auto-captured, showing the `rgt record --stdin` / intermediate-text-file reporting paths). The text MUST contain the anchor phrases `Recording Values from Non-Text Sources` and `already captured automatically`, which the contract tests assert against

**Checkpoint**: the constant compiles; its exact wording is fixed for the story tests

---

## Phase 3: User Story 1 - `rgt record` Reports Unsupported Formats Clearly (Priority: P1) 🎯 MVP

**Goal**: `rgt record` on a binary/non-UTF-8 input returns a clear "unsupported file format for `rgt record`" error naming the supported path (FR-001), for both file and `--stdin`

**Independent Test**: A contract test feeds a non-UTF-8 fixture and asserts the error names "unsupported file format" and not the raw UTF-8 message

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T003 [P] [US1] Contract test in `tests/contract/test_cli_record_derive_contract.rs`: `execute_record` on a non-UTF-8 file returns an error containing "unsupported file format for `rgt record`" (and NOT "stream did not contain valid UTF-8"); a plain-text file still succeeds

### Implementation for User Story 1

- [x] T004 [US1] In `execute_record` (`src/cli/record.rs`), read input as bytes (`fs::read` for files, `io::stdin().read_to_end` for `--stdin`) and `String::from_utf8`; on UTF-8 failure return the explicit unsupported-format error (FR-001)

**Checkpoint**: US1 contract test passes

---

## Phase 4: User Story 2 - Agents Are Told to Report Non-Text Values (Priority: P1)

**Goal**: The instruction is appended to the 7 rules artifacts (FR-002), scoped to non-text formats (FR-004)

**Independent Test**: Contract tests assert the instruction text appears exactly once in each of the 7 artifacts after `rgt init`

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T005 [P] [US2] Contract tests in `tests/contract/hook_installer.rs`: after `rgt init` for codex, windsurf, cline, antigravity, kilocode, copilot, and vibe, assert each instructions file contains the anchor phrase `Recording Values from Non-Text Sources` exactly once and the anchor phrase `already captured automatically` (the plain-text exclusion)

### Implementation for User Story 2

- [x] T006 [US2] Splice `glue::NON_TEXT_SOURCE_INSTRUCTION` into the block content of the 7 rules artifacts in `src/hooks/installer.rs`: `codex_section()` (Codex), Windsurf `rules_content` in `write_windsurf`, and the `glue::{CLINE_RULES, ANTIGRAVITY_RULES, KILO_RULES, COPILOT_CLI_RULES, VIBE_PROMPT}` blocks as written via `ensure_text_block` (append the instruction before the end marker)

**Checkpoint**: US2 contract tests pass; all 7 artifacts carry the instruction once

---

## Phase 5: User Story 3 - Claude Code Gets the Same Instruction (Priority: P2)

**Goal**: `rgt init --agent claude-code` writes a `CLAUDE.md` (project by default, `~/.claude/CLAUDE.md` with `--global`) carrying the instruction (FR-003)

**Independent Test**: Contract test asserts `CLAUDE.md` is written with the instruction, idempotently, preserving user content

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T007 [P] [US3] Contract test in `tests/contract/hook_installer.rs`: `rgt init --agent claude-code` writes a project `CLAUDE.md` containing the instruction; with `--global` writes `~/.claude/CLAUDE.md`; re-run is idempotent and user content outside the RGT block is preserved

### Implementation for User Story 3

- [x] T008 [US3] In `write_claude_code` (`src/hooks/installer.rs`), additionally write a `CLAUDE.md` block via the existing `ensure_text_block` helper: open marker `## RGT Integration`, body = `glue::NON_TEXT_SOURCE_INSTRUCTION`, end marker `glue::RGT_END_MARKER` (project `PathBuf::from("CLAUDE.md")`, or `home/.claude/CLAUDE.md` with `--global`)

**Checkpoint**: US3 contract test passes

---

## Phase 6: User Story 4 - Remaining Agents Verified Before Adding Instructions (Priority: P3)

**Goal**: No guessed instructions for unverified/plugin-only agents (FR-006)

**Independent Test**: Contract test asserts no instructions file is written for cursor, copilot (VS Code Chat), gemini, opencode, pi, hermes as part of this feature

### Tests for User Story 4 (write FIRST, expect FAIL) ⚠️

- [x] T009 [P] [US4] Contract test in `tests/contract/hook_installer.rs`: after `rgt init` for cursor/copilot/gemini/opencode/pi/hermes, assert NO `CLAUDE.md`-equivalent or instructions file carrying the instruction was created (only the existing hook/plugin artifacts)

### Implementation for User Story 4

- [x] T010 [US4] Confirm no instructions-writing code path exists for these agents (no code change expected); if the test surfaces a path, remove/guard it

**Checkpoint**: US4 test passes; unverified agents untouched

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation

- [x] T011 [P] Update `README.md` (Record command + agents table) to document the non-text-source reporting behavior and the new `rgt record` unsupported-format error
- [x] T012 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T013 Run the manual scenarios in `specs/023-non-text-source-capture/quickstart.md` (PDF record error; instruction present in `.clinerules` and `CLAUDE.md`)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS US2/US3
- **US1 (Phase 3)**: Depends on Setup only (independent of the instruction const)
- **US2 (Phase 4)**: Depends on Foundational
- **US3 (Phase 5)**: Depends on Foundational
- **US4 (Phase 6)**: Depends on none (asserts absence)
- **Polish (Phase 7)**: Depends on all

### User Story Dependencies

- **US1 (P1)**: independent — the record error
- **US2 (P1)**: after Foundational — 7 rules artifacts
- **US3 (P2)**: after Foundational — Claude Code `CLAUDE.md`
- **US4 (P3)**: independent — absence assertion

### Within Each User Story

- Tests written first, must FAIL before implementation
- Instruction constant (T002) before splicing (US2/US3)
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T003 ∥ T005 ∥ T007 ∥ T009 (test files, different concerns)
- T011 ∥ T012 (Polish, different concerns)
- US2/US3 implementation both edit `src/hooks/installer.rs` → sequential

---

## Parallel Example: Story Tests

```bash
Task: "US1 record-error contract test (test_cli_record_derive_contract.rs)"
Task: "US2 instruction-in-rules contract tests (hook_installer.rs)"
Task: "US3 CLAUDE.md contract test (hook_installer.rs)"
Task: "US4 no-guessed-instructions contract test (hook_installer.rs)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + 2 (Foundational: instruction const)
2. Phase 3: User Story 1 (record error) — independent, no instruction needed
3. **STOP and VALIDATE**: binary record gives a clear error
4. Then US2/US3/US4 incrementally

### Incremental Delivery

1. Foundational → instruction const fixed
2. US1 → clear record error (MVP)
3. US2 → 7 rules artifacts carry the instruction
4. US3 → Claude Code `CLAUDE.md`
5. US4 → unverified agents untouched (assertion)
6. Polish → docs + full-suite validation

---

## Notes

- [P] tasks = different files, no dependencies; US2/US3 impl tasks are sequential (both edit `src/hooks/installer.rs`)
- [Story] label maps task to user story for traceability
- The instruction wording is fixed once in T002 and asserted by string match in the contract tests (do not paraphrase in tests)
- Unverified agents (cursor/copilot/gemini/opencode/pi/hermes) must NOT gain instructions (FR-006); the US4 test locks this in
- Commit after each task or logical group; stop at each checkpoint

---

## Phase 8: Convergence

- [x] T014 [US1] Add a contract test in `tests/contract/test_cli_record_derive_contract.rs` feeding non-UTF-8 bytes via `--stdin` (an existing file path + binary stdin) and asserting the same "unsupported file format for `rgt record`" error, per FR-001 (partial — file-path branch is tested, stdin branch is not)
