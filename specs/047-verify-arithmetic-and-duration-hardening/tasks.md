# Tasks: Verify Arithmetic, Tolerance, and Duration Logic

**Input**: Feature specification `specs/047-verify-arithmetic-and-duration-hardening/spec.md` and plan `specs/047-verify-arithmetic-and-duration-hardening/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US4] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US1] Add unit tests in `src/verify/expression.rs` asserting that `verify_expression` rejects formulas where any parent variable `a..` is missing (e.g. `parents: [a, b]` with `"a + 0"` or `"42"`).
- [x] T002 [US2] Add unit tests in `src/verify/expression.rs` asserting that `approx_eq` rejects `0.0` vs `1e-10`, rejects opposite signs (`1e-10` vs `-1e-10`), and correctly verifies large and small float expressions.
- [x] T003 [US3] Add contract tests in `tests/contract/test_cli_record_derive_contract.rs` asserting that `rgt derive` records `computed_val` rather than caller's rounded claim.
- [x] T004 [US4] Add unit tests in `tests/unit/test_date_diff.rs` asserting that `ValueData::Duration::to_string_repr()` formats `86_401s` (1d 1s) and `90_005s` (1d 1h 5s) with full seconds precision.

---

## Phase 2: Implementation

- [x] T005 [US1] In `src/verify/expression.rs`, check that every bound parent variable `a..` (up to `parents.len()`) is referenced as a standalone identifier token in `expression`.
- [x] T006 [US2] In `src/verify/expression.rs`, update `approx_eq` with relative tolerance `diff <= (1e-9 * largest).max(f64::EPSILON * 8.0)` and strict sign agreement when above epsilon.
- [x] T007 [US3] In `src/verify/expression.rs` and `src/verify/mod.rs`, return the evaluated `f64` from expression verification, and in `src/cli/derive.rs` record `ValueData::Number(computed_val)`.
- [x] T008 [US4] In `src/types/value.rs`, update `to_string_repr()` for `Duration` when `days > 0` to format non-zero seconds without truncation, and update `tests/unit/test_duration_identity.rs` accordingly.

---

## Phase 3: Verification & Parity

- [x] T009 Run the complete test suite (`cargo test </dev/null`) and verify all 46+ test targets pass.
- [x] T010 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
