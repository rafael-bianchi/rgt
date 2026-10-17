# Tasks: Capture Bash-Tool Reads

**Input**: Design documents from `/specs/026-capture-bash-tool-reads/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed behavior.

**Organization**: Tasks are grouped by user story; the parser primitives (rtk/grep + stdout) are foundational because both US1 and US2 depend on them.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/hooks/parser.rs`, `tests/unit/hook_parsers.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates)

- [x] T001 Confirm `serde_json` is already present in `Cargo.toml` (no new dependencies — see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The parser primitives every user story depends on — `rtk`/`grep` recognition in `extract_path_from_command` and stdout-as-content in `command_dialect`

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `tests/unit/hook_parsers.rs`: `extract_path_from_command("rtk read /p/f")` == `Some("/p/f")`; `extract_path_from_command("rtk cat /p/f")` == `None`; `extract_path_from_command("grep -r x /p")` == `Some("/p")`; `extract_path_from_command("cat /p/f")` still works; `command_dialect` on a Bash payload with `tool_response.stdout` returns `content == Some(stdout)`

### Implementation for Foundational Primitives

- [x] T003 In `extract_path_from_command` (`src/hooks/parser.rs`), recognize `rtk` as a wrapper (first token `rtk` → check the second token against the read-style list) and add `grep` to `READ_STYLE` (FR-002)
- [x] T004 In `command_dialect` (`src/hooks/parser.rs`), read `tool_response.stdout` and set `content` when non-empty; leave `content: None` (path-only) otherwise (FR-003)

**Checkpoint**: foundational tests pass; `rtk read`/`grep` and stdout captured

---

## Phase 3: User Story 1 - Bash Reads Are Captured for Claude Code and Cursor (Priority: P1) 🎯 MVP

**Goal**: `normalize_agent_event` for `None`/`claude-code`/`cursor` falls back to command-based extraction when the Read-shaped dialect yields no path (FR-001)

**Independent Test**: The §8.1 Bash payload normalizes to a non-empty path + content; a native `Read` event is unchanged

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T005 [P] [US1] Unit tests in `tests/unit/hook_parsers.rs`: `normalize_agent_event(None|claude-code|cursor, bash_payload)` returns a capture with `path == Some(...)` and `content == Some(...)`; a native `Read`-shaped payload still normalizes via the default dialect (path + content as before)

### Implementation for User Story 1

- [x] T006 [US1] In `normalize_agent_event` (`src/hooks/parser.rs`), for `None`/`claude-code`/`cursor`, call `default_dialect` first and fall back to `command_dialect` when the result has `path == None` (FR-001)

**Checkpoint**: US1 tests pass; Bash reads are captured for the two flagship agents

---

## Phase 4: User Story 2 - RTK's `rtk read` Rewrite Is Recognized (Priority: P1)

**Goal**: An `rtk read <path>` command (RTK's auto-rewrite of `cat`/`read`) is captured end-to-end (FR-002, built on T003)

**Independent Test**: An end-to-end normalize of an `rtk read` Bash payload yields the path

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T007 [P] [US2] Unit test in `tests/unit/hook_parsers.rs`: `normalize_agent_event(None, {"tool_input":{"command":"rtk read /p/report.csv"},"tool_response":{"stdout":"amount,42\n"}})` returns `path == Some("/p/report.csv")` and `content == Some("amount,42\n")`; an `rtk` command with a non-read subcommand stays a no-op

### Implementation for User Story 2

- [x] T008 [US2] No new code — T003 provides the wrapper recognition; verify the end-to-end path is exercised (if a gap appears, adjust `command_dialect`/`extract_path_from_command`)

**Checkpoint**: US2 tests pass; RTK-paired setups capture reads

---

## Phase 5: User Story 3 - Captured Output Is Used, With a Disk Fallback (Priority: P2)

**Goal**: stdout is used as content when present; path-only otherwise; RGT stays passive (FR-003/FR-004)

**Independent Test**: A path-only Bash event yields `content == None` and `path == Some`; a non-read command is a no-op; no command is mutated

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T009 [P] [US3] Unit tests in `tests/unit/hook_parsers.rs`: a Bash read with empty/absent stdout → `path` set and `content == None`; a non-read Bash command (`ls`, `git status`) → `path == None`; assert the captured `command` string is never rewritten (compare input vs any stored value)

### Implementation for User Story 3

- [x] T010 [US3] No new code — T004 provides stdout/fallback behavior; the disk fallback already exists in `handle_passive_hook_event` (unchanged)

**Checkpoint**: US3 tests pass; passive, correct fallback behavior

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation

- [x] T011 [P] Update `README.md`'s agent matrix / capture note to state that Claude Code and Cursor Bash reads (`cat`/`head`/`tail`/`grep`/`rtk read`, ...) are now captured
- [x] T012 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T013 Run the manual scenarios in `specs/026-capture-bash-tool-reads/quickstart.md` (Bash read captured; RTK wrapper)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational
- **US2 (Phase 4)**: Depends on Foundational (T003)
- **US3 (Phase 5)**: Depends on Foundational (T004)
- **Polish (Phase 6)**: Depends on all

### User Story Dependencies

- **US1 (P1)**: after Foundational — dispatch fallback
- **US2 (P1)**: after Foundational — rtk wrapper (impl in T003)
- **US3 (P2)**: after Foundational — stdout/fallback (impl in T004)

### Within Each User Story

- Tests written first, must FAIL before implementation
- Foundational primitives (T003/T004) before the story wiring (T006)
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 ∥ T005 ∥ T007 ∥ T009 (all `tests/unit/hook_parsers.rs`, different test functions)
- T003 ∥ T004 (same file — run sequentially, but independent primitives)
- T011 ∥ T012

---

## Parallel Example: Foundational + Story Tests

```bash
Task: "rtk/grep + stdout tests (hook_parsers.rs)"
Task: "dispatch fallback tests (US1)"
Task: "rtk end-to-end tests (US2)"
Task: "fallback/passive tests (US3)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + 2 (Foundational: rtk/grep + stdout primitives)
2. Phase 3: User Story 1 (dispatch fallback) — the §8 P1 capture gap
3. **STOP and VALIDATE**: Bash reads are captured for Claude Code/Cursor
4. Then US2/US3 incrementally

### Incremental Delivery

1. Foundational → rtk/grep + stdout primitives
2. US1 → dispatch fallback (MVP)
3. US2 → rtk end-to-end
4. US3 → fallback/passive behavior
5. Polish → docs + full-suite validation

---

## Notes

- [P] tasks = different files, no dependencies; US1/US2/US3 all edit `src/hooks/parser.rs`, so their *implementation* tasks run sequentially (the test tasks are independent)
- [Story] label maps task to user story for traceability
- US2/US3 have no new implementation beyond the Foundational primitives (T003/T004); their tasks exist to lock in the behavior with tests
- Commit after each task or logical group; stop at each checkpoint
