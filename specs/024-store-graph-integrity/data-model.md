# Data Model: Store & Graph Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for the store/graph integrity changes. No new entities — the change tightens invariants on existing ones.

## Entities

### DbStore / Connection

The SQLite connection wrapper.

| Attribute | Type | Notes |
|---|---|---|
| `conn` | `rusqlite::Connection` | Every open sets a non-zero `busy_timeout` (e.g. 5s). |

Rules:
- `open` and `open_in_memory` both set `busy_timeout` (FR-003).
- WAL mode already on; `busy_timeout` covers concurrent writers.

### TrackedNode

A stored value node.

| Attribute | Type | Notes |
|---|---|---|
| `id` | `String` | **Now ≥ 128 bits** (32 hex chars) for new writes (FR-005). |
| `value` / `value_kind` / `source_doc_id` / `line_number` | — | Value-bearing columns. |

Rules:
- `insert_tracked_node`'s `ON CONFLICT(id) DO UPDATE` updates all value-bearing columns (`number_val`, `date_val`, `duration_secs`, `source_doc_id`, `line_number`) to the latest write (FR-006).

### DerivationEdge

A parent→child provenance edge.

| Attribute | Type | Notes |
|---|---|---|
| `parent_node_id` / `child_node_id` | `String` | References to `tracked_nodes.id`. |

Rules:
- Insert is rejected when `parent_node_id == child_node_id` (self-loop) or when `parent` is already reachable from `child` (would close a cycle) — enforced at the write path via a recursive CTE (FR-001).
- New-DB schema additionally carries `CHECK (parent_node_id <> child_node_id)` (defense-in-depth).

### InvalidationCascade

The rebuilt in-memory graph.

| Attribute | Type | Notes |
|---|---|---|
| `engine` | `GraphEngine` | Built from DB edges. |

Rules:
- `build_from_db` returns an error if any edge can't be added (cycle), instead of silently dropping it (FR-002).

### StatusReport

The `rgt status` output path.

Rules:
- `evaluate_and_invalidate_all` failures propagate to a non-zero exit (FR-004), never a false "all fresh".

## Validation rules (from FR-001..FR-007)

- **Cycle rejection at write** (FR-001/FR-002).
- **Concurrency-safe** (FR-003/FR-004).
- **Collision-resistant IDs + latest-value re-record** (FR-005/FR-006).
- **Atomic hook inserts** (FR-007).

## Out of scope

Schema migrations (`PRAGMA user_version`), garbage collection, `total_invalidated` double-count (029), and re-keying existing 48-bit IDs (029).
