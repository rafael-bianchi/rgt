# MCP Contract: query_provenance (Updated)

**Feature Branch**: `005-fix-provenance-query-depth` | **Date**: 2026-08-01

## Overview

This contract supersedes the `query_provenance` definition in the 001-provenance-tracking MCP contract. The tool name and request parameters are unchanged. Response schema gains additive fields for complete lineage traversal.

## Tool: `query_provenance`

**Description**: Query the complete derivation lineage ("Why is this value X?") for a given tracked node. Returns ALL ancestor nodes traversed via BFS, not just immediate parents.

### Request

```jsonc
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "query_provenance",
    "arguments": {
      "node_id": "abc123def456..."  // string, required: ID of the tracked node to trace
    }
  }
}
```

### Response (Success — Deep Chain Example)

```jsonc
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"node_id\":\"hash_C...\",\"is_stale\":false,\"total_ancestors\":3,\"lineage_steps\":[...]}"
      }
    ]
  }
}
```

The `text` field contains a JSON string with the following structure:

| Field | Type | Description |
|---|---|---|
| `node_id` | string | ID of the queried node |
| `is_stale` | boolean | Whether the queried node is stale |
| `total_ancestors` | integer **(NEW)** | Total count of unique ancestor nodes traversed (excludes the queried node) |
| `lineage_steps` | array | Ordered list (BFS level-order) of nodes from queried node back to roots |

Each `lineage_steps` entry:

| Field | Type | Description | New? |
|---|---|---|---|
| `node_id` | string | Node identifier | Existing |
| `value_kind` | string | `"Number"`, `"Date"`, or `"Duration"` | Existing |
| `value` | string | Human-readable value representation | Existing |
| `is_stale` | boolean | Staleness flag | Existing |
| `stale_reason` | string\|null | Reason for staleness if applicable | **Now on all steps** (was only first) |
| `parent_ids` | string[] | IDs of this node's direct parents | **Now on all steps** (was only first) |
| `line_number` | integer\|null | Source line for Root nodes | **Now on all steps** (was only parents) |
| `node_type` | string **(NEW)** | `"Root"` or `"Derived"` | Yes |

### Response (Single Root Node)

```json
{
  "node_id": "root_A...",
  "is_stale": false,
  "total_ancestors": 0,
  "lineage_steps": [
    {
      "node_id": "root_A...",
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

### Response (Error — Node Not Found)

```jsonc
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Node not found: nonexistent_id"
  }
}
```

### Response (Cycle Guard)

If the traversal encounters a cycle (e.g., A→B→A), the BFS terminates naturally via the `visited` set. The response includes all nodes visited before the cycle was detected. No error is returned — cycles are handled defensively.

## Backward Compatibility

- All existing fields (`node_id`, `is_stale`, `lineage_steps`) are preserved with identical types.
- New fields (`total_ancestors`, `node_type`, `stale_reason` expansion) are additive.
- Clients that ignore unknown JSON fields continue to work without modification.
- The `lineage_steps` array length increases from [1 + immediate_parents] to [1 + all_ancestors], which is the desired fix.

## Contract Test

Test file: `tests/contract/test_mcp_contract.rs`

The existing contract test for `query_provenance` validates:
1. Request/response JSON-RPC envelope format
2. Successful query returns `node_id`, `is_stale`, `lineage_steps` fields

Updated contract test should additionally validate:
1. `total_ancestors` field present in response
2. `node_type` field present in each lineage step
3. Multi-level chain returns ancestor count > immediate parent count
