# Tasks: Hook Editor Path Quoting Fix

**Input**: Design documents from `/specs/031-hook-editor-path-quoting-fix/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/command-recognition-contract.md, quickstart.md

**Tests**: Included — the spec requires empirical coverage of the spaced-path case (Constitution IX).

**Organization**: Single user story; small, self-contained change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the current (buggy) behavior.

- [x] T001 Reproduce the bug: add a quick throwaway assertion or inspect `is_rgt_command("\"/Applications/My App/rgt\" hook post")` returns false today (confirm via `cargo test` after adding the red unit test in T002); confirm `shell_quote` in `src/hooks/installer.rs:231` is the writer producing the quoted form

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The quote-aware tokenizer.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [x] T002 Add `fn split_first_token(cmd: &str) -> (String, &str)` to `src/hooks/editor.rs` (quote-aware first-token extraction honoring a leading `"` pair and `\` escapes; unquoted branch preserves current split behavior) and rewire `is_rgt_command` to use it (exe from token, remaining `hook pre|post [--agent X]` from the rest)

**Checkpoint**: `cargo build` compiles; existing tests still pass.

---

## Phase 3: User Story 1 - RGT Recognizes Its Own Hooks Under a Spaced Path (Priority: P1) 🎯 MVP

**Goal**: Spaced-path commands are recognized as RGT-owned; re-init is idempotent.

**Independent Test**: New unit tests in `tests/unit/hook_editors.rs` and a contract test in `tests/contract/hook_installer.rs`.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T003 [US1] Unit tests in `tests/unit/hook_editors.rs` for `is_rgt_command`: spaced quoted path (true), spaced nested `hooks[]` form via `element_has_rgt_command` (true), spaced non-RGT `"/usr/local/bin/rtk" hook post` (false), bare `rgt hook post` (true), unquoted `/usr/local/bin/rgt hook post` (true), Windows `"C:\Program Files\rgt.exe" hook pre` (true), `--agent` variant (true), unclosed-quote input (no panic)

### Implementation for User Story 1

- [x] T004 [US1] Add a contract test in `tests/contract/hook_installer.rs`: run `detect_and_configure_hooks_with_config_and_exe` twice with `injected_exe()` set to a spaced path (`/Applications/My App/rgt`), asserting the second run does not duplicate the RGT command entry and the settings.json contains exactly one such entry

**Checkpoint**: `cargo test --test test_hook_editors --test test_hook_installer_contract` green.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [x] T005 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; confirm no regressions and no production changes outside `src/hooks/editor.rs`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T002).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story.

### Within User Story 1

- Tests written first (T003), then implementation seam (T002 already done; T004 contract).

### Parallel Opportunities

- T003 (unit) and T004 (contract) touch different files and could run in parallel; both depend on T002.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002)
3. Phase 3: US1 (T003–T004)
4. **STOP and VALIDATE**: hook_editors + hook_installer_contract green

### Incremental Delivery

1. Foundational → quote-aware tokenizer
2. US1 → predicate + idempotency coverage for spaced paths
3. Polish → fmt/clippy/full test

---

## Notes

- The writer (`shell_quote`) is correct and unchanged; only the reader changes.
- No new dependencies (hand-rolled tokenizer, Constitution XI).
- The predicate only inspects the executable token + `hook pre|post`; trailing `--agent` is ignored.
