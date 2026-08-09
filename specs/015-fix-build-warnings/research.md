# Research: Eliminate Build Warnings & Cargo Suggestions

**Feature**: 015-fix-build-warnings
**Date**: 2026-08-09

## Decision: Remove unused glob re-exports in `src/graph/mod.rs` and `src/store/mod.rs`

**Decision**: Remove `pub use engine::*;` (graph/mod.rs:4) and `pub use queries::*;` / `pub use schema::*;` (store/mod.rs:6-7).

**Rationale**: The glob re-exports are unused — no code references `engine::*`, `queries::*`, or `schema::*` via these paths. Before removal, each module's public items must be verified to still be reachable through their canonical paths (e.g., `graph::engine::GraphEngine` instead of `graph::GraphEngine` if any caller uses the re-export). Since the warnings are `unused_imports`, the re-exports genuinely have no consumers and can be deleted.

**Alternatives considered**:
- Keep and add `#[allow(unused_imports)]`: Rejected — masks the warning without cleaning the code; violates the spec's no-blanket-suppression rule.

**Pre-requisite verification**: Grep the crate for any usage of `GraphEngine`, `DbStore`, `schema`, etc. through the re-export paths before deleting; if a caller relies on the short path, fix the caller to use the canonical path instead.

## Decision: Delete `list_stale_nodes_json` (leftover from removed MCP)

**Decision**: Remove `pub fn list_stale_nodes_json` from `src/query/mod.rs:72`.

**Rationale**: The function was part of the removed MCP server feature (spec 013 removed MCP, including the `list_stale_values` tool). It is unreferenced anywhere in the crate or tests (`grep` shows only its definition). Retaining dead code contradicts the cleanup goal.

**Alternatives considered**:
- `#[allow(dead_code)]` + justification: Rejected — the function is genuinely obsolete (MCP is gone), not future-facing API. Deletion is honest and cleaner.
- Wire to a live call site: Rejected — `rgt status --stale-only --json` already provides the JSON stale-value output; duplicating via this function adds no value.

## Decision: Fix `run_verify_cli` dead-code warning by calling the bin-crate copy

**Decision**: Change `src/main.rs:159` from `rgt::verify::run_verify_cli(...)` to `crate::verify::run_verify_cli(...)`.

**Rationale**: `src/main.rs:9` declares `mod verify;`, creating a bin-crate copy of `src/verify/mod.rs`. `main.rs:159` currently calls the lib-crate path (`rgt::verify::run_verify_cli`), leaving the bin copy's `run_verify_cli` dead → warning. Calling `crate::verify::run_verify_cli` uses the bin copy, eliminating the warning. The lib copy remains `pub` (lib crates don't warn on unused `pub` items).

**Alternatives considered**:
- Remove `mod verify;` from main.rs: Rejected — `src/cli/derive.rs` uses `crate::verify::verify` in the bin context; removing the declaration would break that import (the reason `mod verify;` was added in feature 001).
- `#[allow(dead_code)]` on the bin's `run_verify_cli`: Rejected — the one-line call-path fix is cleaner.

## Decision: Implement `Default` for `GraphEngine` (clippy suggestion)

**Decision**: Add `impl Default for GraphEngine { fn default() -> Self { Self::new() } }` in `src/graph/engine.rs`.

**Rationale**: `GraphEngine::new()` takes no arguments and initializes to empty state — a textbook `Default`. Implementing it is idiomatic and lets callers use `GraphEngine::default()` / `Default::default()`.

**Alternatives considered**:
- Declined with rationale: Rejected — there is no design reason to decline; the suggestion is genuinely good. (Declination is only appropriate where `match` clarity etc. wins; this is a pure win.)

## Decision: Convert single-pattern `match` to `if let` in distribution tests (clippy)

**Decision**: Convert the `match output { Ok(out) => {...}, Err(e) => {...} }` blocks in `tests/integration/test_distribution_quickstart.rs:40` and `tests/integration/test_distribution_performance.rs:34` to `if let` where the match has exactly one meaningful arm plus an error arm handled via `if let ... else` / early handling.

**Rationale**: Clippy's `single_match` / `match_single_binding` lint flags a `match` with a single pattern. In these tests, the `Ok` arm is the only actionable branch; the `Err` arm just returns early. `if let` is the idiomatic form.

**Implementation note**: The exact rewrite depends on the current error handling. If the `Err` arm does `return`, use:
```rust
let Ok(out) = output else { return; };
```
If it returns a value or asserts, preserve that behavior exactly. Verify by reading each test's full body before editing.

**Alternatives considered**:
- `#[allow(clippy::single_match)]`: Rejected — the lint is correct; idiomatic rewrite is preferred.

## Decision: Simplify `!x.is_some()` to `x.is_none()` in `test_hooks_installer.rs:95`

**Decision**: Change `!hooks.get("rgt_hook").is_some()` to `hooks.get("rgt_hook").is_none()`.

**Rationale**: Clippy's `nonminimal_bool` lint flags the double negation. `is_none()` is semantically identical and clearer.

**Alternatives considered**:
- None — this is a pure simplification with no trade-off.

## Decision: All changes are refactor-only (verification strategy)

**Decision**: After edits, verify zero-warning status with:
1. `cargo build --all-targets 2>&1 | grep warning` → empty
2. `cargo test --no-run 2>&1 | grep warning` → empty
3. `cargo clippy --all-targets 2>&1 | grep warning` → empty
4. `cargo test` → 108 tests pass (baseline count)

**Rationale**: Each verification is deterministic (no wall-clock/flaky deps), satisfying Constitution IX.

**Alternatives considered**:
- Trusting compilation: Rejected — explicit verification is required by the success criteria and Constitution IX.

## Implementation Results (SC-004 disposition coverage)

All 9 warnings (W1-W9) were resolved at implementation time. Final verification (2026-08-09):

| Warning | Disposition | Result |
|---|---|---|
| W1 `engine::*` unused import | Removed re-export; updated `tests/unit/test_invalidation.rs` to canonical `rgt::graph::engine::GraphEngine` path | ✅ |
| W2 `queries::*` unused import | Removed re-export (all callers use canonical `crate::store::queries::`) | ✅ |
| W3 `schema::*` unused import | Removed re-export (all callers use canonical `crate::store::schema::`) | ✅ |
| W4 `list_stale_nodes_json` dead fn | Deleted (MCP leftover); also removed now-unused `list_stale_nodes` import in `src/query/mod.rs` | ✅ |
| W5 `run_verify_cli` dead fn (bin) | Changed `src/main.rs:159` to `crate::verify::run_verify_cli` | ✅ |
| W6 clippy `Default` for `GraphEngine` | Implemented `impl Default` delegating to `new()` | ✅ |
| W7 clippy `match` → `if let` (quickstart) | Converted | ✅ |
| W8 clippy `match` → `if let` (performance) | Converted | ✅ |
| W9 clippy `!is_some()` → `is_none()` | Simplified | ✅ |

Final counts: `cargo build --all-targets` = 0 warnings; `cargo clippy --all-targets` = 0 lints; `cargo test --no-run` = 0 warnings; full suite 14/14 passing.
