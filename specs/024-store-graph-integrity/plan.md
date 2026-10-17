# Implementation Plan: Store & Graph Integrity

**Branch**: `024-store-graph-integrity` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/024-store-graph-integrity/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Make the SQLite store and provenance graph corruption-resistant (findings §7.2):

1. **Cycle/self-loop guard at the write path (FR-001/FR-002)**: `insert_derivation_edge` (`src/store/queries.rs`) gains a pre-insert recursive-CTE reachability check (reject when `parent == child` or when `parent` is already reachable from `child`), so cycles are rejected where they're written — not only in the transient in-memory graph. `build_from_db` (`src/graph/invalidation.rs`) stops `let _ =`-discarding edge errors.
2. **Concurrency-safe reads (FR-003/FR-004)**: `DbStore::open`/`open_in_memory` (`src/store/db.rs`) set a non-zero `busy_timeout`; `execute_status` (`src/cli/status.rs`) propagates `evaluate_and_invalidate_all` failures instead of discarding them.
3. **Collision-resistant IDs + latest-write values (FR-005/FR-006)**: node IDs widen from 48 to 128 bits (`[..12]` → `[..32]` in `src/types/node.rs`); `insert_tracked_node`'s `ON CONFLICT` updates all value-bearing columns (`number_val`, `date_val`, `duration_secs`, `source_doc_id`, `line_number`), not just staleness.
4. **Atomic hook inserts (FR-007)**: `handle_passive_hook_event` (`src/hooks/mod.rs`) wraps its extraction loop in a single transaction (rollback on failure, still fail-open), matching `rgt record`.

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: `rusqlite` (bundled SQLite) — already present. **No new dependencies** (the cycle guard is a SQL recursive CTE; the in-memory `petgraph` graph is unchanged).

**Storage**: SQLite (`.rgt/store.db`, WAL mode). Schema gains a self-loop `CHECK` on `derivation_edges` for **new** databases only (existing DBs are handled by the pre-insert CTE guard; adding a CHECK to an existing table would require a migration, which is out of scope → 029).

**Testing**: `cargo test` — unit/contract tests for cycle rejection (self-loop, 2-node, multi-node), `busy_timeout` presence, ID length ≥ 32 hex, `ON CONFLICT` value-column update, and hook-insert transactionality. `cargo fmt`/`clippy`.

**Target Platform**: macOS, Linux, Windows (SQLite behavior is uniform).

**Project Type**: CLI tool (Rust single binary).

**Performance Goals**: the recursive-CTE reachability check is O(edges-in-subgraph) per insert and only runs on derivation-edge inserts (already the rare path); `busy_timeout` adds bounded lock-wait; no change to the sub-ms detection path.

**Constraints**:
- FR-001: `insert_derivation_edge` rejects `parent == child` and any edge closing a cycle, before writing.
- FR-002: `build_from_db` returns `Err` on a cycle instead of silently dropping the edge.
- FR-003: non-zero `busy_timeout` (e.g. 5s) on every connection.
- FR-004: `execute_status` propagates invalidation errors (non-zero exit).
- FR-005: root + derived IDs are ≥ 32 hex chars (128 bits).
- FR-006: `ON CONFLICT(id)` updates value-bearing columns to the latest write.
- FR-007: hook inserts are one transaction; failures roll back and still fail open.

**Scale/Scope**: edits to `src/store/db.rs`, `src/store/schema.rs`, `src/store/queries.rs`, `src/graph/invalidation.rs`, `src/cli/status.rs`, `src/types/node.rs`, `src/hooks/mod.rs`, and tests. No new modules; no CLI-flag changes.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new dependencies. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | This feature strengthens it — cycles rejected at the persistent write path. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Hook transaction still fails open (non-blocking). |
| IX. Test-Driven Verification | ✅ PASS | Test-first tasks for each finding. |
| X. Documentation Matches Behavior | ✅ PASS | No user-facing flag/CLI change; behavior change is a correctness fix. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy enforced. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/024-store-graph-integrity/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (recursive-CTE cycle guard, busy_timeout)
├── data-model.md        # Phase 1 output (store/graph domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (store/graph invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── store/
│   ├── db.rs            # MODIFIED: busy_timeout on open/open_in_memory
│   ├── schema.rs        # MODIFIED: self-loop CHECK on derivation_edges (new DBs)
│   └── queries.rs       # MODIFIED: insert_derivation_edge cycle guard; insert_tracked_node value-column update
├── graph/
│   └── invalidation.rs  # MODIFIED: build_from_db propagates (not drops) edge errors
├── cli/
│   └── status.rs        # MODIFIED: propagate evaluate_and_invalidate_all errors
├── types/
│   └── node.rs          # MODIFIED: ID truncation [..12] -> [..32] (root + derived)
└── hooks/
    └── mod.rs           # MODIFIED: transactional batch insert

tests/
└── (unit/contract tests for the above)
```

**Structure Decision**: Single-project layout. The cycle guard lives in `queries.rs` (the write path), `busy_timeout` in `db.rs`, ID width in `node.rs`, and the transaction in `hooks/mod.rs` — no new modules.

## Complexity Tracking

None — no constitution violations; no new dependencies (the cycle guard is SQL, not a new crate).
