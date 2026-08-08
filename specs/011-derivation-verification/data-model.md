# Data Model: Derivation Verification

**Feature**: `011-derivation-verification` | **Date**: 2026-08-08

## Overview

No database schema changes. Verification is a read-only check against existing parent nodes. The `rgt verify` CLI loads parent values, computes expected result, and exits with a status code — no state is written.

## Entities

### Verification Request (CLI arguments)

| Field | Type | Description |
|---|---|---|
| `--parents` | `Vec<String>` | Comma-separated node IDs in variable order |
| `--operation` | `String` | `EXPRESSION` or `DATE_DIFF` |
| `--expression` | `String` (optional) | Formula string for EXPRESSION |
| `--result` | `f64` | Caller-claimed result to verify |

### Verification Result (exit code + stderr)

Success (exit 0): no stdout, no stderr.

Failure — mismatch (exit 1):
```
Error: verification failed for EXPRESSION "a + b"
  expected: 300
  provided: 500
  hint: retry with --result 300
```

Failure — invalid input (exit 2):
```
Error: parent node 'xyz' not found in database
```

### Operation Mapping

| Operation | Computation | Input Types |
|---|---|---|
| `EXPRESSION` | `evalexpr(expr, {a: val[0], b: val[1], ...})` | Number parents |
| `DATE_DIFF` | `date_diff(parent[0], parent[1])` | Date parents |
| Unknown | No computation (exit 0, warning on stderr) | Any |

### Variable Binding (EXPRESSION)

```
parent_ids[0] → a → parent_value[0]
parent_ids[1] → b → parent_value[1]
parent_ids[2] → c → parent_value[2]
...
```

## Downstream Consumers

| Consumer | Impact |
|---|---|
| `rgt verify` CLI | Primary entry point — parses args, opens DB, calls verify module, exits |
| Hook scripts (Claude Code, Cursor, etc.) | Thin delegates that call `rgt verify` and translate exit code to agent JSON |
| `date_diff()` in `src/types/value.rs` | Wired into DATE_DIFF path; `#[allow(dead_code)]` removed |
