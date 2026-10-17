# Feature Specification: Fix Provenance Query Traversal Depth

**Feature Branch**: `005-fix-provenance-query-depth`

**Created**: 2026-08-01

**Status**: Draft

**Input**: User description: "bug: handle_query_provenance only walks one level of ancestors. For any derivation chain deeper than two levels (e.g. source_file → A → B → C), querying node C will only return B, omitting A and the original source file. Replace the single-level loop with a BFS queue traversal."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Full lineage query for deep derivation chains (Priority: P1)

An AI coding agent uses `query_provenance` to ask "why is this number X" and receives the complete derivation lineage back to the original source files, regardless of how many derivation steps the value went through.

**Why this priority**: This is the core value proposition of RGT's provenance tracking — agents need the full chain to reason correctly about data lineage. Without this fix, the feature silently returns incomplete results.

**Independent Test**: Can be fully tested by setting up a 3-level derivation chain (root → derived1 → derived2), querying the deepest node, and verifying all three levels plus the source file reference appear in the response with the correct `total_ancestors` count.

**Acceptance Scenarios**:

1. **Given** a provenance graph with chain `file.md:10 → node_A (42.0) → node_B (84.0) → node_C (168.0)`, **When** the agent queries provenance for `node_C`, **Then** the response lineage includes `node_C`, `node_B`, `node_A`, and the source file node, with `total_ancestors: 3`.
2. **Given** a provenance graph with a single root node (no parents), **When** the agent queries provenance for that node, **Then** the response lineage includes only that node with an empty parent list and `total_ancestors: 0`.
3. **Given** a provenance graph with branching (one derived node has two independent parent chains), **When** the agent queries the derived node, **Then** all ancestors from both parent chains are included in the traversal without duplication.

---

### User Story 2 - Resilience against graph cycles (Priority: P2)

If the provenance graph contains an unexpected cycle (e.g., due to a bug or data corruption), the query handler terminates gracefully without entering an infinite loop.

**Why this priority**: An infinite loop in the query handler would freeze the MCP server, blocking all subsequent agent interactions. While cycles shouldn't occur in a correct DAG, the system must be defensive.

**Independent Test**: Can be tested by manually inserting a self-referencing edge in the database and verifying the query returns successfully (rather than looping) with the visited nodes listed.

**Acceptance Scenarios**:

1. **Given** a provenance graph with a cycle (node A → node B → node A), **When** the agent queries provenance for node B, **Then** the response returns lineage steps for both nodes without entering an infinite loop, and the operation completes under the latency budget.
2. **Given** a provenance graph with a self-loop (node A → node A), **When** the agent queries provenance for node A, **Then** the handler returns immediately with just node A's information.

---

### User Story 3 - Performance under large graphs (Priority: P3)

Agents can query provenance on derivation chains up to 1,000 nodes deep without noticeable delay, maintaining RGT's sub-second interactive response guarantees.

**Why this priority**: Graph response time matters for agent UX, but extremely deep chains (>100 nodes) are rare in practice. This ensures the BFS implementation doesn't introduce unacceptable performance regressions.

**Independent Test**: Can be tested by generating a synthetic linear chain of 1,000 nodes and measuring the total query wall-clock time.

**Acceptance Scenarios**:

1. **Given** a linear derivation chain of 1,000 nodes, **When** the agent queries provenance for the deepest node, **Then** the BFS traversal completes in under 200ms with all 1,000 ancestors returned correctly.
2. **Given** a provenance graph with 10,000 total nodes (shallow branching), **When** the agent queries provenance for a node with 5 ancestors, **Then** the response still returns within 5ms.

---

### Edge Cases

- What happens when a node has no parents (root/source node)? The query returns only the queried node with an empty `parent_ids` array and `total_ancestors: 0`.
- What happens when an ancestor node was deleted from the database (dangling edge)? The BFS traversal skips the missing node gracefully without failing the entire query.
- What happens when the queried node ID doesn't exist? The handler returns a descriptive error (existing behavior, unchanged).
- What happens with very deep chains (>10,000 nodes)? The BFS completes within the latency budget via the `visited` set preventing O(K²) behavior.
- What happens with a diamond dependency (A → B → D and A → C → D)? Node D's query returns all four nodes without duplication via the `visited` set.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST traverse all ancestor nodes in a provenance chain back to the original source file nodes using breadth-first (BFS) or depth-first (DFS) traversal.
- **FR-002**: System MUST include a cycle guard (`visited` set) to prevent infinite loops when encountering unexpected graph cycles or self-loops.
- **FR-003**: System MUST include a `total_ancestors` field in the query response indicating the total number of unique ancestor nodes traversed.
- **FR-004**: System MUST include `node_type` and `line_number` fields in each lineage step entry to distinguish root (source file) nodes from derived nodes.
- **FR-005**: System MUST return results deterministically — repeated queries for the same node produce identical lineage outputs.
- **FR-006**: System MUST gracefully handle dangling edges (parent node not found in DB) by skipping that branch without failing the entire query.
- **FR-007**: System MUST maintain backward compatibility with the existing `query_provenance` MCP response schema (`node_id`, `is_stale`, `lineage_steps`).

### Key Entities

- **TrackedNode**: A node in the provenance DAG. Key attributes: `id`, `node_type` (Root or Derived), `value_kind`, `value`, `line_number`, `is_stale`. Root nodes link to a source document; derived nodes link to parent nodes via derivation edges.
- **DerivationEdge**: A directed edge connecting a parent node to a child derived node. Key attributes: `parent_node_id`, `child_node_id`, `operation_type`, optional `expression`.
- **LineageStep**: A serialized entry in the query response. Key attributes: `node_id`, `value_kind`, `value`, `is_stale`, `stale_reason`, `parent_ids`, `line_number`, `node_type`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Querying a node at depth N returns all N ancestor levels back to the original root source node(s) with 100% accuracy for chains up to 1,000 levels.
- **SC-002**: The `total_ancestors` field in the response correctly reflects the total count of unique ancestor nodes for all tested chain configurations (linear, branching, diamond).
- **SC-003**: Full BFS traversal for a 1,000-node linear chain completes in under 200ms on standard developer hardware.
- **SC-004**: Queries for single-root nodes (0 ancestors) return in under 1ms, maintaining parity with the existing single-level implementation.
- **SC-005**: A purposefully-inserted cycle (node A → node B → node A) does not cause an infinite loop; the query returns within 100ms (effectively instantaneous since only 2 nodes are traversed before the visited set terminates the BFS).

## Assumptions

- The provenance graph stored in SQLite (`.rgt/store.db`) is assumed to be a DAG under normal operation; cycles are defensive edge cases.
- The `get_tracked_node` and `get_parent_edges` store query functions already exist and return correct results for valid IDs.
- The MCP server uses synchronous handlers (no async runtime); BFS is performed synchronously.
- Standard developer hardware is defined as a macOS/Linux machine with an SSD and at least 8GB RAM, representative of the target developer audience.
- The existing integration tests in `tests/integration/test_derivation_chain.rs` cover the current single-level behavior and will be extended for multi-level chains.
- Backward compatibility with the existing `query_provenance` JSON-RPC response schema must be maintained — only new fields (`total_ancestors`, `node_type`) may be added, not removed.
