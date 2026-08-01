# Quickstart: Validating Provenance Query BFS Fix

**Feature Branch**: `005-fix-provenance-query-depth` | **Date**: 2026-08-01

## Prerequisites

- RGT built from source: `cargo build`
- No existing `.rgt/store.db` in the project root (start fresh)

## Setup: Build and Initialize

```bash
cargo build
rm -f .rgt/store.db
```

## Scenario 1: Multi-Level Derivation Chain Returns Complete Lineage

**Goal**: Verify that a 3-level chain (root → A → B → C) returns all 4 nodes when querying C.

**Steps**:

1. Seed the database with a linear chain:
   ```bash
   # Start MCP server in background
   cargo run -- mcp &
   MCP_PID=$!
   sleep 1
   ```

2. Record a root value from a source file:
   ```bash
   # Create a test file with a number
   echo "temperature: 42.5" > /tmp/test_source.txt

   # Send record_value via MCP (assumes test helper or direct JSON-RPC)
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"record_value","arguments":{"file_path":"/tmp/test_source.txt","value_kind":"NUMBER","number_value":42.5,"line_number":1}}}' | cargo run -- mcp
   # Capture node_id from response → ROOT_ID
   ```

3. Derive value A (multiply by 2):
   ```bash
   echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"record_derivation","arguments":{"parent_node_ids":["ROOT_ID"],"operation_type":"multiply","value_kind":"NUMBER","number_value":85.0,"expression":"42.5 * 2"}}}' | cargo run -- mcp
   # → DERIVED_A_ID
   ```

4. Derive value B (multiply by 2 again):
   ```bash
   echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"record_derivation","arguments":{"parent_node_ids":["DERIVED_A_ID"],"operation_type":"multiply","value_kind":"NUMBER","number_value":170.0,"expression":"85.0 * 2"}}}' | cargo run -- mcp
   # → DERIVED_B_ID
   ```

5. Query provenance for DERIVED_B_ID:
   ```bash
   echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"query_provenance","arguments":{"node_id":"DERIVED_B_ID"}}}' | cargo run -- mcp
   ```

**Expected outcome**:
- `total_ancestors` = 2 (DERIVED_A_ID and ROOT_ID)
- `lineage_steps` length = 3 (DERIVED_B, DERIVED_A, ROOT)
- First step has `node_type: "Derived"`, last step has `node_type: "Root"`
- Last step has `line_number: 1` and `parent_ids: []`

**Cleanup**:
```bash
kill $MCP_PID 2>/dev/null
```

---

## Scenario 2: Root Node Query Returns Self Only

**Goal**: Verify that querying a root node with no parents returns exactly one lineage step.

**Steps**:

1. Record a root value:
   ```bash
   # Using the test source file from Scenario 1
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"record_value","arguments":{"file_path":"/tmp/test_source.txt","value_kind":"DATE","date_value":"2026-08-01T12:00:00Z","line_number":2}}}' | cargo run -- mcp
   # Capture node_id from response → DATE_ROOT_ID
   ```

2. Query provenance for DATE_ROOT_ID:
   ```bash
   echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_provenance","arguments":{"node_id":"DATE_ROOT_ID"}}}' | cargo run -- mcp
   ```

**Expected outcome**:
- `total_ancestors` = 0
- `lineage_steps` length = 1
- Single step has `parent_ids: []`, `node_type: "Root"`

---

## Scenario 3: Run Automated Integration Tests

**Goal**: Execute the test suite to verify BFS traversal in CI-compatible form.

**Run**:
```bash
cargo test test_multi_level_derivation_chain_query
cargo test test_root_node_query
cargo test test_branching_chain
cargo test test_cycle_detection
```

**Expected outcome**: All tests pass. The updated `test_multi_level_derivation_chain_query` asserts exactly the expected number of lineage steps (not just ≥2).

---

## Scenario 4: Performance Verification (Optional)

**Goal**: Confirm BFS traversal meets latency budget for deep chains.

**Steps**:

1. Run the deep-chain performance test:
   ```bash
   cargo test test_deep_linear_chain_performance -- --nocapture
   ```

**Expected outcome**: Test reports traversal time <200ms for a 1,000-node chain in the test assertions.

---

## Scenario 5: Cycle Resilience (Defensive)

**Goal**: Confirm that the handler doesn't hang on unexpected graph cycles.

**Steps**:
```bash
cargo test test_cycle_detection -- --nocapture
```

**Expected outcome**: Test passes — handler returns without infinite loop. Response includes both nodes in the cycle without entering an infinite loop.

---

## Troubleshooting

| Issue | Fix |
|---|---|
| "Node not found" | Ensure the MCP server has access to the same `.rgt/store.db` used during recording |
| Test expects more steps than returned | The BFS traversal may not be recursing — check `visited` set and queue logic |
| BFS seems to skip a node | Verify `get_tracked_node` returns `Some` for that node ID (check DB directly with `sqlite3 .rgt/store.db`) |
