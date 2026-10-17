# Tasks: Derive & Verify Hardening

**Input**: Design documents from `/specs/021-derive-verify-hardening/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed user-facing behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/verify/{expression,mod,date_diff}.rs`, `src/cli/derive.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates for this feature)

- [x] T001 Confirm `evalexpr = "13.1.0"` is already present in `[dependencies]` in `Cargo.toml` (no new dependencies are required — see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared guards every user story depends on — expression validation (len/depth) and the operation allow-list predicate

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `src/verify/expression.rs` `#[cfg(test)]` for `validate_expression`: (a) over `MAX_EXPRESSION_LEN` → `Err` naming the length limit; (b) nesting deeper than `MAX_EXPRESSION_DEPTH` (e.g. 300 parens) → `Err` naming the depth limit; (c) exactly at the depth limit → `Ok`; (d) empty/whitespace-only → `Err`
- [x] T003 [P] Unit tests in `src/verify/mod.rs` `#[cfg(test)]` for `valid_operation`: `"EXPRESSION"`/`"DATE_DIFF"` → `true`; `"expression"`/`"Expression"`/`"date_diff"`/`""`/`"RANDOM"` → `false`

### Implementation for Foundational Primitives

- [x] T004 Implement `validate_expression(expr: &str) -> Result<(), String>` plus `MAX_EXPRESSION_LEN` (4096) and `MAX_EXPRESSION_DEPTH` (256) in `src/verify/expression.rs`: reject over-length and over-depth input with a clean error naming the limit, before any evalexpr parse/eval
- [x] T005 Implement `pub fn valid_operation(op: &str) -> bool` in `src/verify/mod.rs` (case-sensitive allow-list `EXPRESSION` | `DATE_DIFF`)

**Checkpoint**: foundational tests pass; both helpers compile

---

## Phase 3: User Story 1 - A Malformed `--expression` Can Never Crash the CLI (Priority: P1) 🎯 MVP

**Goal**: An over-limit `--expression` is rejected with a clean error and a normal non-zero exit — never a stack overflow / SIGABRT (FR-001)

**Independent Test**: Unit tests feed a deep (e.g. 1000-nested) and over-length expression to `verify_expression` and assert a clean `Err` (no panic); a release-mode CLI run exits non-zero without SIGABRT (quickstart manual scenario 1)

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T006 [US1] Unit tests in `src/verify/expression.rs`: `verify_expression` with a 1000-deep nested expression returns `Err` containing "depth"; a 5000-byte expression returns `Err` containing "length"; a valid in-limit expression still verifies

### Implementation for User Story 1

