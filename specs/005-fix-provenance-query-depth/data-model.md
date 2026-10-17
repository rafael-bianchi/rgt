# Data Model: Fix Provenance Query Traversal Depth

**Feature Branch**: `005-fix-provenance-query-depth` | **Date**: 2026-08-01

## Overview

This feature fixes the `handle_query_provenance` BFS traversal logic. **No schema changes are required** — the existing `source_documents`, `tracked_nodes`, and `derivation_edges` tables already store the complete graph. The fix is purely in the traversal algorithm that reads from these tables.

## Entities (Existing — No Changes)

### TrackedNode (`tracked_nodes` table)

| Column | Type | Description |
|---|---|---|
| `id` | TEXT (PK) | Unique node identifier (hash-based or sequential) |
| `node_type` | TEXT | `Root` (read from source file) or `Derived` (computed) |
| `value_kind` | TEXT | `Number`, `Date`, or `Duration` |
| `value_number` | REAL | Numeric value (null for non-number nodes) |
| `value_text` | TEXT | String representation (null for number-only nodes) |
| `source_doc_id` | TEXT (FK) | Link to `source_documents.id` (null for Derived nodes) |
| `line_number` | INTEGER | Source file line number (null for Derived nodes) |
| `is_stale` | INTEGER | 0 = fresh, 1 = stale (invalidated via cascade) |
| `stale_reason` | TEXT | Reason string (null if fresh) |
| `created_at` | TEXT | ISO-8601 timestamp |
| `updated_at` | TEXT | ISO-8601 timestamp |

**Query function used by BFS traversal**: `get_tracked_node(conn, node_id) → Result<Option<TrackedNode>>`

### DerivationEdge (`derivation_edges` table)

| Column | Type | Description |
|---|---|---|
| `id` | TEXT (PK) | Unique edge identifier |
| `parent_node_id` | TEXT (FK) | References `tracked_nodes.id` |
| `child_node_id` | TEXT (FK) | References `tracked_nodes.id` |
| `operation_type` | TEXT | e.g., `multiply`, `add`, `date_diff` |
| `expression` | TEXT | Optional human-readable expression string |

**Query function used by BFS traversal**: `get_parent_edges(conn, child_node_id) → Result<Vec<DerivationEdge>>`

### SourceDocument (`source_documents` table)

Not directly queried during BFS traversal, but referenced by Root nodes via `source_doc_id`. Source file info (path, line_number) is propagated through Root `TrackedNode` entries in the lineage output.

## Traversal Algorithm (New)

The BFS traversal replaces the single-level loop (`handlers.rs:209-218`) with the following logic:

```
function bfsLineage(nodeId):
    visited = HashSet<String>
    queue = VecDeque<String>
    queue.push_back(nodeId)
    lineageSteps = []

    while queue is not empty:
        currentId = queue.pop_front()
        if visited.contains(currentId): continue
        visited.insert(currentId)

        node = get_tracked_node(currentId)
        if node is None: continue  // dangling edge — skip

        parentEdges = get_parent_edges(currentId)
        parentIds = [edge.parent_node_id for edge in parentEdges]

        lineageSteps.push({
            node_id: node.id,
            value_kind: node.value_kind,
            value: node.value,
            is_stale: node.is_stale,
            stale_reason: node.stale_reason,
            parent_ids: parentIds,
            line_number: node.line_number,
            node_type: node.node_type,
        })

        for pid in parentIds:
            queue.push_back(pid)

    totalAncestors = visited.len() - 1  // exclude queried node itself
    return { node_id: nodeId, is_stale: ..., lineage_steps: lineageSteps, total_ancestors: totalAncestors }
```

### Key Properties

- **Determinism**: Queue FIFO ordering + stable HashSet iteration-free output means identical inputs produce identical outputs.
- **Diamond handling**: Shared ancestors are visited once; `visited` set prevents duplication.
- **Dangling edges**: Missing nodes (deleted from DB) are silently skipped.
- **Root nodes**: When `parentEdges` is empty, `parentIds` is `[]`, and the node is the last entry in the queue (no new enqueues). Traversal terminates naturally.
- **Cycles**: Self-loops (A→A) are caught by `visited.contains()` and skipped; cross-node cycles (A→B→A) produce complete-but-bounded output.

## Response Schema (Updated)

```json
{
  "node_id": "abc123...",
  "is_stale": false,
  "total_ancestors": 3,
  "lineage_steps": [
    {
      "node_id": "abc123...",
      "value_kind": "Number",
      "value": "168.0",
      "is_stale": false,
      "stale_reason": null,
      "parent_ids": ["def456..."],
      "line_number": null,
      "node_type": "Derived"
    },
    {
      "node_id": "def456...",
      "value_kind": "Number",
      "value": "84.0",
      "is_stale": false,
      "stale_reason": null,
      "parent_ids": ["ghi789..."],
      "line_number": null,
      "node_type": "Derived"
    },
    {
      "node_id": "ghi789...",
      "value_kind": "Number",
      "value": "42.0",
      "is_stale": false,
      "stale_reason": null,
      "parent_ids": [],
      "line_number": 10,
      "node_type": "Root"
    }
  ]
}
```

### Changes from Current Schema

| Field | Before | After |
|---|---|---|
| `total_ancestors` | Not present | Added: count of unique ancestors (excl. queried node) |
| `lineage_steps[].node_type` | Not present | Added: `"Root"` or `"Derived"` |
| `lineage_steps[].stale_reason` | Only on queried node | Present on all lineage steps |
| `lineage_steps[].line_number` | Only on parent nodes | Present on queried node too (when applicable) |
| `lineage_steps[].parent_ids` | Only on queried node | Present on all lineage steps |

All changes are **additive** — no existing field removed or renamed.
