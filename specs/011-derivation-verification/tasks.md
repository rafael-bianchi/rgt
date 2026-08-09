# Tasks: Derivation Verification

**Input**: Design documents from `/specs/011-derivation-verification/`

**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Required per spec — unit tests for each operation type, CLI contract tests for exit codes and stderr.

**Organization**: Tasks grouped by user story. US1 (EXPRESSION) is the foundation — everything else builds on it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths

## Phase 1: Setup

**Purpose**: Add dependency, initialize module structure.

- [x] T001 Add `evalexpr` to `Cargo.toml` — `evalexpr = "11"` under `[dependencies]`
- [x] T002 [P] Create `src/verify/` directory with empty module files: `src/verify/mod.rs`, `src/verify/expression.rs`, `src/verify/date_diff.rs`

---

## Phase 2: Foundational — Verification Module

**Purpose**: Implement the shared verification functions. These are called by the CLI subcommand but are independent of CLI parsing.

- [x] T003 Implement `verify_expression` in `src/verify/expression.rs` — accepts `parents: &[ValueData]`, `expression: &str`, `result: f64`, returns `Result<(), String>`. Binds parent values to variables a,b,c..., evaluates with `evalexpr::eval_with_context`, compares with float tolerance (rel < 1e-12 or abs < 1e-9)
- [x] T004 [P] Implement `verify_date_diff` in `src/verify/date_diff.rs` — accepts `parents: &[ValueData]`, `result: i64`, returns `Result<(), String>`. Extracts two DateTime values, calls `date_diff()`, compares `duration_seconds`
- [x] T005 Implement `verify` dispatch in `src/verify/mod.rs` — matches `operation_type` to "EXPRESSION", "DATE_DIFF", or unknown. Calls the appropriate function. Returns `Ok(())` for match/unverifiable, `Err(msg)` for mismatch
- [x] T006 Remove `#[allow(dead_code)]` from `date_diff()` in `src/types/value.rs` — make it `pub` and usable by the verify module
- [x] T007 Register `verify` module in `src/lib.rs` — add `pub mod verify;` and re-export

---

## Phase 3: User Story 1 — Expression Verification (Priority: P1) 🎯 MVP

**Goal**: `rgt verify --parents a,b --operation EXPRESSION --expression "a + b" --result 300` works. Exit 0 on match, 1 on mismatch, 2 on invalid input.

**Independent Test**: `cargo run -- verify --parents n1,n2 --operation EXPRESSION --expression "a + b" --result <correct>`. Exit 0. Run with wrong result, exit 1.

### Implementation

- [x] T008 [US1] Create `src/cli/verify.rs` — implement `execute_verify` function that parses `--parents`, `--operation`, `--expression`, `--result` from clap args, opens DB, loads parent values, calls `verify::verify()`, prints errors to stderr, exits with correct code (0/1/2)
- [x] T009 [US1] Add `Verify` variant to `Commands` enum in `src/cli/mod.rs` — register the verify subcommand with clap derive
- [x] T010 [US1] Add verify subcommand to `main.rs` match block — dispatch to `cli::verify::execute_verify()`
- [x] T011 [US1] Manual smoke test — `cargo run -- verify --parents <real_ids> --operation EXPRESSION --expression "a + b" --result <val>`, verify exit codes

**Checkpoint**: `rgt verify --operation EXPRESSION` works end-to-end.

---

## Phase 4: User Story 2 — Date Diff Verification (Priority: P1)

**Goal**: `rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000` works. The CLI reuses the same subcommand; only the operation type and computation differ.

**Independent Test**: Record two Date nodes, run `rgt verify --operation DATE_DIFF` with correct `--result`. Exit 0. Run with wrong result, exit 1.

### Implementation

- [x] T012 [US2] Manual smoke test for DATE_DIFF — record two Date nodes, verify correct and incorrect durations
- [x] T013 [US2] Verify `date_diff()` computes correct values — run existing `test_date_diff` tests

