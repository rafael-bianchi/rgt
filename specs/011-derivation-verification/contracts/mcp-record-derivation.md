# MCP `record_derivation` Contract (with Verification)

**Feature**: `011-derivation-verification` | **Date**: 2026-08-08

## Request (unchanged)

```json
{
  "parent_node_ids": ["node_raw_a1b2", "node_raw_c3d4"],
  "operation_type": "ADDITION",
  "value_kind": "NUMBER",
  "number_value": 450.0,
  "expression": null
}
```

## Behavior by Operation Type

### Known Verifiable Operations

| Operation | Parent Count | Verification Rule | Error on Mismatch |
|---|---|---|---|
| `ADDITION` | 1+ | sum(parents) ≈ number_value | ✅ Rejected |
| `SUBTRACTION` | 2 | parent[0] - parent[1] ≈ number_value | ✅ Rejected |
| `MULTIPLICATION` | 1+ | product(parents) ≈ number_value | ✅ Rejected |
| `DIVISION` | 2 | parent[0] / parent[1] ≈ number_value | ✅ Rejected |
| `DATE_DIFF` | 2 | date_diff(parent[0], parent[1]).num_seconds() == duration_seconds | ✅ Rejected |
| `EXPRESSION` | 1+ | eval(expression, {a: val[0], b: val[1], ...}) ≈ number_value | ✅ Rejected |
| `IDENTITY` | 1 | parent[0] == value | ✅ Rejected |

### Unknown Operations

Operations not in the list above (including `null`/missing `operation_type`) are accepted without verification. Response includes `"verified": false`.

## Success Response (verified)

```json
{
  "node_id": "node_drv_e5f6g7h8",
  "is_stale": false,
  "parent_count": 2,
  "verified": true
}
```

## Success Response (unverified — unknown operation)

```json
{
  "node_id": "node_drv_e5f6g7h8",
  "is_stale": false,
  "parent_count": 2,
  "verified": false
}
```

## Error Response (verification failed)

```json
{
  "error": "Derivation verification failed for ADDITION: expected 300, got 500"
}
```

## Floating-Point Tolerance

Comparisons use both relative error (< 1e-12) and absolute difference (< 1e-9). The looser tolerance wins. This means:
- `300.0 ≈ 300.0000000001` → true (abs diff 1e-10 < 1e-9)
- `300.0 ≈ 300.1` → false (abs diff 0.1 > 1e-9)
- `1e12 ≈ 1e12 + 0.5` → true (rel error 5e-13 < 1e-12)
- `1e12 ≈ 1e12 + 5.0` → false (rel error 5e-12 > 1e-12)

## Expression Variable Binding

Parent values are bound to variables `a` through `z` in `parent_node_ids` order. If a parent value is a `Date`, it is converted to Unix timestamp seconds. If a `Duration`, it is converted to total seconds.

Example: `parent_node_ids = [nodeA, nodeB, nodeC]` with values `[300.0, 150.0, 2.0]` and expression `"(a + b) * c / 100"` evaluates to `(300 + 150) * 2 / 100 = 9.0`.
