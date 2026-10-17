# Contract: `rgt verify` / `rgt derive` CLI & Evaluation Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the unit/contract tests for the verify/derive paths.

## Operation allow-list

- Valid `--operation` values (case-sensitive): `EXPRESSION`, `DATE_DIFF`.
- Anything else → `Error: unknown operation '<op>' — valid operations: EXPRESSION, DATE_DIFF`, **exit 2**, for BOTH commands.
- `rgt verify` MUST NOT print "skipping verification" or exit 0 for an unrecognized operation.
- `rgt derive` MUST NOT write a derived node or any derivation edge when the operation is unrecognized.

## Expression limits (FR-001)

- `MAX_EXPRESSION_LEN = 4096` bytes; `MAX_EXPRESSION_DEPTH = 256` (max parenthesis nesting).
- A violation → clean error naming the exceeded limit, normal non-zero exit, **before** any evaluation. Never a stack overflow / SIGABRT.
- In-limit expressions are unaffected.

## Evaluation invariants (FR-003/FR-004)

- Result coercion uses `as_number()` semantics: `"2*50"` verifies against `100`.
- Builtin functions are disabled: `random()`, `str::*`, `math::*`, `min`/`max`/`if`/`bitand`/`typeof`, regex — all rejected. Only bound variables `a`–`z` and `+ - * / % ^` and parentheses evaluate.
- Evaluation is deterministic: the same expression + parents yields the same result every run.

## Parent-count (FR-005)

- 26 parents (`a`–`z`) are supported. 27+ → `Error: at most 26 parents supported (variables a–z)`, no panic, no wraparound.

## DATE_DIFF (FR-006)

- Convention: `--parents <date1>,<date2>` computes `date2 - date1`.
- A negative computed duration → error naming the `date2 - date1` convention ("parents may be swapped"), never a silent opposite-sign pass.
- Non-Date parents → existing type error (unchanged).

## Exit codes

| Code | Meaning | When |
|---|---|---|
| 0 | Success | Verification matched; or `derive` recorded a verified derivation. |
| 1 | Verification mismatch | Computed ≠ claimed (mismatch, division by zero, evaluation error, negative DATE_DIFF). |
| 2 | Invalid input | Unknown operation, missing `--expression`, >26 parents, over-limit expression, missing parent node. |

## Acceptance linkage

- US1/FR-001 → expression limits + exit code 2 (clean error).
- US2/FR-002 → operation allow-list + exit 2 + no derive-record.
- US3/FR-003/FR-004 → integer coercion + determinism.
- US4/FR-005/FR-006 → parent cap + DATE_DIFF sign enforcement.
