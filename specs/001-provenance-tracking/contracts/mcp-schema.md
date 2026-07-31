# MCP Interface Contract: `rgt mcp`

**Feature Branch**: `001-provenance-tracking` | **Date**: 2026-07-31

## Protocol Overview

`rgt mcp` implements the official Model Context Protocol (MCP) specification over standard I/O streams using JSON-RPC 2.0.

---

## Exposed MCP Tools

### 1. `record_value`

Registers a root value (number or date) read directly from a source file.

#### Parameters Schema (`object`)
- `file_path` (string, required): Absolute or repo-relative path to source file.
- `value_kind` (string, required): `'NUMBER'` or `'DATE'`.
- `number_value` (number, optional): Numeric value if `value_kind` is `'NUMBER'`.
- `date_value` (string, optional): ISO-8601 date string if `value_kind` is `'DATE'`.
- `line_number` (integer, optional): 1-indexed line number in source file.

#### Return Output
```json
{
  "node_id": "node_raw_8f9a2b1c",
  "is_new": true,
  "is_stale": false
}
```

---

### 2. `record_derivation`

Registers a new value derived from one or more parent value nodes.

#### Parameters Schema (`object`)
- `parent_node_ids` (array of strings, required): Parent node IDs backing this derivation.
- `value_kind` (string, required): `'NUMBER'`, `'DATE'`, or `'DURATION'`.
- `number_value` (number, optional): Resulting numeric value.
- `date_value` (string, optional): Resulting ISO-8601 date string.
- `duration_seconds` (integer, optional): Resulting duration in seconds (for date-diffs).
- `operation_type` (string, required): `'ADD'`, `'SUBTRACT'`, `'MULTIPLY'`, `'DIVIDE'`, `'DATE_DIFF'`, or `'TRANSFORM'`.
- `expression` (string, optional): Human-readable formula or description (e.g. `"EndDate - StartDate"`).

#### Return Output
```json
{
  "node_id": "node_drv_3c4d5e6f",
  "is_stale": false,
  "parent_count": 2
}
```

---

### 3. `query_provenance`

Retrieves the complete derivation chain and source files for a target node ID ("Why is this value X?").

#### Parameters Schema (`object`)
- `node_id` (string, required): Target node ID to query.

#### Return Output
```json
{
  "node_id": "node_drv_3c4d5e6f",
  "value_kind": "DURATION",
  "duration_seconds": 864000,
  "is_stale": false,
  "lineage_steps": [
    {
      "node_id": "node_drv_3c4d5e6f",
      "operation": "DATE_DIFF",
      "expression": "EndDate - StartDate",
      "parent_ids": ["node_raw_111", "node_raw_222"]
    },
    {
      "node_id": "node_raw_111",
      "source_file": "docs/schedule.md",
      "line_number": 14,
      "date_value": "2026-08-01T00:00:00Z"
    },
    {
      "node_id": "node_raw_222",
      "source_file": "docs/schedule.md",
      "line_number": 15,
      "date_value": "2026-08-11T00:00:00Z"
    }
  ]
}
```

---

### 4. `list_stale_values`

Lists all recorded nodes currently marked as stale along with the modified source files that invalidated them.

#### Parameters Schema (`object`)
- None required.

#### Return Output
```json
{
  "stale_nodes": [
    {
      "node_id": "node_drv_3c4d5e6f",
      "stale_reason": "PARENT_NODE_STALE",
      "invalidated_by_source": "docs/schedule.md"
    }
  ]
}
```
