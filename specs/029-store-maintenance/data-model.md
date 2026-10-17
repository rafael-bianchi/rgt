# Data Model: Store Maintenance

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

No new persisted tables; this feature adds a schema-version field and transient GC/dedup logic.

## 1. SchemaVersion (new, persisted via SQLite)

- Stored in `PRAGMA user_version` (an application-owned integer).
- `SCHEMA_VERSION = 1` is the current supported version.
- Existing unversioned stores (`user_version == 0`) are backfilled to `1` on open (no data migration — the schema is unchanged).

State transitions on open:

| Stored `user_version` | Action |
|-----------------------|--------|
| `0` | stamp to `SCHEMA_VERSION` |
| `== SCHEMA_VERSION` | proceed |
| `< SCHEMA_VERSION` | apply migrations `[v..SCHEMA_VERSION]` |
| `> SCHEMA_VERSION` | error (exit 1) — newer store |

## 2. Migration (new, transient)

- An ordered, idempotent step that moves the schema from version `N` to `N+1`.
- Currently the list is empty (`SCHEMA_VERSION == 1`); the framework exists so a future schema change appends a migration.

## 3. ObsoleteNode (GC candidate, transient)

A `tracked_nodes` row that is:

- `is_stale = 1`, AND
- not a parent in any `derivation_edges` row (no derived dependents).

`rgt gc` deletes these transactionally (cascading to their `derivation_edges` via `ON DELETE CASCADE`) and reports the count. `--vacuum` additionally reclaims pages after commit.

## 4. InvalidationCount (transient)

`invalidate_file` returns the number of **distinct** node IDs invalidated (root + downstream), deduplicated via a set (FR-005).
