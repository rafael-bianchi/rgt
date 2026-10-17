# Research: Fix Provenance Query Traversal Depth

**Feature Branch**: `005-fix-provenance-query-depth` | **Date**: 2026-08-01

## Purpose

Resolve architectural and design unknowns for fixing `handle_query_provenance` in `src/mcp/handlers.rs`, which currently only traverses one level of ancestor nodes.

## Research Questions

### RQ-1: BFS vs DFS traversal for provenance lineage queries

**Decision**: Breadth-first search (BFS) using `std::collections::VecDeque`.

**Rationale**: BFS processes nodes level-by-level (immediate parents first, then grandparents), producing a lineage output where closer ancestors appear before distant ones. For provenance queries where the user/agent reads from the queried node outward, BFS ordering naturally matches how one reasons about derivation chains. Additionally, BFS with a `visited` set naturally handles diamond dependencies (shared ancestors) without duplicate entries — each unique node is visited exactly once in the order of its shortest-path distance from the queried node.

**Alternatives considered**:

| Alternative | Why Rejected |
|---|---|
| DFS (stack-based recursion) | Recursion risks stack overflow for chains >10,000 nodes. Manual stack with `Vec` adds complexity without ordering benefit for lineage. |
| SQL recursive CTE | Requires a recursive `WITH RECURSIVE` query that would: (a) couple traversal logic to SQLite, (b) make cycle detection harder, (c) lose Rust-level type safety for node/value processing. Simpler to stay in Rust where the node struct is already available. |
| `petgraph` graph traversal | RGT already depends on `petgraph` for DAG operations, but the provenance graph is persisted in SQLite, not in a `petgraph::DiGraph` in memory. Loading the entire graph into `petgraph` for each query would be O(N) memory overhead for large graphs. BFS over SQL-queried edges keeps memory O(depth). |

---

### RQ-2: Cycle guard implementation

**Decision**: `std::collections::HashSet` of visited node IDs.

**Rationale**: A `HashSet` provides O(1) amortized membership checks, ensuring the BFS traversal doesn't revisit nodes. This is a correctness requirement (Constitution Principle III: graph correctness) even though the provenance graph is expected to be a DAG. Cyclic edges could be introduced by bugs, data corruption, or concurrent modifications. The `visited` set is zero-cost in the common case (DAG with no cycles) and provides an essential safety net.

**Alternatives considered**:

| Alternative | Why Rejected |
|---|---|
| Ignore cycles (trust DAG invariant) | Violates defensive programming. An infinite loop in the MCP server blocks all agent interactions with no recovery mechanism. |
| Depth-limit cap instead of visited set | A fixed depth cap (e.g., 10,000) doesn't protect against cycles within the limit and introduces arbitrary behavior for deep-but-valid chains. |
| `BTreeSet` | Ordered, but O(log n) insertion/lookup vs O(1) for `HashSet`. Provenance queries don't benefit from sorted order in the visited tracker. |

---

### RQ-3: Backward-compatible response schema changes

**Decision**: Add `total_ancestors` (number) and `node_type` (string) fields to the response without removing or renaming existing fields.

**Rationale**: The existing `query_provenance` response schema is:
```json
{
  "node_id": "...",
  "is_stale": false,
  "lineage_steps": [
    { "node_id": "...", "value_kind": "...", "value": "...", "is_stale": false, "parent_ids": [...], "line_number": null }
  ]
}
```

New fields (`total_ancestors`, `node_type` in each lineage step, and `stale_reason` in parent step entries) are additive — existing consumers that ignore unknown fields continue to work. The `lineage_steps` array already exists and its entries are extended in-place without restructuring.

**Alternatives considered**:

| Alternative | Why Rejected |
|---|---|
| New top-level `lineage_tree` field replacing `lineage_steps` | Breaking change for any existing consumer. Spec requires backward compatibility (FR-007). |
| Separate `total_ancestors` endpoint | Unnecessary API surface increase for a single integer. Simpler to include in the existing response. |

---

### RQ-4: Handling missing parent nodes (dangling edges)

**Decision**: Skip missing nodes silently during BFS traversal.

**Rationale**: If a derivation edge references a `parent_node_id` that no longer exists in the `tracked_nodes` table (e.g., due to manual cleanup or a bug), the BFS should continue traversing other branches rather than failing the entire query. This is consistent with the edge case specification and Principle III's requirement for correctness under unexpected graph states. The missing node is simply not enqueued for further traversal.

**Implementation**: `get_tracked_node` returns `Option<TrackedNode>`. If `None`, skip and continue the BFS loop for remaining queue entries.

---

### RQ-5: Performance characteristics of BFS with SQL queries

**Decision**: Acceptable. Each node in the BFS queue triggers two SQL queries (`get_tracked_node` + `get_parent_edges`), which are indexed lookups on `id` and `child_node_id` columns respectively.

**Rationale**: The spec requires <200ms for 1,000-node chains. With WAL-mode SQLite and an SSD, single-row indexed lookups are ~0.1-0.5ms. For a 1,000-node chain, this means 2,000 sequential queries at ~0.1ms each = ~200ms worst case. This meets the budget. For typical shallow graphs (<100 nodes), the query time is dominated by the first few lookups and remains well under 10ms.

**Optimization opportunities (out of scope for this fix)**: A single recursive CTE or batched `IN (...)` query could reduce round-trips, but introduces complexity not warranted for the expected graph sizes.

---

### RQ-6: Testing strategy

**Decision**: Extend existing `tests/integration/test_derivation_chain.rs` with multi-level chain tests and add unit-style tests for cycle and edge cases.

**Rationale**: The existing test file already has `test_multi_level_derivation_chain_query` which creates a 3-node chain. This test currently expects ≥2 steps (which the current code provides) — after the fix, it should assert exactly the expected number of steps (4: queried node + 3 ancestors including root). Additional tests will cover: root-only queries, branching chains, diamond dependencies, cycles, and dangling edges.

**Test plan**:

| Test | Scope | What it validates |
|---|---|---|
| `test_multi_level_derivation_chain_query` (updated) | Integration | 3-level chain returns all 4 nodes |
| `test_deep_linear_chain` (new) | Integration | 10-node chain returns all 11 lineage steps |
| `test_branching_chain` (new) | Integration | Diamond-shaped graph returns all nodes without duplication |
| `test_root_node_query` (new) | Integration | Root node returns only itself with empty parents |
| `test_cycle_detection` (new) | Unit/Integration | Self-loop terminates without infinite recursion |

---

## Summary of Decisions

| ID | Decision | Rationale |
|---|---|---|
| D-001 | BFS traversal with `VecDeque` | Level-ordered output, no recursion depth risk, handles diamonds naturally |
| D-002 | `HashSet` visited set for cycle guard | O(1) lookup, zero-cost for correct DAGs, essential safety net |
| D-003 | Additive schema changes only | Backward compatibility (FR-007) |
| D-004 | Skip dangling edges silently | Resilient to data corruption, non-blocking for valid branches |
| D-005 | No new external dependencies | `VecDeque` and `HashSet` from `std::collections`, zero build impact |
| D-006 | Extend existing test file | Avoids test file proliferation, uses existing test fixtures |
