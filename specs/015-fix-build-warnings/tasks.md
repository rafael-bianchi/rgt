# Tasks: Eliminate Build Warnings & Cargo Suggestions

**Input**: Design documents from `specs/015-fix-build-warnings/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: No new test files. The cleanup is refactor-only; verification is via deterministic warning-grep checks (quickstart scenarios) plus the existing 108-test suite which must stay green.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Establish the baseline warning inventory before changes

- [x] T001 Capture baseline: run `cargo build --all-targets 2>&1 | grep warning`, `cargo clippy --all-targets 2>&1 | grep warning`, and `cargo test --no-run 2>&1 | grep warning`; record the counts and confirm they match the inventory in `specs/015-fix-build-warnings/data-model.md` (W1-W9)
- [x] T002 Confirm baseline tests pass: `cargo test` reports 108 passing / 0 failed

---

## Phase 2: User Story 1 - Warning-Free Build for All Targets (Priority: P1) 🎯 MVP

**Goal**: Eliminate all rustc warnings (unused imports + dead code) across lib, bin, and test targets.

**Independent Test**: `cargo build --all-targets 2>&1 | grep warning` produces zero output; `cargo test --no-run 2>&1 | grep warning` produces zero output.

### Tests for User Story 1 (verification, no new test files) ⚠️

> **NOTE**: These are verification greps, not new test files. Run after each fix batch.

- [x] T003 [US1] Verify zero rustc warnings: `cargo build --all-targets 2>&1 | grep warning` returns no output
- [x] T004 [US1] Verify zero test-compile warnings: `cargo test --no-run 2>&1 | grep warning` returns no output

### Implementation for User Story 1

- [x] T005 [US1] Remove unused glob re-export `pub use engine::*;` in `src/graph/mod.rs:4` — first verify no caller relies on the short path (grep for `graph::GraphEngine` or bare `GraphEngine` usage), then delete the line
- [x] T006 [P] [US1] Remove unused glob re-exports `pub use queries::*;` and `pub use schema::*;` in `src/store/mod.rs:6-7` — first verify callers use canonical paths (grep for `DbStore`, `upsert_source_document`, `insert_tracked_node` imports), then delete both lines
- [x] T007 [US1] Delete dead function `list_stale_nodes_json` from `src/query/mod.rs:72` (MCP leftover; confirmed unreferenced by grep)
- [x] T008 [US1] Fix `run_verify_cli` dead-code warning: change `src/main.rs:159` from `rgt::verify::run_verify_cli(...)` to `crate::verify::run_verify_cli(...)` so the bin-crate copy is used (keep `mod verify;` at line 9)

**Checkpoint**: `cargo build --all-targets 2>&1 | grep warning` is empty for rustc warnings (imports + dead code resolved).

---

## Phase 3: User Story 2 - Cargo Suggestions Addressed (Priority: P2)

**Goal**: Address all clippy suggestions — implement the `Default` impl and simplify idiomatic expressions in tests.

**Independent Test**: `cargo clippy --all-targets 2>&1 | grep warning` produces zero output.

### Tests for User Story 2 (verification) ⚠️

> **NOTE**: Verification grep only; no new test files.

- [x] T009 [US2] Verify zero clippy warnings: `cargo clippy --all-targets 2>&1 | grep warning` returns no output

### Implementation for User Story 2

- [x] T010 [US2] Add `impl Default for GraphEngine { fn default() -> Self { Self::new() } }` in `src/graph/engine.rs` (clippy suggestion; delegates to existing `new()`)
- [x] T011 [P] [US2] Convert single-pattern `match output { Ok(...) => ... }` to `if let` in `tests/integration/test_distribution_quickstart.rs:40` — use `if let Ok(out) = output else { ... }` preserving the exact error-handling behavior of the current `Err` arm
- [x] T012 [P] [US2] Convert single-pattern `match output { Ok(...) => ... }` to `if let` in `tests/integration/test_distribution_performance.rs:34` — use `if let Ok(out) = output else { ... }` preserving the exact error-handling behavior of the current `Err` arm
- [x] T013 [US2] Simplify `!hooks.get("rgt_hook").is_some()` to `hooks.get("rgt_hook").is_none()` in `tests/integration/test_hooks_installer.rs:95`

**Checkpoint**: `cargo clippy --all-targets 2>&1 | grep warning` is empty.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final verification of the zero-warning contract and full-suite regression

- [x] T014 Run `cargo fmt --all -- --check` and fix any formatting issues
- [x] T015 Run the full test suite `cargo test` — confirm 108 tests pass with 0 failures (no behavior change)
- [x] T016 Run all three verification greps together: `cargo build --all-targets`, `cargo clippy --all-targets`, `cargo test --no-run` — all produce zero warning lines
- [x] T017 Run quickstart.md validation scenarios end-to-end (see `specs/015-fix-build-warnings/quickstart.md`)
- [x] T018 [P] Verify `cargo clippy --all-targets 2>&1 | grep -c warning` == 0 and document the final counts in `specs/015-fix-build-warnings/research.md` (confirm SC-004: 100% disposition coverage)

**Checkpoint**: Local quality gates green; zero warnings across all targets.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — establishes baseline
- **US1 (Phase 2)**: Depends on Setup
- **US2 (Phase 3)**: Depends on Setup. **Independent of US1** for implementation (different files: engine.rs + tests vs graph/mod.rs + store/mod.rs + query/mod.rs + main.rs), but both must be complete before Polish
- **Polish (Phase 4)**: Depends on both user stories

### User Story Dependencies

- **US1 (P1)**: Independent — removes imports and dead code
- **US2 (P2)**: Independent — implements Default + test simplifications
- Note: US1 T008 (main.rs) and US2 T010 (engine.rs) touch different files; no conflicts

### Within Each User Story

- Verification greps (T003-T004, T009) guide implementation
- Fixes applied then re-verified

### Parallel Opportunities

- T005 and T006 (different files: graph/mod.rs vs store/mod.rs) can run in parallel
- T011 and T012 (different test files) can run in parallel
- US1 implementation and US2 implementation can proceed in parallel (disjoint files)
- T014 and T018 (fmt + clippy count) can run in parallel

---

## Parallel Example: US1 + US2

```bash
# Developer A: User Story 1 (rustc warnings)
Tasks: T003, T005, T006, T007, T008, T004

# Developer B: User Story 2 (clippy suggestions)
Tasks: T010, T011, T012, T013, T009
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: US1 (T003-T008)
3. **STOP and VALIDATE**: `cargo build --all-targets 2>&1 | grep warning` is empty; `cargo test` still green
4. Open PR — the clean build is the MVP

### Incremental Delivery

1. Setup → baseline confirmed
2. US1 → zero rustc warnings (imports + dead code)
3. US2 → zero clippy warnings (suggestions applied)
4. Polish → full zero-warning contract + 108 tests green

### Parallel Team Strategy

With two developers:

1. Team completes Setup together
2. Developer A: US1 — unused imports + dead code (graph/mod.rs, store/mod.rs, query/mod.rs, main.rs)
3. Developer B: US2 — clippy suggestions (engine.rs, 3 test files)
4. Both merge into one PR, then Polish phase validates the complete zero-warning contract

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- No new test files — verification is via deterministic grep checks (per Constitution IX, these are deterministic with no flaky dependencies)
- No crate-level `#![allow(...)]` attributes (per spec Assumption 4)
- No public signature or behavior changes (per SC-005)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
