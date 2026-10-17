# Duration Format Contract

**Feature**: `008-human-readable-durations` | **Date**: 2026-08-01

## Overview

All `ValueData::Duration` values displayed through RGT interfaces (MCP, CLI) MUST use the human-readable format defined below. This replaces the previous raw-seconds-only format.

## Format Specification

### Canonical Format

```
[sign]{value}{unit} {value}{unit} ...
```

- `sign`: `-` if the duration is negative, empty string otherwise.
- `value`: Non-negative integer.
- `unit`: One of `d` (days), `h` (hours), `m` (minutes), `s` (seconds).

### Unit Thresholds

| Threshold | Units Displayed | Example |
|---|---|---|
| >= 86,400s (1 day) | `d`, `h`, `m` | `14d 0h 0m` |
| >= 3,600s (1 hour) | `h`, `m`, `s` | `2h 15m 30s` |
| >= 60s (1 minute) | `m`, `s` | `5m 30s` |
| < 60s | `s` only | `45s` |
| Zero | `0s` | `0s` |

### Trailing Zero Rules

- Trailing zero units are **omitted**: `14d` (not `14d 0h 0m`)
- Middle zero units are **preserved**: `1h 0m 5s`

### Negative Duration Rule

- When `num_seconds() < 0`: prepend `-` and format the absolute value. Example: `-5d 0h 0m`.

## Affected Interfaces

### MCP `record_derivation` — Return Output

```json
{
  "node_id": "node_drv_3c4d5e6f",
  "is_stale": false,
  "parent_count": 2
}
```

*No change* — `record_derivation` returns only metadata, not the formatted value.

### MCP `query_provenance` — `lineage_steps[].value`

**Before**:
```json
{"value": "1209600s", "value_kind": "DURATION"}
```

**After**:
```json
{"value": "14d 0h 0m", "value_kind": "DURATION"}
```

### MCP `list_stale_values` — `stale_nodes[].value`

**Before**:
```json
{"value": "1209600s", "value_kind": "DURATION"}
```

**After**:
```json
{"value": "14d 0h 0m", "value_kind": "DURATION"}
```

### CLI `rgt query` — value field

**Before**:
```
Step 1: Node [node_drv_abc] (Kind: DURATION, Value: 1209600s) -> Stale: false
```

**After**:
```
Step 1: Node [node_drv_abc] (Kind: DURATION, Value: 14d 0h 0m) -> Stale: false
```

### CLI `rgt status` — stale node list and JSON output

**Before** (JSON):
```json
{"value": "1209600s", "value_kind": "DURATION"}
```

**After** (JSON):
```json
{"value": "14d 0h 0m", "value_kind": "DURATION"}
```

### CLI `rgt graph` — mermaid, DOT, text

All three formats display values through `to_string_repr()` and are updated automatically.

## Compatibility

- **Storage**: The serialized form persisted to SQLite is produced by serde, not by `to_string_repr()`. Storage format is **unaffected**.
- **Node IDs**: Duration node IDs generated after this change will differ from those generated before, because `to_string_repr()` output is incorporated into BLAKE3 hashes. Users should regenerate their databases with `rgt init -g --force` after upgrading.
