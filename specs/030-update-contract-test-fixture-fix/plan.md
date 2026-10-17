# Implementation Plan: Update Contract Test Fixture Fix

**Branch**: `feat/030-update-contract-test-fixture-fix` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/030-update-contract-test-fixture-fix/spec.md`

## Summary

Two contract tests in `tests/contract/test_update_contract.rs` hardcode `v0.4.1` as a "genuinely newer" stub release. Once `Cargo.toml` was bumped to `0.5.0`, the semver gate correctly reports "already at the latest version" and both tests fail before reaching their assertions. Fix is test-fixture-only: derive a strictly-newer stub tag from `CARGO_PKG_VERSION` at test time (bump the minor component) so the tests pass at 0.5.0 and keep passing on any future version bump. Production code is untouched.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021), tests only.

**Primary Dependencies**: `semver` (already a dependency; used by `src/updater/version.rs`). No new dependencies.

**Storage**: N/A (no data involved; test-fixture-only change).

**Testing**: `cargo test --test test_update_contract` (13 tests, currently 2 failing). A small unit test for the derivation helper asserts strict-newer semantics across release and pre-release shapes.

**Target Platform**: All (test-only change; runs in CI on macOS/Linux/Windows).

**Project Type**: CLI (single Rust binary) — but this feature is confined to `tests/`.

**Performance Goals**: N/A.

**Constraints**: Constitution IX (deterministic tests — the helper must derive from the compile-time version constant, never wall-clock/network). Constitution XI (no new dependencies, no production-code change).

**Scale/Scope**: One test file (`tests/contract/test_update_contract.rs`), two failing tests, one small helper.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Test-only change; no runtime effect. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Unaffected. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | The fix is itself a test-fixture repair; the helper gets a dedicated unit test. Deterministic via `CARGO_PKG_VERSION`. |
| X. Documentation matches behavior | PASS | No user-facing behavior changes. |
| XI. Idiomatic Rust, exit codes | PASS | No production change; tests use the existing `semver` crate. |

**Post-design re-check**: No violations — no dependencies added, no production behavior changed.

## Project Structure

### Documentation (this feature)

```text
specs/030-update-contract-test-fixture-fix/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output (N/A — no data; retained as a note)
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── fixture-invariants.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
tests/
└── contract/
    └── test_update_contract.rs   # the only production-adjacent change (tests only)
```

**Structure Decision**: Single test file. A helper (e.g. `fn newer_stub_tag() -> String`) is added inside the existing `mod tests` block of `test_update_contract.rs`, parsing `env!("CARGO_PKG_VERSION")` with the `semver` crate and returning `v{current}.{minor+1}.0`. No shared test-utility file (two call sites do not justify one).

## Complexity Tracking

> None — no constitution violations requiring justification.
