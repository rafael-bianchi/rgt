# Data Model: Derive & Verify Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for the `rgt verify`/`rgt derive` paths. Existing `ValueData`/`TrackedNode` entities are unchanged; the entities below carry this feature's constraints.

## Entities

### Operation

The `--operation` value supplied to `verify`/`derive`.

| Attribute | Type | Notes |
|---|---|---|
| `value` | `String` | Only `EXPRESSION` and `DATE_DIFF` are valid (case-sensitive). |

Rules:
- Any value outside the set — including `expression`/`Expression`/`date_diff` — is **invalid input** (exit 2), never "skipped" or exit 0 (FR-002).
- `derive` records no node and no edges on an invalid operation.

### Expression

The `--expression` formula string (only meaningful for `Operation::EXPRESSION`).

| Attribute | Type | Notes |
|---|---|---|
| `text` | `String` | Formula over bound parent variables `a`–`z` and operators `+ - * / % ^` and parentheses. |
| `length` | `usize` | Must be ≤ `MAX_EXPRESSION_LEN` (4096). |
| `max_depth` | `usize` | Max parenthesis nesting; must be ≤ `MAX_EXPRESSION_DEPTH` (256). |

Rules:
- Violating either bound is a clean error naming the limit, before any evaluation (FR-001).
- Only bound variables and documented operators are evaluated; builtin functions are disabled (FR-004).

### ParentBinding

A parent node value bound to a single-letter variable.

| Attribute | Type | Notes |
|---|---|---|
| `index` | `usize` | Position in `--parents`; maps to variables `a`–`z`. |
| `value` | `ValueData` | Number / Duration / Date, coerced to float for EXPRESSION. |

Rules:
- At most 26 parents (`a`–`z`); more → clean error, no panic/wrap (FR-005).

### EvaluationResult

The computed value compared against the claimed `result`.

| Attribute | Type | Notes |
|---|---|---|
| `number` | `f64` | Obtained via `as_number()` so integer-typed results coerce to float (FR-003). |
| `deterministic` | `bool` | Guaranteed by builtin-function disabling (FR-004). |

### DateDiffResult

The `DATE_DIFF` duration, computed as `date2 - date1` from the two positional `--parents`.

| Attribute | Type | Notes |
|---|---|---|
| `seconds` | `i64` | Must be **non-negative**; a negative value is a swapped-order signal → error naming the `date2 - date1` convention (FR-006). |

### DerivationRecord

The derived node written by `rgt derive`. **Precondition**: an operation that ran verification **and passed**; never written when the operation was invalid/unverifiable (FR-002).

## Validation rules (from FR-001..FR-006)

- **Pre-evaluation guard**: length ≤ 4096 and nesting ≤ 256, checked before eval (FR-001).
- **Allow-list**: operation ∈ {EXPRESSION, DATE_DIFF} (FR-002).
- **Coercion**: `as_number()` (FR-003).
- **Determinism**: builtins disabled (FR-004).
- **Parent cap**: ≤ 26 (FR-005).
- **Sign enforcement**: DATE_DIFF ≥ 0 (FR-006).

## Out of scope

Value extraction, node-ID generation (the §7.3 same-line collision and 48-bit IDs belong to specs 022/024), storage, and graph changes.
