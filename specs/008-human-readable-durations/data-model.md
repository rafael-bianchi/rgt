# Data Model: Human-Readable Duration Display

**Feature**: `008-human-readable-durations` | **Date**: 2026-08-01

## Entities

### ValueData::Duration

**Description**: A derived graph node value representing a time span between two Date nodes, stored as a `chrono::Duration` (signed duration with nanosecond precision).

**No structural changes**: This feature changes only the string representation (`to_string_repr()`), not the storage format, serialization, or schema.

**Fields** (unchanged):

| Field | Type | Description |
|---|---|---|
| `Duration` | `chrono::Duration` | Signed duration with nanosecond precision. Serialized as-is via serde. |

**Display Format** (changed):

| Duration Range | Old Format | New Format | Example |
|---|---|---|---|
| >= 86,400s | `{n}s` | `{d}d {h}h {m}m` | `14d` |
| >= 3,600s | `{n}s` | `{h}h {m}m {s}s` | `2h 15m 30s` |
| >= 60s | `{n}s` | `{m}m {s}s` | `5m 30s` |
| < 60s | `{n}s` | `{s}s` | `45s` |
| Negative | `{n}s` | `-{format}` | `-5d` |
| Zero | `0s` | `0s` | `0s` |

**Validation Rules** (unchanged):
- Duration is always a valid `chrono::Duration` value (invariant guaranteed by Rust type system).
- The serialized form stored in SQLite is produced by serde, not by `to_string_repr()`, so storage is unaffected by this change.

## Downstream Consumers

| Consumer | Path | Impact |
|---|---|---|
| Node ID hashing | `node.rs:63,74` | Hash input changes → new node IDs for Duration values |
| CLI status (JSON) | `status.rs:25` | `"value"` JSON field changes format |
| CLI status (text) | `status.rs:58` | Stale node display changes format |
| CLI graph | `graph.rs:19,41,64` | Mermaid, DOT, and text graph views change format |
| CLI query | `query.rs:22` | Displays via MCP handler JSON |
| MCP `query_provenance` | `handlers.rs:227` | `"value"` in lineage steps changes format |
| MCP `list_stale_values` | `handlers.rs:261` | `"value"` in stale node list changes format |

## State Transitions

No state transitions are introduced or modified by this feature. Duration nodes are immutable once stored.
