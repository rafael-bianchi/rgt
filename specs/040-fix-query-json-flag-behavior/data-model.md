# Data Model: Fix Query JSON Flag Behavior

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data changes. The relevant "entity" is the query output shape (already produced by `query_provenance`).

## 1. QueryOutput (transient, presentation-only)

Two presentations of the same lineage data:

### JSON (`--json`) — canonical machine contract

Exactly the `serde_json::Value` from `query_provenance` (no wrapper):

| Field | Type | Meaning |
|-------|------|---------|
| `node_id` | string | The queried node |
| `is_stale` | bool | Staleness of the queried node |
| `total_ancestors` | integer | Distinct ancestors counted (visited minus root) |
| `lineage_steps` | array | One entry per visited node (queried node first, then ancestors) |

Each step: `node_id`, `value_kind`, `value`, `is_stale`, `stale_reason` (nullable), `parent_ids` (array), `line_number` (nullable), `node_type`.

### Text (default) — human-readable

Same steps, rendered as text per the format in `research.md` §3. Both presentations carry identical data (SC-003).
