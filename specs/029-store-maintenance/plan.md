# Implementation Plan: Store Maintenance

**Branch**: `feat/029-store-maintenance` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-store-maintenance/spec.md`

## Summary

Add store hygiene so the provenance graph degrades gracefully. Track the SQLite schema version via `PRAGMA user_version`, fail fast with a clear error when the store is newer than the binary supports, and apply forward migrations idempotently. Add an opt-in `rgt gc` command that transactionally removes obsolete nodes (obsolete nodes (stale, no dependents)) and reports the count, with `--vacuum` for physical space reclamation. Fix the staleness invalidated-count to report distinct nodes (no double-counting a node reachable from multiple stale ancestors).

## Technical Context

**Language/Version**: Rust (existing project; edition 2021)

**Primary Dependencies**: `rusqlite` (SQLite), `chrono`, `clap`, `petgraph`, `serde_json` — all existing. No new dependencies.

**Storage**: SQLite at `.rgt/store.db`; `PRAGMA user_version` for schema versioning.

**Testing**: `cargo test` — unit `tests/unit/store_integrity.rs`, `tests/unit/test_invalidation.rs`; contract `tests/contract/test_cli_record_derive_contract.rs` (or a new `tests/contract/test_store_maintenance_contract.rs`).

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: `rgt gc` and the schema-version check are O(1)/O(nodes); the version check adds no measurable startup cost (one `PRAGMA` read).

**Constraints**: Zero new runtime dependencies (Constitution I, XI); GC is opt-in and transactional (never corrupts); version-skew failure is a clear error (exit 1), never a raw SQL error.

**Scale/Scope**: Schema versioning is a single integer; GC operates on the existing `tracked_nodes`/`derivation_edges` tables.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | `PRAGMA user_version` is native SQLite via `rusqlite`; no new crate. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | GC only removes obsolete nodes; invalidated-count fix is cosmetic. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Additive `gc` subcommand + `--vacuum` flag. |
| VI. Dual passive/active surface | PASS | `gc` is an active command. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | New unit + contract tests. |
| X. Documentation matches behavior | PASS | README/AGENTS updated with `rgt gc`. |
| XI. Idiomatic Rust, exit codes | PASS | Version-skew error exits 1 with an actionable message. |

**Post-design re-check**: No violations — no dependencies added, all changes additive or a cosmetic count fix.

## Project Structure

### Documentation (this feature)

```text
specs/029-store-maintenance/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── store-maintenance-invariants.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── store/
│   ├── schema.rs        # SCHEMA_VERSION const + user_version stamping/migrations
│   ├── db.rs            # run version check on open
│   └── queries.rs       # delete_stale_leaf_nodes / count helpers
├── graph/
│   └── invalidation.rs  # distinct-node dedup in invalidate_file
├── cli/
│   ├── gc.rs            # NEW: execute_gc (collect + optional vacuum)
│   └── mod.rs
└── main.rs              # Gc subcommand wiring

tests/
├── unit/
│   ├── store_integrity.rs    # schema-version unit tests
│   └── test_invalidation.rs  # dedup count test
└── contract/
    └── test_store_maintenance_contract.rs  # gc + version-skew contract
```

**Structure Decision**: Single-project CLI. One new module (`src/cli/gc.rs`); edits to `schema.rs`, `db.rs`, `queries.rs`, `invalidation.rs`, `main.rs`.

## Complexity Tracking

> None — no constitution violations requiring justification.
