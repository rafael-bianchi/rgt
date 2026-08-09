# Data Model: Eliminate Build Warnings & Cargo Suggestions

**Feature**: 015-fix-build-warnings

## Overview

This feature has no runtime data model — it is a code-hygiene cleanup. This document formalizes the warning inventory as the "entities" with their dispositions, so the contracts and quickstart can reference concrete items.

## Warning Inventory (entities)

| ID | Warning | Location | Kind | Disposition |
|---|---|---|---|---|
| W1 | unused import `engine::*` | `src/graph/mod.rs:4` | rustc | Remove the glob re-export; verify no caller relies on the short path |
| W2 | unused import `queries::*` | `src/store/mod.rs:6` | rustc | Remove the glob re-export; verify callers use canonical paths |
| W3 | unused import `schema::*` | `src/store/mod.rs:7` | rustc | Remove the glob re-export; verify callers use canonical paths |
| W4 | dead fn `list_stale_nodes_json` | `src/query/mod.rs:72` | rustc (dead_code) | Delete (MCP leftover) |
| W5 | dead fn `run_verify_cli` (bin copy) | `src/main.rs:159` calls lib path | rustc (dead_code) | Change call to `crate::verify::run_verify_cli` |
| W6 | clippy: consider `Default` impl | `src/graph/engine.rs:13` | clippy | Add `impl Default for GraphEngine` |
| W7 | clippy: single-pattern `match` | `tests/integration/test_distribution_quickstart.rs:40` | clippy | Convert to `if let` |
| W8 | clippy: single-pattern `match` | `tests/integration/test_distribution_performance.rs:34` | clippy | Convert to `if let` |
| W9 | clippy: `!x.is_some()` → `is_none()` | `tests/integration/test_hooks_installer.rs:95` | clippy | Simplify to `.is_none()` |

## State Transitions

Each warning item transitions through:

```
present ──fix applied──> verified-removed (grep shows no output)
   │
   └──declined (documented)──> justified (only where appropriate, per spec Assumptions)
```

For this feature, the expected terminal state for ALL items is `verified-removed` (no declinations required — every suggestion is genuinely worth applying per research.md).

## Validation Invariants

- After cleanup, `cargo build --all-targets 2>&1 | grep -c warning` == 0
- After cleanup, `cargo clippy --all-targets 2>&1 | grep -c warning` == 0
- `cargo test` still reports 108 passing tests
- No `#![allow(...)]` crate-level attributes are introduced
- No public function signatures change
