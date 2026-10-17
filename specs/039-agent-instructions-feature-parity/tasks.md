# Tasks: Agent Instructions Feature Parity

**Input**: Design documents from `/specs/039-agent-instructions-feature-parity/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/glue-parity-contract.md, quickstart.md

**Tests**: Included — the spec requires contract coverage of the updated glue text (Constitution IX).

**Organization**: Single user story; text-only edits across two source files + CONTRIBUTING.md.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the stale glue text and baseline.

- [ ] T001 Confirm baseline: run `cargo test --test test_hook_installer_contract` green; confirm the 5 glue constants + `codex_section()` + Windsurf base lack `doctor`/`gc`/`number-format`; confirm the repo's `AGENTS.md` IS current (the canonical target); confirm no test asserts the absence of these terms

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The parity assertions every edit is validated against.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T002 Add parity assertions to `tests/contract/hook_installer.rs`: for cline, antigravity, kilocode, vibe, copilot (CLI instructions), codex (AGENTS.md), and windsurf, assert the written artifact contains `doctor`, `gc`, and `number-format` (extend the existing harness; keep existing positive assertions untouched)

**Checkpoint**: The new tests fail against current (stale) glue text.

---

## Phase 3: User Story 1 - Installed Agent Instructions Describe the Full Toolset (Priority: P1) 🎯 MVP

**Goal**: All 7 glue artifacts teach `doctor`/`gc`/`--number-format`; CONTRIBUTING.md is current.

**Independent Test**: `cargo test --test test_hook_installer_contract` (new parity assertions pass).

### Tests for User Story 1

> **NOTE: T002 is the test; it must fail before the edits and pass after.**

### Implementation for User Story 1

- [ ] T003 [US1] Update `CLINE_RULES` in `src/hooks/glue/mod.rs`: add `--number-format` to the record line and `rgt gc [--vacuum]` + `rgt doctor` bullets under Inspecting the Graph (structured style)
- [ ] T004 [US1] Update `ANTIGRAVITY_RULES` and `KILO_RULES` in `src/hooks/glue/mod.rs` (same additions, `##` style)
- [ ] T005 [US1] Update `COPILOT_CLI_RULES` in `src/hooks/glue/mod.rs` (same additions, `###` style)
- [ ] T006 [US1] Update `VIBE_PROMPT` in `src/hooks/glue/mod.rs`: add `--number-format` to the record mention and `doctor`/`gc` sentences (compact style)
- [ ] T007 [US1] Update `codex_section()` in `src/hooks/installer.rs` (structured style additions)
- [ ] T008 [US1] Update the Windsurf base string in `src/hooks/installer.rs` (`write_windsurf`): add `--number-format` to the record mention and `doctor`/`gc` sentences (compact style)
- [ ] T009 [US1] Update `CONTRIBUTING.md`: list `doctor.rs`/`gc.rs` (and `query.rs` if absent) in the `src/cli/` tree; update test examples to reference the real current test files

**Checkpoint**: `cargo test --test test_hook_installer_contract` green; new parity assertions pass.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T010 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; confirm the repo's committed `AGENTS.md` stays consistent with the updated `codex_section()` output and the `with_instruction` non-text-source block is byte-identical before/after

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

- T003–T008 touch two source files; T009 touches CONTRIBUTING.md — can run in parallel after T002.

### Parallel Opportunities

- T003/T004/T005/T006 (glue/mod.rs), T007/T008 (installer.rs), T009 (CONTRIBUTING.md) — file-distinct; glue edits are sequential within the same file to avoid conflicts.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002)
3. Phase 3: US1 (T003–T009)
4. **STOP and VALIDATE**: hook_installer_contract green

### Incremental Delivery

1. Foundational → parity assertions
2. US1 → glue constants + codex + windsurf + CONTRIBUTING updated
3. Polish → fmt/clippy/full test + wrapper-unchanged check

---

## Notes

- Text-only change; no installer logic. Users must re-run `rgt init` (or `--force`) to pick up corrected text.
- Preserve each artifact's heading style (`###` vs `##` vs prose).
- The `with_instruction` non-text-source block must remain byte-identical.
