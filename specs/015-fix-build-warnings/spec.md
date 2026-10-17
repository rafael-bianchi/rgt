# Feature Specification: Eliminate Build Warnings & Cargo Suggestions

**Feature Branch**: `fix/015-build-warnings`

**Created**: 2026-08-09

**Status**: Draft

**Input**: User description: "Check all builds for warnings and cargo suggestions, planning to solve all of them."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Warning-Free Build for All Targets (Priority: P1)

A developer runs `cargo build`, `cargo build --all-targets`, `cargo test`, and `cargo clippy --all-targets` on the repository. The build output is clean: zero warnings from rustc or clippy, across the library, binary, and all test targets. A clean build reduces noise in CI logs, makes genuine new warnings (from future changes) immediately visible, and satisfies the constitution's code-quality mandate that CI must reject warning-producing PRs.

**Why this priority**: The current build produces warnings in the library, binary, and test targets (5 rustc warnings in the bin target alone, plus clippy lints). These mask regressions — a new warning is indistinguishable from pre-existing noise. This is a foundational hygiene issue.

**Independent Test**: Run `cargo build --all-targets 2>&1 | grep warning` and `cargo clippy --all-targets 2>&1 | grep warning` — both produce zero output. CI's `fmt` and `clippy` jobs are green.

**Acceptance Scenarios**:

1. **Given** a clean `develop` checkout, **When** `cargo build` runs, **Then** no warnings are emitted (the bin target generates 0 warnings, down from 5).
2. **Given** a clean `develop` checkout, **When** `cargo build --all-targets` runs, **Then** no warnings are emitted (unused imports, dead code, and clippy lints are all resolved).
3. **Given** a clean `develop` checkout, **When** `cargo test --no-run` runs, **Then** test targets compile with no warnings.
4. **Given** a clean `develop` checkout, **When** `cargo clippy --all-targets` runs, **Then** no clippy warnings or errors are emitted.
5. **Given** the full test suite, **When** `cargo test` runs, **Then** all 108 tests still pass (no behavior changed by warning cleanup).
6. **Given** the CI pipeline, **When** a PR is opened, **Then** the `fmt`, `clippy`, and all `test` jobs are green with zero warnings.

---

### User Story 2 - Cargo Suggestions Addressed (Priority: P2)

A developer reviews `cargo clippy` output for actionable suggestions (e.g., "consider adding a `Default` implementation"). These suggestions are either implemented (where they improve the code) or explicitly documented as intentional deviations (where they don't). No suggestion is left unexamined.

**Why this priority**: Clippy suggestions are idiomatic-Rust improvements. Addressing them raises code quality, but each one needs judgment — some are genuinely better, others are stylistic. This is secondary to the zero-warning goal.

**Independent Test**: `cargo clippy --all-targets` emits no warnings. A review note in the plan documents each suggestion that was declined and why.

**Acceptance Scenarios**:

1. **Given** `src/graph/engine.rs` defines `GraphEngine::new()`, **When** clippy suggests adding `Default`, **Then** either `impl Default for GraphEngine` is added (delegating to `new()`), or a documented rationale exists for declining it.
2. **Given** a test uses `match` to destructure a single `Result` pattern, **When** clippy suggests `if let`, **Then** the test is updated to use `if let` (or an equivalent idiomatic form).
3. **Given** a boolean expression like `!x.is_some()`, **When** clippy suggests `x.is_none()`, **Then** the expression is simplified.
4. **Given** unused imports exist in `src/graph/mod.rs` and `src/store/mod.rs`, **When** cleaned, **Then** either the imports are removed or the items they reference are actually used.

---

### Edge Cases

- **Dead code that is intentionally public API**: `list_stale_nodes_json` and `run_verify_cli` are never called within the crate but may be part of the public/test surface. Removing them could break external contract tests. Resolution: either mark with `#[allow(dead_code)]` + a justification comment, wire them to a real call site, or remove them if confirmed unused — with a note.
- **Clippy suggestion conflicts with intended style**: A `match` with a single arm might be clearer than `if let` in context. Each suggestion must be evaluated on merit, not applied blindly.
- **Warning fixes must not change behavior**: All changes are refactors only — no logic changes, no signature changes that would break callers, no test expectation changes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `cargo build` MUST complete with zero warnings on the bin target.
- **FR-002**: `cargo build --all-targets` MUST complete with zero warnings across lib, bin, and test targets.
- **FR-003**: `cargo test --no-run` MUST compile all test targets with zero warnings.
- **FR-004**: `cargo clippy --all-targets` MUST complete with zero warnings and zero errors.
- **FR-005**: All existing tests MUST continue to pass (no behavior change from warning cleanup).
- **FR-006**: The unused import in `src/graph/mod.rs:4` (`engine::*`) MUST be resolved (removed or actually used).
- **FR-007**: The unused imports in `src/store/mod.rs:6-7` (`queries::*`, `schema::*`) MUST be resolved (removed or actually used).
- **FR-008**: The dead function `list_stale_nodes_json` in `src/query/mod.rs:72` MUST be resolved (removed, used, or explicitly allowed with justification).
- **FR-009**: The dead function `run_verify_cli` in `src/verify/mod.rs:32` MUST be resolved (removed, used, or explicitly allowed with justification).
- **FR-010**: The clippy suggestion for a `Default` implementation on `GraphEngine` (src/graph/engine.rs:13) MUST be addressed (implemented or explicitly declined with rationale).
- **FR-011**: The `match`-with-single-pattern clippy warnings in `tests/integration/test_distribution_quickstart.rs:40` and `tests/integration/test_distribution_performance.rs:34` MUST be addressed (converted to `if let` or documented).
- **FR-012**: The `!x.is_some()` nonminimal-bool warning in `tests/integration/test_hooks_installer.rs:95` MUST be resolved (simplified to `is_none()`).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo build --all-targets` and `cargo clippy --all-targets` both report zero warnings (0 of 8+ current warnings remain).
- **SC-002**: `cargo test` reports 108+ tests passing with zero failures and zero warnings during compilation.
- **SC-003**: The CI `clippy` job is green on the fix PR, confirming zero clippy warnings under CI's toolchain.
- **SC-004**: Every clippy suggestion and dead-code item has a disposition: either fixed in code or documented with a rationale in the plan/research (100% coverage of the warning inventory).
- **SC-005**: No public function signatures or behaviors change — the cleanup is refactor-only, verified by the full test suite passing unchanged.
- **SC-006**: `cargo fmt --all -- --check` passes after all changes.

## Assumptions

- All warnings are pre-existing and unrelated to any in-flight feature (develop is clean at the time of this spec).
- Removing or `#[allow]`-marking dead code is acceptable where the function is confirmed unreferenced by the crate and its tests; if it's part of a documented public surface, a `#[allow(dead_code)]` with justification is preferred over deletion.
- Clippy suggestions are advisory; each is evaluated on merit. The default is to apply them, but a documented declination is acceptable (e.g., where `match` is clearer than `if let`).
- No new dependencies are introduced; no `#![allow]` crate-level attributes that blanket-suppress warnings (per-context allowances with comments are acceptable).
- The 108-test baseline count holds at the time of implementation; the count may shift slightly if a test is legitimately adjusted, but no test should be deleted to hide a warning.