**Checkpoint**: `rgt verify --operation DATE_DIFF` works end-to-end.

---

## Phase 5: User Story 3 — Unknown Operations (Priority: P2)

**Goal**: `rgt verify --operation CUSTOM --result 42` exits 0 with warning on stderr. No blocking of derivations for unknown ops.

**Independent Test**: Run `rgt verify --parents n1 --operation UNKNOWN --result 42`. Exit 0. Stderr contains warning.

### Implementation

- [x] T014 [US3] Verify unknown operation behavior — the `verify::verify()` dispatch in T005 already handles this. Run smoke test to confirm pass-through works
- [x] T015 [US3] Manual smoke test — `rgt verify --parents <real_id> --operation CUSTOM --result 42`, verify exit 0 + stderr warning

**Checkpoint**: Unknown operations pass through without blocking.

---

## Phase 6: Tests & Polish

**Purpose**: Unit tests, CLI contract tests, full verification.

- [x] T016 [P] Write unit tests in `tests/unit/test_verification.rs` — test `verify_expression` (match, mismatch, float tolerance, division by zero, unknown variable), test `verify_date_diff` (match, mismatch, negative, reverse order)
- [x] T017 [P] Write CLI contract tests in `tests/contract/test_cli_verify.rs` — test exit codes (0=pass, 1=mismatch, 2=invalid), stderr content for each case, unknown operation warning
- [x] T018 Run `cargo fmt --all -- --check` to verify formatting
- [x] T019 Run `cargo clippy --all-targets` to verify no errors
- [x] T020 Run `cargo test --all -- --test-threads=1` to verify all tests pass
- [x] T021 [P] Update `rgt init -g` output message in `src/hooks/installer.rs` to document the variable mapping convention (`parent[0]=a, parent[1]=b, ...`) and supported operations (`EXPRESSION`, `DATE_DIFF`) so the LLM discovers them when hooks are installed (FR-010, SC-006)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — add dependency + create files
- **Foundational (Phase 2)**: Depends on setup (evalexpr available)
- **US1 EXPRESSION (Phase 3)**: Depends on foundational (verify module exists)
- **US2 DATE_DIFF (Phase 4)**: Depends on foundational + US1 (CLI exists, just smoke test)
- **US3 Unknown (Phase 5)**: Depends on foundational (dispatch already handles this)
- **Polish (Phase 6)**: Depends on all user stories

### Parallel Opportunities

- **Phase 1**: T001 and T002 run in parallel
- **Phase 2**: T003 and T004 run in parallel (different files); T005 depends on both
- **Phase 6**: T016, T017, and T021 run in parallel (different files)

---

## Implementation Strategy

### MVP First (US1 — EXPRESSION Only)

1. Complete Phase 1 (dependency + structure)
2. Complete Phase 2 (verify module)
3. Complete Phase 3 (CLI subcommand)
4. **STOP and VALIDATE**: `rgt verify --operation EXPRESSION` works
5. Feature is usable — expressions are verified

### Incremental Delivery

1. Setup → dependencies ready
2. Foundational → verify module functional (unit testable)
3. US1 → CLI works for EXPRESSION (MVP!)
4. US2 → CLI works for DATE_DIFF
5. US3 → Unknown ops pass-through verified
6. Polish → full test suite green

---

## Notes

- US1 (EXPRESSION) and US2 (DATE_DIFF) share the same CLI entry point — only the operation type and verification function differ
- T003-T005 are the core logic and can be tested independently of the CLI via unit tests
- The `date_diff()` removal of `#[allow(dead_code)]` (T006) may surface clippy warnings if no other callers exist — resolve during T019
- Hook scripts are thin delegates per Constitution VIII — hooks call `rgt verify` directly. The `rgt init -g` output documents the variable mapping convention (T021) so the LLM discovers it at hook install time.
