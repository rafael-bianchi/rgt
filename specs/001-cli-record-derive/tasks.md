# Tasks: CLI Record & Derive Commands

**Input**: Design documents from `specs/001-cli-record-derive/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Constitution Principle IX mandates test coverage for new behavior. Test tasks are included and marked with ⚠️.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Verify prerequisites — no new project scaffolding needed (existing single-binary Rust project)

- [x] T001 Verify `cargo build` passes on clean develop branch
- [x] T002 Verify `cargo test` passes with no regressions

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Wire new CLI subcommands into the binary before implementing their logic. This MUST complete before US1 or US2 can begin.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T003 Register `Record` and `Derive` variants in the `Commands` enum in `src/main.rs`
- [x] T004 Add `Record` and `Derive` dispatch arms in the `match cli.command` block in `src/main.rs`
- [x] T005 [P] Add `pub mod record; pub mod derive; pub use record::*; pub use derive::*;` to `src/cli/mod.rs`
- [x] T006 [P] Create stub files `src/cli/record.rs` and `src/cli/derive.rs` with placeholder handler functions (return `Ok(())`)

**Checkpoint**: `cargo build` compiles with new subcommands registered. `rgt record` and `rgt derive` appear in `rgt --help`.

---

## Phase 3: User Story 1 - Record Root Values from Data Files (Priority: P1) 🎯 MVP

**Goal**: Implement `rgt record <FILE>` and `rgt record --stdin <FILE>` to extract numeric/date values from files and upsert root nodes into the provenance graph.

**Independent Test**: Create a CSV file with numeric values, run `rgt record <file>`, verify with `rgt status` that root nodes appear with correct values and line numbers, and verify stdout prints `Recorded <N> values from <file>`.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T007 [P] [US1] Unit tests for value limit enforcement and empty file handling in `tests/unit/test_record.rs`
- [x] T008 [P] [US1] Contract test for record command exit codes and stdout format in `tests/contract/test_cli_record_derive_contract.rs` (record section only)

### Implementation for User Story 1

- [x] T009 [US1] Implement file reading and stdin handling logic in `src/cli/record.rs`
- [x] T010 [US1] Implement extraction loop: call `hooks::parser::extract_values_from_content()` to get values from file content
- [x] T011 [US1] Implement 10,000 value soft limit: break loop at 10,000, print warning to stderr
- [x] T012 [US1] Implement node recording: call `upsert_source_document()`, `get_metadata_snapshot()`, `compute_blake3_hash()` for source tracking
- [x] T013 [US1] Implement node upsert: for each extracted value, call `TrackedNode::generate_root_id()` then `insert_tracked_node()`
- [x] T014 [US1] Implement stdout summary: print `Recorded <N> values from <file>` on success
- [x] T015 [US1] Implement error handling: file not found → exit 2 with `Error: file not found: <file>`
- [x] T016 [US1] Run `cargo test test_record test_cli_record_derive_contract` and verify all US1 tests pass

**Checkpoint**: `rgt record budget.csv` creates root nodes. `rgt record --stdin` works. Exit codes correct. Tests pass.

---

## Phase 4: User Story 2 - Verify-Then-Record Derived Values (Priority: P1)

**Goal**: Implement `rgt derive --parents <ids> --operation <op> [--expression <expr>] --result <val>` to verify a computation against parent node values and only insert derived nodes + edges if verification succeeds.

**Independent Test**: Record two root nodes, run `rgt derive` with a correct expression and result, verify the derived node appears in `rgt status`. Then attempt `rgt derive` with a wrong result, verify the derivation is rejected.

### Tests for User Story 2 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T017 [P] [US2] Contract test for derive command exit codes (0/1/2) and output format in `tests/contract/test_cli_record_derive_contract.rs` (derive section only)
- [x] T018 [US2] Integration test for full record→derive→staleness workflow in `tests/integration/test_cli_record_derive_integration.rs`

### Implementation for User Story 2

- [x] T019 [US2] Implement parent node lookup: parse `--parents` comma-separated list, call `get_tracked_node()` for each, exit 2 if any not found
- [x] T020 [US2] Implement verification call: call `verify::verify(&parent_values, operation, expression, result)`
- [x] T021 [US2] Implement rejection: on verification failure, print error message and exit 1 (no nodes/edges inserted)
- [x] T022 [US2] Implement invalid input rejection: wrong parent count for DATE_DIFF, missing expression for EXPRESSION → exit 2
- [x] T023 [US2] Implement derived node creation: call `TrackedNode::generate_derived_id()` then `insert_tracked_node()` with `NodeType::Derived`
- [x] T024 [US2] Implement edge insertion: for each parent, call `insert_derivation_edge()`
- [x] T025 [US2] Implement stdout output: print `Derived node: <node_id>` on success
- [x] T026 [US2] Run `cargo test test_cli_record_derive_contract test_cli_record_derive_integration` and verify all US2 tests pass

**Checkpoint**: `rgt derive` verifies before recording. Wrong results rejected. Exit codes correct. Tests pass.

---

## Phase 5: User Story 3 - Agent Integration via Updated Rules Files (Priority: P2)

**Goal**: Update AGENTS.md template in the hook installer so `rgt init --agent codex` includes instructions for `rgt record` and `rgt derive`.

**Independent Test**: Run `rgt init --agent codex --force` in a fresh project, verify AGENTS.md contains instructions for `rgt record` and `rgt derive`.

### Implementation for User Story 3

- [x] T027 [US3] Extend the `codex_section` and windsurf rules template in `src/hooks/installer.rs` with `rgt record` usage instructions and `rgt derive` usage instructions with example expression syntax
- [x] T028 [US3] Verify AGENTS.md template renders correctly: `rgt init --agent codex --force` then `grep "rgt record" AGENTS.md` and `grep "rgt derive" AGENTS.md`
- [x] T029 [US3] Run existing `test_hooks_installer` test to confirm no regressions in other agent integrations

**Checkpoint**: AGENTS.md includes record/derive instructions. Existing hook installer tests still pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Validation, documentation, and cleanup across all user stories

- [x] T030 [P] Run `cargo fmt --all -- --check` and fix any formatting issues
- [x] T031 [P] Run `cargo clippy --all-targets` and fix any warnings
- [x] T032 Run full test suite: `cargo test` — all existing and new tests pass
- [x] T033 Run quickstart.md validation scenarios end-to-end (see `specs/001-cli-record-derive/quickstart.md`)
- [x] T034 [P] Verify `rgt --help` lists `record` and `derive` subcommands with descriptions
- [x] T035 Verify README.md covers `rgt record` and `rgt derive` commands (Constitution X — documentation matches behavior)
- [x] T036 [P] Add timing assertion to `tests/unit/test_record.rs` verifying a 10,000-value file records in under 1 second

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational (Phase 2) — T003-T006 must complete first
- **US2 (Phase 4)**: Depends on Foundational (Phase 2) — T003-T006 must complete first. Independent of US1 (can use root nodes from `rgt hook post` for testing). However, integration test T018 requires US1 completed to set up root nodes.
- **US3 (Phase 5)**: Depends on US1 and US2 being implemented (needs to document existing commands)
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1)**: Independent after Foundational. No dependencies on other stories.
- **US2 (P1)**: Independent after Foundational for implementation. Integration test (T018) depends on US1 for root node setup.
- **US3 (P2)**: Depends on US1 and US2 completion — documents their usage.

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Core implementation before error handling
- Error handling before final test pass verification
- Story complete before moving to next priority

### Parallel Opportunities

- T007 and T008 (US1 tests) can run in parallel
- T017 can start in parallel with US1 development; T018 (integration) requires US1 complete
- US1 and US2 implementation can proceed in parallel after Phase 2 (if team capacity allows)
- T005 and T006 (cli mod.rs + stubs) can run in parallel
- T030 and T031 (fmt + clippy) can run in parallel
- T034 and T035 (help output + README) can run in parallel

---

## Parallel Example: Phase 2 Foundational

```bash
# Run these together:
Task: T005 "Add pub mod record; pub mod derive to src/cli/mod.rs"
Task: T006 "Create stub files src/cli/record.rs and src/cli/derive.rs"
```

---

## Parallel Example: User Story 1 Tests

```bash
# Run these together:
Task: T007 "Unit tests for value limit enforcement in tests/unit/test_record.rs"
Task: T008 "Contract test for record command in tests/contract/test_cli_record_derive_contract.rs"
```

---

## Parallel Example: User Story 1 + 2 (after Foundational)

```bash
# Developer A: User Story 1
Tasks: T007-T016

