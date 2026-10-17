# Implementation Plan: Derive & Verify Hardening

**Branch**: `021-derive-verify-hardening` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/021-derive-verify-hardening/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Make `rgt verify` / `rgt derive` safe and trustworthy (findings §7.3):

1. **No crash (CRITICAL)**: a `--expression` over a documented length (≤ 4096) or parenthesis-nesting depth (≤ 256) is rejected with a clean error **before** `evalexpr` evaluation — the deep-nesting SIGABRT/stack-overflow (§7.3, `src/verify/expression.rs:38`) becomes a normal non-zero exit.
2. **No silent bypass (HIGH)**: `--operation` outside `{EXPRESSION, DATE_DIFF}` — including case variants — is invalid input (exit 2, valid-list message) for both commands, replacing `verify/mod.rs:59`'s `_ => Ok(())` pass-through and `cli/derive.rs`'s `_ => warning + Ok`. `rgt derive` records nothing on an unverifiable operation.
3. **Trustworthy results (MEDIUM)**: `as_float()` → `as_number()` so integer results verify (`"2*50"` → 100); `HashMapContext::set_builtin_functions_disabled(true)` so only bound variables + `+ - * / % ^` work — `random()`/`str::*`/`math::*`/`if`/regex are gone and evaluation is deterministic.
4. **Clean parent edge cases (LOW)**: >26 parents → clean error (no `u8` wrap); DATE_DIFF enforces the `date2 - date1` convention by rejecting negative computed durations (a swapped order) with a convention-naming error (clarified 2026-08-29, Option A).

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: `evalexpr` 13.1 (already in `Cargo.toml`). **No new dependencies.** Uses `HashMapContext::set_builtin_functions_disabled(true)` (Context trait, verified in the vendored source) and `Value::as_number()` (Int→Float coercion).

**Storage**: N/A — no schema/store changes.

**Testing**: `cargo test` — extend `src/verify/expression.rs` and `src/verify/date_diff.rs` unit tests; add contract tests for `run_verify_cli`/`execute_derive` exit codes (invalid operation → 2, deep/long expression → clean error) via a temp `.rgt` store; `cargo fmt`/`clippy` per constitution XI.

**Target Platform**: macOS, Linux, Windows (no platform-specific code).

**Project Type**: CLI tool (Rust single binary).

**Performance Goals**: unchanged — expression validation is O(n) over the expression string (microseconds); evaluation unchanged for in-limit expressions.

**Constraints**:
- FR-001: `MAX_EXPRESSION_LEN = 4096`, `MAX_EXPRESSION_DEPTH = 256` (depth = max parenthesis nesting). Violations are clean errors naming the limit, before evaluation.
- FR-002: case-sensitive allow-list `{EXPRESSION, DATE_DIFF}`; anything else → exit 2 with the valid list; `derive` records nothing on rejection.
- FR-003: `as_number()` (Int coerces to Float).
- FR-004: `set_builtin_functions_disabled(true)` on the evalexpr context.
- FR-005: reject > 26 parents ("at most 26 parents supported: a–z"); exactly 26 works.
- FR-006: `verify_date_diff` rejects a negative computed duration with a `date2 - date1` convention-naming error (a swapped order). The existing `test_date_diff_negative` expectation flips from Ok to Err.

**Scale/Scope**: rework `src/verify/expression.rs` (validate + as_number + disable builtins + parent cap), `src/verify/mod.rs` (operation allow-list, remove pass-through), `src/verify/date_diff.rs` (sign enforcement), `src/cli/derive.rs` (operation validation + no-record-on-reject), and tests. No `main.rs`/CLI-flag changes.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new dependencies; evalexpr is already present. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | Unaffected (no graph changes). |
| IV. Rich Value Types | ✅ PASS | DATE_DIFF stays a first-class Duration derivation; sign enforced. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Unaffected. |
| IX. Test-Driven Verification | ✅ PASS | Test-first: crash regression, exit-code contract, integer coercion, determinism, parent cap, negative DATE_DIFF. |
| X. Documentation Matches Behavior | ✅ PASS | `--help`/README for the new limits and exit-2 operation validation in the same changeset. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy enforced; errors name the offending input. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/021-derive-verify-hardening/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (evalexpr API verification)
├── data-model.md        # Phase 1 output (verify domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (verify/derive CLI + evaluation invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── verify/
│   ├── expression.rs    # MODIFIED: validate_expression (len/depth); as_number(); disable builtins; parent cap
│   ├── mod.rs           # MODIFIED: operation allow-list; remove `_ => Ok(())` pass-through
│   └── date_diff.rs     # MODIFIED: reject negative computed duration (convention-naming error)
└── cli/
    └── derive.rs        # MODIFIED: operation validation (exit 2); no record on reject

tests/
├── unit/  (existing test files' inline `#[cfg(test)]` modules extended)
└── contract/test_cli_record_derive_contract.rs  # MODIFIED: exit-code + crash-regression contract tests
```

**Structure Decision**: Single-project layout. The verification entry points live in `src/verify/*`; the CLI wrappers (`run_verify_cli`, `execute_derive`) enforce the operation allow-list and exit codes. No new modules needed — the changes are localized to the existing verify/derive surfaces.

## Complexity Tracking

None — no constitution violations; no new dependencies (evalexpr already present).
