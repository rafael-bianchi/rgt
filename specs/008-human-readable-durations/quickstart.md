# Quickstart: Human-Readable Duration Display

**Feature**: `008-human-readable-durations` | **Date**: 2026-08-01

## Prerequisites

- Rust toolchain installed (`rustc`, `cargo`)
- Repository cloned and on the feature branch
- `rgt` built with `cargo build`

## Validation Scenarios

### Scenario 1: Multi-Day Duration via MCP

**Goal**: Verify that a 14-day duration displays as `14d 0h 0m` through MCP.

**Steps**:
1. Start the MCP server: `rgt mcp` (runs in foreground, use another terminal)
2. Record two date values:
   ```json
   {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"record_value","arguments":{"file_path":"test/example.md","value_kind":"DATE","date_value":"2026-01-01T00:00:00Z","line_number":1}}}
   ```
   ```json
   {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"record_value","arguments":{"file_path":"test/example.md","value_kind":"DATE","date_value":"2026-01-15T00:00:00Z","line_number":2}}}
   ```
3. Derive the duration between them:
   ```json
   {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"record_derivation","arguments":{"parent_node_ids":["<node1_id>","<node2_id>"],"value_kind":"DURATION","duration_seconds":1209600,"operation_type":"DATE_DIFF","expression":"date2 - date1"}}}
   ```
4. Query the derivation node:
   ```json
   {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"query_provenance","arguments":{"node_id":"<derived_node_id>"}}}
   ```

**Expected**: The `lineage_steps[].value` for the Duration node shows `14d 0h 0m`, not `1209600s`.

### Scenario 2: Negative Duration

**Goal**: Verify that a negative duration displays with `-` prefix.

**Steps**:
1. Derive a duration with date order reversed (earlier date second):
   ```json
   {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"record_derivation","arguments":{"parent_node_ids":["<later_date_node>","<earlier_date_node>"],"value_kind":"DURATION","duration_seconds":-432000,"operation_type":"DATE_DIFF","expression":"earlier - later"}}}
   ```
2. Query or check via `rgt status`.

**Expected**: The displayed value begins with `-` (e.g., `-5d 0h 0m`).

### Scenario 3: Sub-Minute Duration

**Goal**: Verify that durations under 60 seconds still display correctly.

**Steps**:
1. Derive a duration of 45 seconds.
2. Query the node.

**Expected**: The displayed value is `45s`.

### Scenario 4: CLI Status Output

**Goal**: Verify `rgt status` uses the new format.

**Steps**:
```bash
cargo run -- status
cargo run -- status --json
```

**Expected**: Any Duration values in the output use human-readable format in both JSON and text modes.

### Scenario 5: Unit Tests

**Goal**: Verify that all existing and new tests pass.

**Steps**:
```bash
cargo test --all -- --test-threads=1
```

**Expected**: All tests pass. `test_date_diff` includes new assertions on human-readable format.

### Scenario 6: Doc Comment Verification

**Goal**: Verify the `date_diff()` doc comment exists and documents the year/month limitation.

**Steps**:
```bash
cargo doc --no-deps --open
# Or inspect src/types/value.rs directly
```

**Expected**: The doc comment on `date_diff()` explicitly mentions that `chrono::Duration` has no `num_years()` or `num_months()` methods and that callers needing year/month display should use calendar-aware methods on original Date nodes.