- [x] T007 [US1] Call `validate_expression(expression)?` at the top of `verify_expression` in `src/verify/expression.rs` (defense-in-depth, returns a clean `Err`), AND enforce it at the CLI entry points so it exits 2: in `run_verify_cli` (`src/verify/mod.rs`) and `execute_derive` (`src/cli/derive.rs`), when `operation == "EXPRESSION"` and an `--expression` is provided, validate it immediately after operation validation and exit 2 on a limit violation (matching `contracts/verify-behavior.md`'s exit table)

**Checkpoint**: US1 tests pass; over-limit expressions return clean errors

---

## Phase 4: User Story 2 - Verification Can Never Be Silently Bypassed (Priority: P1)

**Goal**: Any `--operation` outside `{EXPRESSION, DATE_DIFF}` (case variants included) is invalid input → exit 2 with the valid list; `rgt derive` records nothing (FR-002)

**Independent Test**: Unit test that `verify()` returns `Err` for an unknown operation; contract test that `execute_derive`/`run_verify_cli` reject before DB access (exit 2)

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T008 [US2] Unit tests in `src/verify/mod.rs`: `verify(&[], "expression", None, 0.0)` → `Err` mentioning the operation; `verify(&[], "EXPRESSION", Some("1"), 1.0)` → not an unknown-operation error. Plus a contract test in `tests/contract/test_cli_record_derive_contract.rs` asserting `execute_derive` with an unknown operation exits 2 (via a spawned `rgt derive --operation bogus` subprocess against an empty temp `.rgt` dir, asserting exit code 2 and no node recorded)

### Implementation for User Story 2

- [x] T009 [US2] In `src/verify/mod.rs`: change `verify()`'s unknown-operation arm from `_ => Ok(())` to `_ => Err("unknown operation ... valid operations: EXPRESSION, DATE_DIFF")`; in `run_verify_cli`, validate `operation` via `valid_operation` at the very top (before DB open) and exit 2 with the valid list
- [x] T010 [US2] In `src/cli/derive.rs`: validate `operation` via `valid_operation` at the top (exit 2 + valid list) before any DB/parent work; remove the `_ => warning + Ok` arm; ensure no derived node or edges are recorded on rejection

**Checkpoint**: US2 tests pass; unknown operations exit 2 and never record

---

## Phase 5: User Story 3 - Verification Results Are Trustworthy and Deterministic (Priority: P1)

**Goal**: Integer-typed results verify (`as_number`), and evalexpr builtins are disabled so evaluation is deterministic (FR-003/FR-004)

**Independent Test**: Unit tests assert `"2*50" → 100` verifies; `"random()"`/`"str::len(...)"` are rejected; same expression+parents evaluate identically twice

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T011 [P] [US3] Unit tests in `src/verify/expression.rs`: (a) `verify_expression(&[], "2*50", 100.0)` → `Ok`; (b) `"random()"` → `Err`; (c) `"str::len(\"a\")"` → `Err`; (d) `"if(a > b, 1, 2)"` → `Err` (builtin `if` disabled); (e) evaluating the same expression+parents twice yields identical results

### Implementation for User Story 3

- [x] T012 [US3] In `verify_expression` (`src/verify/expression.rs`): change `computed.as_float()` to `computed.as_number()`; call `context.set_builtin_functions_disabled(true).map_err(...)?` after building the `HashMapContext`

**Checkpoint**: US3 tests pass; integer results verify, builtins are rejected

---

## Phase 6: User Story 4 - Parent-Count and Date-Ordering Edge Cases Error Cleanly (Priority: P2)

**Goal**: >26 parents error cleanly (no panic/wrap); DATE_DIFF rejects negative (swapped-order) durations (FR-005/FR-006)

**Independent Test**: Unit tests assert 27 parents → `Err`, 26 → `Ok`; a negative DATE_DIFF → `Err` naming the convention

### Tests for User Story 4 (write FIRST, expect FAIL) ⚠️

- [x] T013 [P] [US4] Unit tests: in `src/verify/expression.rs`, 27 parents → `Err` containing "at most 26"; 26 parents → `Ok`. In `src/verify/date_diff.rs`, flip `test_date_diff_negative` to expect `Err` naming the `date2 - date1` convention, and keep the positive/mismatch tests

### Implementation for User Story 4

- [x] T014 [US4] In `verify_expression` (`src/verify/expression.rs`): add a `parents.len() > 26` guard returning "at most 26 parents supported (variables a–z)"; replace `((b'a' + i as u8) as char)` with `b"abcdefghijklmnopqrstuvwxyz"[i]`. In `verify_date_diff` (`src/verify/date_diff.rs`): reject a negative computed duration with a convention-naming error

**Checkpoint**: US4 tests pass; no panic/wrap, negative DATE_DIFF rejected

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation

- [x] T015 [P] Update `README.md` and any `--help`-adjacent docs for the operation allow-list (exit 2), expression limits (4096/256), 26-parent cap, and DATE_DIFF convention enforcement
- [x] T016 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T017 Run the manual scenarios in `specs/021-derive-verify-hardening/quickstart.md` (crash regression, casing, integer result, determinism)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational (T004 `validate_expression`)
- **US2 (Phase 4)**: Depends on Foundational (T005 `valid_operation`) — independent of US1
- **US3 (Phase 5)**: Depends on Foundational — edits `expression.rs` (shares the file with US1/US4, so run sequentially after US1)
- **US4 (Phase 6)**: Depends on Foundational — edits `expression.rs` + `date_diff.rs`
- **Polish (Phase 7)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: after Foundational — crash guard
- **US2 (P1)**: after Foundational — operation allow-list (different files from US1)
- **US3 (P1)**: after Foundational — result correctness (shares `expression.rs` with US1)
- **US4 (P2)**: after Foundational — parent cap + DATE_DIFF sign

### Within Each User Story

- Tests written first, must FAIL before implementation
- Foundational helpers before wiring (US1/US2); coercion/builtins (US3) and caps/sign (US4) are additive edits to the same functions
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 ∥ T003 (Foundational tests, different files)
- T004 ∥ T005 (independent files)
- T011 ∥ T013 (US3/US4 tests, different files)
- T015 ∥ T016 (Polish, different concerns)

---

## Parallel Example: Foundational Phase

```bash
# Tests first (expect FAIL):
Task: "validate_expression tests (src/verify/expression.rs)"
Task: "valid_operation tests (src/verify/mod.rs)"

# Implementation (independent files):
Task: "validate_expression + MAX_* constants (src/verify/expression.rs)"
Task: "valid_operation (src/verify/mod.rs)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (no-op) + Phase 2 (Foundational helpers)
2. Phase 3: User Story 1 (no crash on over-limit expressions)
3. **STOP and VALIDATE**: over-limit expressions return clean errors (the CRITICAL is fixed)
4. Then US2/US3/US4 incrementally

### Incremental Delivery

1. Foundational → shared guards
2. US1 → crash fixed (MVP, the CRITICAL)
3. US2 → no silent verification bypass
4. US3 → trustworthy/deterministic results
5. US4 → parent cap + DATE_DIFF sign
6. Polish → docs + full-suite validation

---

## Notes

- [P] tasks = different files, no dependencies; US1/US3/US4 all edit `src/verify/expression.rs`, so their implementation tasks are sequential within the file
- [Story] label maps task to user story for traceability
- The stack-overflow regression is a clean-`Err` test (no in-process process-abort test); the release-binary no-SIGABRT behavior is covered by quickstart manual scenario 1
- No `main.rs`/CLI-flag changes — operation validation is in-code
- Commit after each task or logical group; stop at each checkpoint

---

## Phase 8: Convergence

- [x] T018 Add CLI-layer pre-checks in `run_verify_cli` (`src/verify/mod.rs`) and `execute_derive` (`src/cli/derive.rs`) so missing `--expression` for `EXPRESSION` and >26 parents exit 2 (invalid input) rather than flowing through `verify()`'s mismatch path as exit 1, per `contracts/verify-behavior.md` exit table / FR-005 (partial)