# Developer B: User Story 2
Tasks: T017 starts immediately; T018-T026 after US1 implementation provides seed data
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: Foundational (T003-T006)
3. Complete Phase 3: User Story 1 (T007-T016)
4. **STOP and VALIDATE**: Run `rgt record` on real files, verify nodes in `rgt status`
5. Deploy/demo if ready — `rgt record` alone enables rules-file agents to participate in provenance tracking

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (`rgt record`) → Test independently → MVP ready
3. Add US2 (`rgt derive`) → Test independently → Full derivation workflow
4. Add US3 (AGENTS.md) → Test independently → Agent instructions complete
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: US1 — `rgt record` (core recording)
   - Developer B: US2 — `rgt derive` (derivation verification)
3. Developer A or B: US3 — AGENTS.md (after both commands exist)
4. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Tests are required by Constitution Principle IX — write them first
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence

---

## Phase 7: Convergence

**Purpose**: Address gaps found between spec/plan/tasks and current codebase state.

- [x] T037 Run quickstart.md validation scenarios end-to-end to verify SC-004 (full agent workflow without hook interception) — `specs/001-cli-record-derive/quickstart.md` (missing)
- [x] T038 Add timing assertion to `tests/unit/test_record.rs` verifying a 10,000-value file records in under 1 second per SC-001 (missing)
- [x] T039 Review `mod verify;` in `src/main.rs` — assess whether this is the correct approach for resolving `crate::verify` imports from `cli/derive.rs` or if an alternative import path should be used instead (unrequested addition not in plan)
