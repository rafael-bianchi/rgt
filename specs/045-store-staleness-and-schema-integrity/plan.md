# Implementation Plan: Store Staleness and Schema Integrity

**Branch**: `fix/045-store-staleness-and-schema-integrity` | **Date**: 2026-10-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/045-store-staleness-and-schema-integrity/spec.md`

## Summary

Resolve the critical P0/P1 store integrity and invalidation defects identified in the review:
1. When a source file is modified and re-read (via passive hook or `rgt record`), invalidate removed/modified root values and cascade staleness downstream before updating file metadata.
2. When multiple paths alias the same physical file `(dev, ino)`, re-key existing `tracked_nodes.source_doc_id` before deleting duplicate `source_documents` rows so root nodes never become un-tracked orphans.
3. Add seamless SQLite schema v2→v3 migration adding `duration_secs INTEGER` and updating the `value_kind` CHECK constraint to include `'DURATION'`, while strictly rejecting newer schemas (`user_version > SCHEMA_VERSION`) before executing any DDL.
4. Make `evaluate_and_invalidate_all` run automatically before active commands (`query`, `derive`, `verify`, `graph`, `gc`), fix Tier-1 timestamp preservation so `REHASH_THRESHOLD` is not reset prematurely, and make `rgt gc` iterative to a fixed point.
5. Make `get_or_create_rdf_project_id` read before writing, walk parent directories to discover `.rgt/store.db` project roots, ensure passive hooks never auto-initialize uninitialized directories, and wrap document upsert + node insertion into single atomic transactions.

## Technical Context

**Language/Version**: Rust 2021 edition.
**Primary Dependencies**: `rusqlite` (bundled SQLite 3), `blake3`, `petgraph`, `chrono`, `walkdir`.
**Storage**: SQLite `.rgt/store.db` with `PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;`. Schema version 3.
**Testing**: Rust unit tests (`tests/unit/test_invalidation.rs`, `tests/unit/store_integrity.rs`), contract tests (`tests/contract/test_store_maintenance_contract.rs`, `tests/contract/test_hooks_contract.rs`), integration tests.

## Constitution Check

| Principle / rule | Status | Reason |
|---|---|---|
| I. Rust-only single binary | PASS | All fixes are pure Rust in `src/store`, `src/graph`, `src/detection`, `src/cli`. |
| II. Speed-first detection | PASS | Passive hook exits early if no `.rgt/` exists; bulk queries reduce graph load from $O(E \cdot (V+E))$ to $O(V+E)$. |
| III. Minimal & correct invalidation | PASS | Core fix: re-recording modifies invalidates obsolete nodes immediately rather than masking staleness. |
| IV. First-class Date and Duration | PASS | Schema migration v3 ensures all stores cleanly support `DURATION` values without SQLite check constraint failures. |
| V-VIII. CLI-first integration | PASS | Commands evaluate staleness before presenting data; no hook protocol changes. |
| IX. Test-driven verification | PASS | Every scenario covered by deterministic tests. |
| XI. Quality, errors, portability | PASS | Checked timestamp arithmetic avoids i64 overflow; atomic transactions prevent corrupt state. |

## Proposed Changes

### 1. Schema Migration & Project Root Discovery (`src/store/`)
- In `src/store/schema.rs`:
  - Bump `SCHEMA_VERSION = 3`.
  - In `stamp_or_migrate_schema`:
    - Check `current_ver > SCHEMA_VERSION` before doing anything; return an error immediately.
    - If `current_ver < 3`:
      - Alter/rebuild `tracked_nodes` to include `duration_secs INTEGER` and `CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION'))`.
      - Preserve all existing rows, edges, and capture records.
      - Set `PRAGMA user_version = 3;`.
- In `src/store/db.rs`:
  - Enhance `DbStore::open_in_project(start_path)`: walk upwards looking for `.rgt/store.db`. If found, use that project root. If not found and called from active CLI (`rgt init`), create in `start_path`; if called from passive hook, return an error/None without creating `.rgt/`.
  - Fix `get_or_create_rdf_project_id`: query `SELECT value FROM project_settings WHERE key = 'rdf_project_id'` first. Only execute `INSERT` if absent.

### 2. Path Aliasing & Re-keying (`src/store/queries.rs`)
- In `upsert_source_document`:
  - When a duplicate row with matching `(dev, ino)` or matching canonical relative path exists with a different ID:
    - Execute `UPDATE tracked_nodes SET source_doc_id = ?1 WHERE source_doc_id = ?2;` before deleting the duplicate row.
  - Wrap the entire operation in the caller's transaction.

### 3. Staleness Invalidation on Re-recording (`src/store/queries.rs` & `src/graph/`)
- When recording a file whose `blake3_hash` differs from `source_documents.blake3_hash`:
  - Query all existing active (`is_stale = 0`) root nodes for this `source_doc_id`.
  - Compare the newly extracted values with existing root nodes by `(line_number, occurrence, value)`.
  - For any existing root node not present in the new set:
    - Mark `is_stale = 1`, `stale_reason = 'SOURCE_FILE_MODIFIED'`.
    - Cascade invalidation downstream to all descendant derived nodes.
  - Insert new root nodes that weren't already present.
  - Update `source_documents` hash and metadata.
  - All performed atomically inside a single SQLite transaction!

### 4. Graph Invalidation Optimization & Pre-command Staleness (`src/graph/`)
- In `src/graph/engine.rs`:
  - `build_from_db`: load all nodes via `SELECT ... FROM tracked_nodes` and all edges via `SELECT ... FROM derivation_edges` in two bulk queries.
  - Check acyclicity once via `petgraph::algo::is_cyclic_directed(&graph)` instead of per-edge insertion.
- In `src/graph/invalidation.rs`:
  - Add helper `ensure_staleness_evaluated(conn)`.
  - Wire it into `cli::query`, `cli::derive`, `cli::verify`, `cli::graph`, `cli::gc`.

### 5. Tier-1 Timestamp & Checked Math (`src/detection/`)
- In `src/detection/tier1.rs`:
  - Replace `mtime * 1_000_000_000 + mtime_nsec` with `mtime.checked_mul(1_000_000_000).and_then(|v| v.checked_add(mtime_nsec))`.
- In `src/detection/mod.rs` & `src/graph/invalidation.rs`:
  - Only update `last_checked_at` when Tier-2 hash check actually executes.
  - When Tier-1 indicates change but Tier-2 confirms hash is unchanged, update `mtime_nsec` and `last_checked_at` in DB to avoid repeated Tier-2 checks on every call.

### 6. Iterative Fixed-Point Garbage Collection (`src/cli/gc.rs`)
- In `src/cli/gc.rs`:
  - Loop `delete_obsolete_nodes` until the count of removed nodes is 0.
  - Clean up any `source_documents` rows where the file is deleted on disk and no `tracked_nodes` reference that `source_doc_id`.
