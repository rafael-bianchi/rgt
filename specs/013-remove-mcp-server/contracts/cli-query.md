# CLI Query Contract (post-MCP removal)

**Feature**: `013-remove-mcp-server` | **Date**: 2026-08-08

## Purpose

Define the `rgt query` interface after MCP removal. The command reads the database directly via `src/query/` — no MCP protocol involved.

## Command

```
rgt query <node_id> [--json]
```

## Output (text mode)

```
Node: <node_id> (Kind: <value_kind>, Value: <value>)
  └─ Source: <file_path>:<line_number>  [if root node]
  └─ Derived via <operation> from:      [if derived node]
      ├─ parent <id> (Kind: ..., Value: ...)
      └─ parent <id> (Kind: ..., Value: ...)
```

## Output (JSON mode, `--json`)

```json
{
  "node": {
    "id": "node_drv_abc",
    "value_kind": "NUMBER",
    "value": "300",
    "is_stale": false,
    "node_type": "DERIVED"
  },
  "lineage_steps": [
    {
      "node_id": "node_raw_def",
      "value": "100",
      "value_kind": "NUMBER",
      "node_type": "ROOT",
      "source_file": "config.json",
      "line_number": 5
    }
  ],
  "total_ancestors": 2
}
```

## Exit Codes

| Code | Condition |
|---|---|
| 0 | Query succeeded (node found, lineage printed) |
| 1 | Node not found in database |
| 2 | Database error |

## Verification

Run `rgt query <node_id>` on a node with known lineage. Output must match the JSON structure above. Existing integration tests in `test_derivation_chain.rs` assert on the lineage structure — these pass after API migration.

## Related Commands

| Command | Data source |
|---|---|
| `rgt query <id>` | `src/query/mod.rs` → DB |
| `rgt status` | `src/query/mod.rs` + `src/graph/invalidation.rs` → DB |
| `rgt verify` | `src/verify/mod.rs` → DB |
| `rgt graph` | DB direct (unchanged) |
| `rgt hook` | stdin (unchanged) |
