# Tasks: Fix Agent Hook Formats (RTK-Informed)

**Input**: Design documents from `/specs/010-fix-agent-hook-formats/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Integration tests must be updated to assert corrected formats per FR-006.

**Organization**: Tasks grouped by user story. All 4 stories modify the same file (`src/hooks/installer.rs`) — implement sequentially to avoid conflicts.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths

## Phase 1: Setup

**Purpose**: No initialization needed. Verify current state.

- [ ] T001 Verify current installer code — read `src/hooks/installer.rs` lines 23-111 to confirm 4 agents with broken formats
- [ ] T002 [P] Verify current integration tests — read `tests/integration/test_hooks_installer.rs` to confirm 4 test functions asserting old formats

---

## Phase 2: Foundational

**Purpose**: None needed. Proceed to user stories.

---

## Phase 3: User Story 1 — Claude Code Hook (Priority: P1) 🎯 MVP

**Goal**: Replace `.claude/hooks.json` (flat schema) with `.claude/settings.json` (nested schema with merge support).

**Independent Test**: `HOME=/tmp/rgt-test cargo run -- init -g --agent claude-code`. Verify `.claude/settings.json` exists with correct hooks schema and `.claude/hooks.json` does not.

### Implementation

- [ ] T003 [US1] Replace Claude Code hook logic in `src/hooks/installer.rs` (lines 23-46) — change file path from `claude_dir.join("hooks.json")` to `claude_dir.join("settings.json")`, update JSON schema to `{"hooks": {"PostToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook post"}]}], "PreToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook pre"}]}]}}`
- [ ] T004 [US1] Add JSON merge logic to `src/hooks/installer.rs` — if `settings.json` exists, read and parse existing JSON, preserve non-`hooks` keys (`permissions`, etc.), add/update the `hooks` key with RGT entries
- [ ] T005 [US1] Add cleanup of old `.claude/hooks.json` on `--force` in `src/hooks/installer.rs` — remove the old file if it exists and warning about deprecated format
- [ ] T006 [US1] Verify manually — run `cargo run -- init -g --agent claude-code` in a temp HOME, check file exists with correct schema

---

## Phase 4: User Story 2 — Cursor Hook (Priority: P1)

**Goal**: Fix Cursor `.cursor/hooks.json` — camelCase event names, `"version": 1`, per-entry `"matcher": "Shell"`.

**Independent Test**: `HOME=/tmp/rgt-test cargo run -- init -g --agent cursor`. Verify `.cursor/hooks.json` has `"version": 1` and `preToolUse`/`postToolUse` (camelCase).

### Implementation

- [ ] T007 [US2] Replace Cursor hook logic in `src/hooks/installer.rs` (lines 48-69) — add `"version": 1`, change event keys from `"PreToolUse"`/`"PostToolUse"` to `"preToolUse"`/`"postToolUse"`, restructure to `{"hooks": {"preToolUse": [{"command": "rgt hook pre", "matcher": "Shell"}], "postToolUse": [{"command": "rgt hook post", "matcher": "Shell"}]}}`
- [ ] T008 [US2] Verify manually — run `cargo run -- init -g --agent cursor` in a temp HOME, check file has correct schema

---

## Phase 5: User Story 3 — Windsurf Rules-File (Priority: P3)

**Goal**: Convert Windsurf from hook JSON (`.codeium/windsurf/hooks.json`) to rules-file (`.windsurfrules`).

**Independent Test**: `cd /tmp/rgt-test && cargo run -- init -g --agent windsurf`. Verify `.windsurfrules` exists with RGT instructions and no `.codeium/windsurf/hooks.json` created.

### Implementation

- [ ] T009 [US3] Replace Windsurf hook logic in `src/hooks/installer.rs` (lines 91-111) — remove hook JSON, write `.windsurfrules` with plain text instructions explaining `rgt` prefix usage
- [ ] T010 [US3] Update output message in `src/hooks/installer.rs` — change configured message from "Windsurf -> ~/.codeium/windsurf/hooks.json" to "Windsurf (rules-file) -> .windsurfrules"
- [ ] T011 [US3] Verify manually — run in a temp directory, check `.windsurfrules` content, confirm no `hooks.json` created

---

## Phase 6: User Story 4 — Codex CLI Instructions (Priority: P3)

**Goal**: Convert Codex CLI from hook JSON (`.codex/hooks.json`) to `AGENTS.md` instructions section.

**Independent Test**: `cd /tmp/rgt-test && cargo run -- init -g --agent codex`. Verify `AGENTS.md` has RGT section and no `.codex/hooks.json` with `rgt_hook` key created.

### Implementation

- [ ] T012 [US4] Replace Codex CLI hook logic in `src/hooks/installer.rs` (lines 71-89) — remove hook JSON, create or append to `AGENTS.md` with an "## RGT Integration" section containing `rgt` usage instructions
- [ ] T013 [US4] Update output message in `src/hooks/installer.rs` — change configured message from "Codex CLI -> ~/.codex/hooks.json" to "Codex CLI (rules-file) -> AGENTS.md"
- [ ] T014 [US4] Verify manually — run in a temp directory, check `AGENTS.md` content, confirm no hooks JSON created

---

## Phase 7: Integration Tests & Polish

**Purpose**: Update test assertions, run full verification suite.

- [ ] T015 Update `test_claude_code_hook_config` in `tests/integration/test_hooks_installer.rs` — assert file is `settings.json` with nested schema, assert `hooks.json` does not exist
- [ ] T016 [P] Update `test_cursor_hook_config` in `tests/integration/test_hooks_installer.rs` — assert `"version": 1`, camelCase event names, `"matcher": "Shell"`
- [ ] T017 [P] Update `test_windsurf_hook_config` in `tests/integration/test_hooks_installer.rs` — assert `.windsurfrules` created with text content, assert no hooks JSON
- [ ] T018 [P] Update `test_codex_hook_config` in `tests/integration/test_hooks_installer.rs` — assert `AGENTS.md` created/appended with RGT section, assert no hooks JSON
- [ ] T019 Update `--agent` help text in `src/cli/init.rs` — add tier classification to each agent: "claude-code (full hook)", "cursor (full hook)", "windsurf (rules-file)", "codex (rules-file)"
- [ ] T020 Run `cargo fmt --all -- --check` to verify formatting
- [ ] T021 Run `cargo clippy --all-targets` to verify no errors
- [ ] T022 Run `cargo test test_hooks_installer -- --test-threads=1` to verify all hook tests pass
- [ ] T023 Run `cargo test --all -- --test-threads=1` to verify no regressions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **Foundational (Phase 2)**: Empty
- **US1 Claude Code (Phase 3)**: No dependencies — start immediately
- **US2 Cursor (Phase 4)**: Depends on Phase 3 setup being done, but independent logically
- **US3 Windsurf (Phase 5)**: Independent — different section of same file
- **US4 Codex (Phase 6)**: Independent — different section of same file
- **Polish (Phase 7)**: Depends on all user stories

### User Story Dependencies

All 4 stories modify `src/hooks/installer.rs` — implement sequentially to avoid merge conflicts. Order by priority: US1 → US2 → US3 → US4.

### Parallel Opportunities

- **Phase 1**: T001 and T002 run in parallel (different files)
- **Phase 7**: T015-T018 run in parallel (4 test functions in the same file — can be updated as a single edit)
- **Phase 7 cross**: T019 (init.rs help text) runs in parallel with T015-T018

---

## Implementation Strategy

### MVP First (Claude Code + Cursor)

1. Complete Phase 1: Setup
2. Complete Phase 3: US1 — Claude Code (T003-T006) 
3. Complete Phase 4: US2 — Cursor (T007-T008)
4. **STOP and VALIDATE**: Both primary agents produce correct hook configs
5. Update tests for US1/US2 (partial T015-T016)

### Incremental Delivery

1. Setup → verify current state
2. US1 → Claude Code hook fixed (MVP!)
3. US2 → Cursor hook fixed
4. US3 → Windsurf converted to rules-file
5. US4 → Codex converted to instructions
6. Tests + polish → all 4 agents verified

### Single Developer Strategy

All 4 stories modify `src/hooks/installer.rs`. Implement in sequence (US1 → US2 → US3 → US4), each building on the previous:

1. US1 (Claude Code): most complex — file change + schema change + merge logic
2. US2 (Cursor): schema change only
3. US3 (Windsurf): replace hooks with text file
4. US4 (Codex): replace hooks with AGENTS.md section
5. Update tests and help text
6. Full verification

---

## Notes

- All 4 stories touch `src/hooks/installer.rs` — must implement sequentially, not in parallel
- Claude Code merge logic (T004) requires `serde_json` to read/write/modify existing settings
- The `--force` flag behavior is already handled by the installer framework — T005 only adds cleanup of old file
- RTK's approach confirmed Windsurf and Codex are rules-file agents — no hooks API exists
- Output messages should include tier classification per the contracts
