# Research: Store & Graph Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Resolves the plan's Technical Context unknowns. The SQLite/rusqlite mechanics are already established (and cited in findings §7.2); no external research needed beyond confirming the exact API/query shapes against the vendored `rusqlite`/SQLite docs.

## 1. Cycle guard at the write path (FR-001/FR-002)

**Decision**: In `insert_derivation_edge`, reject `parent == child` (self-loop) and reject any edge that would close a cycle by checking, via a recursive CTE, whether `parent` is already reachable from `child` before inserting.

**Rationale**: The existing guard is only in `GraphEngine::add_derivation_edge` (transient, `petgraph`), which runs during `build_from_db` rebuild — never on the `INSERT INTO derivation_edges` write path. SQLite's `WITH RECURSIVE ... UNION` is documented as the way to "prevent the recursion from entering an infinite loop if the graph contains cycles" (sqlite.org/lang_with.html), and can express the reachability query directly:
```sql
WITH RECURSIVE reachable(id) AS (
    SELECT child_node_id FROM derivation_edges WHERE parent_node_id = ?child
    UNION
    SELECT e.child_node_id FROM derivation_edges e JOIN reachable r ON e.parent_node_id = r.id
)
SELECT 1 FROM reachable WHERE id = ?parent LIMIT 1;
```
If this returns a row, adding `parent → child` closes a cycle → reject. Self-loop is the `parent == child` short-circuit (the CTE's base case doesn't include the start node itself). This runs on the actual write path and works for both new and existing databases (no migration needed).

**Alternatives considered**:
- A schema-level self-loop `CHECK (parent_node_id <> child_node_id)` alone — only catches self-loops, not multi-node cycles, and can't be added to an existing table without a migration (029). Added to the `CREATE TABLE` for new DBs as defense-in-depth, with the CTE as the actual guard.
- Doing the cycle check in Rust against the loaded graph before every insert — rejected: requires the full graph in memory at every write site and duplicates the DB's source of truth.

## 2. Non-zero `busy_timeout` (FR-003)

**Decision**: Call `conn.busy_timeout(std::time::Duration::from_secs(5))` in `DbStore::open` and `open_in_memory` immediately after `Connection::open`.

**Rationale**: rusqlite's `Connection::busy_timeout` "Set[s] a busy handler that sleeps for a specified amount of time when a table is locked" (docs.rs/rusqlite). SQLite's C default is `0` (immediate `SQLITE_BUSY`, no retry). WAL is already on (`PRAGMA journal_mode = WAL`), but WAL allows concurrent readers/writers, not concurrent *writers* — so a bounded wait is still required. 5s is bounded (avoids indefinite hangs) and covers the PreToolUse/PostToolUse back-to-back pair.

**Alternatives considered**: `busy_handler` with a custom backoff — unnecessary; `busy_timeout` is the simple, correct primitive.

## 3. ID widening (FR-005)

**Decision**: Change `generate_root_id` and `generate_derived_id` to truncate to 32 hex chars (`&hash_hex[..32]` = 128 bits) instead of 12 (48 bits).

**Rationale**: The full blake3 digest is 256 bits; the 48-bit truncation gives ~50% collision odds at ~2^24 (~16.7M) nodes. 128 bits makes collisions cryptographically negligible. IDs remain opaque strings, so existing 48-bit stored IDs stay valid; only new writes use the wider format (re-keying deferred to 029).

**Alternatives considered**: full 256-bit (64 hex) — equally fine but longer; 128 bits is the spec'd minimum and keeps IDs a bit more readable. 96 bits — marginal; 128 is the round, safe choice.

## 4. `ON CONFLICT` value update (FR-006)

**Decision**: In `insert_tracked_node`, extend the `ON CONFLICT(id) DO UPDATE SET` clause to update `number_val`, `date_val`, `duration_secs`, `source_doc_id`, `line_number` (plus the existing staleness columns) from `excluded.*`.

**Rationale**: findings §7.2: the clause today only refreshes `is_stale`/`stale_reason`/`updated_at`, so a colliding re-record keeps the first writer's value forever. `excluded.<col>` refers to the would-be-inserted value (sqlite.org/lang_upsert.html), so updating the value-bearing columns makes a re-record reflect the latest write. With 128-bit IDs this is defense-in-depth (collisions are negligible), but it removes the silent-corruption failure mode entirely.

**Alternatives considered**: erroring on collision — rejected (would break the intended idempotent re-record dedupe); updating only `value_kind`-matching columns is handled implicitly since the incoming `TrackedNode` carries a single coherent `value`.

## 5. Transactional hook inserts (FR-007)

**Decision**: In `handle_passive_hook_event`, wrap the per-value `insert_tracked_node` loop in `conn.unchecked_transaction()`, commit at the end, and roll back + fail-open on any error.

**Rationale**: The hook path (`src/hooks/mod.rs`) inserts each node with a bare per-row call and discards the result, so a mid-batch failure leaves partial state. `rgt record` already uses `conn_mut().transaction()`. Using `unchecked_transaction()` (on `&Connection`) keeps the existing `db.conn()` borrow while giving all-or-nothing semantics; the transaction is dropped (rolled back) if the loop errors, and the hook still returns `Ok(())` (constitution VIII fail-open).

**Alternatives considered**: `conn_mut().transaction()` — requires a mutable borrow; the hook currently holds `db` by value so this is also viable, but `unchecked_transaction` is the minimal change.
