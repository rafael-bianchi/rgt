# Research: Store Maintenance

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Decisions grounded in `src/store/schema.rs` (CREATE IF NOT EXISTS only, no `user_version`), `src/store/db.rs` (`open` → `initialize_schema`), and `src/graph/invalidation.rs` (`invalidate_file` counting). SQLite `PRAGMA user_version` is the documented, dependency-free primitive (findings §7.2).

## 1. Schema versioning via `PRAGMA user_version` (FR-001, FR-002)

**Decision**: Define `SCHEMA_VERSION: i32 = 1`. In `DbStore::open`, after `initialize_schema`, read `PRAGMA user_version`:
- `0` (unversioned — either a fresh DB or a pre-029 store) → stamp it to `SCHEMA_VERSION` (the schema is identical, so this is a backfill with no data migration).
- `== SCHEMA_VERSION` → proceed.
- `> SCHEMA_VERSION` → return a clear error ("store schema version N is newer than this build supports (max M) — upgrade rgt") instead of a SQL error.
- `< SCHEMA_VERSION` → run the ordered, idempotent migrations `[version..SCHEMA_VERSION]`, each bumping `user_version` inside the same transaction as its DDL.

**Rationale**: `PRAGMA user_version` is an application-owned integer SQLite makes no use of — the exact purpose. The `0`→`1` backfill is safe because 029 introduces no column/table change (the current schema already has everything); future changes bump `SCHEMA_VERSION` and append a migration.

**Alternatives considered**: the `rusqlite_migration` crate — rejected: it is a thin wrapper over the same `user_version` primitive and adds a dependency (Constitution XI). Detecting skew via `PRAGMA table_info` — rejected: brittle and indirect.

## 2. Garbage collection (FR-003, FR-004)

**Decision**: `rgt gc [--vacuum]` in a new `src/cli/gc.rs`. It runs a single transaction that deletes every node that is `is_stale = 1` AND has no derived dependents (`id NOT IN (SELECT parent_node_id FROM derivation_edges)`), returning the number of deleted rows. `--vacuum` runs `VACUUM` **after** the transaction commits (SQLite forbids `VACUUM` inside a transaction). Deleting a node cascades to its `derivation_edges` rows via the existing `ON DELETE CASCADE`.

**Rationale**: "obsolete node" is the conservative, safe garbage definition — it never touches live history or anything still depended on. Transactional delete means an interruption leaves the store consistent. `VACUUM` is opt-in (decided in clarify) so the default run stays fast and non-destructive.

**Alternatives considered**: also deleting orphaned `source_documents` (docs with no referencing nodes) — deferred: the spec scopes GC to nodes; doc cleanup is a follow-up if needed. A `--dry-run` flag — not required by the spec; skipped.

## 3. Distinct-node invalidated count (FR-005)

**Decision**: In `invalidate_file` (`src/graph/invalidation.rs`), collect every invalidated node ID (root + downstream dependents) into a `std::collections::HashSet` and return `set.len()`. `evaluate_and_invalidate_all` continues to sum per-file results (files are disjoint node sets in practice, so no cross-file dedup is needed).

**Rationale**: The staleness *state* is already idempotent (`mark_node_stale`); only the printed count is inflated. Dedup via a set is the minimal, deterministic fix and needs no new dependency.

**Alternatives considered**: dedup across files too (global set in `evaluate_and_invalidate_all`) — rejected: a node belongs to exactly one source document, so per-file dedup already yields the true distinct count.

## 4. Version-skew error surface (FR-001)

**Decision**: The error is a `rusqlite`-mapped failure surfaced by the CLI as exit code 1 (expected error), with a message naming both the store's version and the supported max — never the raw "no such column".

**Rationale**: Constitution XI requires errors that explain what happened and how to fix it, and exit 1 for expected errors.
