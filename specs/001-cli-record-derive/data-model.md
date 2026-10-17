# Data Model: CLI Record & Derive Commands

**Feature**: 001-cli-record-derive

## Entities

All entities already exist in the database schema. This feature wires CLI commands to existing tables without schema changes.

### Root Node (table: `tracked_nodes`)

A value extracted from a data file by `rgt record` or passive hooks.

| Attribute | Type | Source |
|---|---|---|
| `id` | TEXT PK | `TrackedNode::generate_root_id(path, line, value)` → `node_raw_<blake3_12hex>` |
| `node_type` | TEXT | Always `ROOT` |
| `value_kind` | TEXT | `NUMBER` or `DATE` |
| `number_val` | REAL | Set for NUMBER; NULL for DATE |
| `date_val` | TEXT (RFC 3339) | Set for DATE; NULL for NUMBER |
| `duration_secs` | INTEGER | NULL (roots are never durations) |
| `source_doc_id` | INTEGER FK → source_documents.id | Links to the file the value was extracted from |
| `line_number` | INTEGER | 1-based line in source file |
| `is_stale` | INTEGER (0/1) | Initially 0; marked as 1 by `rgt status` cascade |
| `stale_reason` | TEXT | `SOURCE_FILE_MODIFIED` when file changed |
| `created_at` | TEXT (RFC 3339) | UTC timestamp of first insertion |
| `updated_at` | TEXT (RFC 3339) | UTC timestamp of last upsert |

**Identity**: Node ID is content-addressed — same file path + line + value always produces same ID. Changed values produce new IDs; old IDs remain stale.

**State transitions**:
```
Active (is_stale=0) → Stale (is_stale=1, reason="SOURCE_FILE_MODIFIED")
                     ↑ `rgt status` after file modified
Active ← Fresh        `rgt record` re-upserts idempotently
```

### Derived Node (table: `tracked_nodes`)

A computed value verified and recorded by `rgt derive`.

| Attribute | Type | Source |
|---|---|---|
| `id` | TEXT PK | `TrackedNode::generate_derived_id(parents, op, value)` → `node_drv_<blake3_12hex>` |
| `node_type` | TEXT | Always `DERIVED` |
| `value_kind` | TEXT | `NUMBER` (EXPRESSION) or `DURATION` (DATE_DIFF) |
| `number_val` | REAL | Result for EXPRESSION; NULL for DURATION |
| `date_val` | TEXT | NULL (derived nodes are never dates directly) |
| `duration_secs` | INTEGER | Seconds for DATE_DIFF; NULL for NUMBER |
| `source_doc_id` | NULL | Derived nodes have no source file |
| `line_number` | NULL | Derived nodes have no source line |
| `is_stale` | INTEGER (0/1) | Initially 0; marked stale by cascade when any parent becomes stale |
| `stale_reason` | TEXT | `PARENT_STALE` when cascaded |
| `created_at` | TEXT (RFC 3339) | UTC timestamp of insertion |
| `updated_at` | TEXT (RFC 3339) | UTC timestamp of last upsert |

**Identity**: Node ID is deterministic from sorted parent IDs + operation type + result value. Same parameters → same ID → no duplicates (ON CONFLICT DO NOTHING on edges).

**State transitions**:
```
Active (is_stale=0) → Stale (is_stale=1, reason="PARENT_STALE")
                     ↑ `rgt status` invalidation cascade: any parent stale → child stale
Active ← Fresh        `rgt derive` re-derives with same parameters (idempotent)
```

### Derivation Edge (table: `derivation_edges`)

A directed link from a parent node to a derived child node.

| Attribute | Type | Source |
|---|---|---|
| `id` | INTEGER PK AUTOINCREMENT | Auto-generated |
| `parent_node_id` | TEXT FK → tracked_nodes.id | One of the parent node IDs from `--parents` |
| `child_node_id` | TEXT FK → tracked_nodes.id | The derived node ID |
| `operation_type` | TEXT | `EXPRESSION` or `DATE_DIFF` |
| `expression` | TEXT | Formula string for EXPRESSION; NULL for DATE_DIFF |

**Constraints**: `UNIQUE(parent_node_id, child_node_id)` — prevents duplicate edges.

**Relationships**:
- Each derived node has N edges (one per parent)
- Each edge links exactly one parent to one child
- Reverse traversal: `get_parent_edges(child_id)` → all parents of a derived node
- Forward traversal: `get_child_edges(parent_id)` → all dependents of a value

### Source Document (table: `source_documents`)

File metadata tracked for staleness detection.

No changes to this entity. `rgt record` calls `upsert_source_document()` (same as passive hooks) which refreshes mtime, size, and BLAKE3 hash on each recording.

## Extraction Rules

`rgt record` uses the same `extract_values_from_content()` function as passive hooks:

- **Numbers**: Regex `\b(\d+(?:\.\d+)?)\b` — captures integers and decimals per line
- **Dates**: Regex `\b(\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?)?)\b` — captures ISO-8601 dates (date-only or datetime)
- **Value limit**: Maximum 10,000 values per file; excess silently dropped with stderr warning
