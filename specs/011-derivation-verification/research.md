# Research: Derivation Verification

**Feature**: `011-derivation-verification` | **Date**: 2026-08-08

## Decision 1: Expression Evaluation Library

**Decision**: `evalexpr` (MIT, crates.io).

**Rationale**:
- MIT license — compatible with Constitution VII.
- Lightweight, pure Rust, no native deps.
- Variable binding via `HashMap<String, Value>` — maps to parent order (a, b, c...).
- Supports `+`, `-`, `*`, `/`, `%`, `^`, parentheses.
- Single function call: `evalexpr::eval_with_context(expr, &context)`.

**Alternatives considered**: `meval` (stale), `fasteval` (overkill), custom parser (unnecessary), symbolica (proprietary).

## Decision 2: Operation Set — Two Types Only

**Decision**: Support only `EXPRESSION` and `DATE_DIFF`. No individual arithmetic operations.

**Rationale**:
- `evalexpr` handles `a + b`, `a * b`, `(a + b) * c / 100` uniformly.
- `DATE_DIFF` is the only operation using non-numeric types — keep separate.
- Reduces surface area: 2 operations instead of 7.

**Alternatives considered**: ADD/SUB/MUL/DIV as separate types — adds maintenance with no benefit. Rejected.

## Decision 3: CLI Subcommand Architecture

**Decision**: `rgt verify --parents <ids> --operation <op> --expression <expr> --result <value>`. Exit 0 on match, 1 on mismatch, stderr for errors.

**Rationale**:
- Constitution VIII: hooks are thin delegates, all logic in CLI.
- Exit codes enable shell scripting (`if rgt verify ...; then ...`).
- stderr errors with `expected`, `provided`, `hint` enable hook auto-correction.

**Alternatives considered**: MCP-only verification — violates Constitution VIII. Rejected.

## Decision 4: CLI Error Contract

**Decision**: Structured stderr output with `expected`, `provided`, and `hint` fields. Exit codes: 0 (pass / unverifiable), 1 (mismatch), 2 (invalid input).

| Exit | Condition | stderr |
|---|---|---|
| 0 | Match or unverifiable | (none) or warning |
| 1 | Mismatch | `expected: X, provided: Y, hint: retry with --result X` |
| 2 | Invalid input (missing parents, bad expression) | Error message |

**Rationale**: Exit 1 with `hint` enables hook scripts to auto-correct without LLM retry. Exit 2 for hard errors prevents silent failures.

## Decision 5: Floating-Point Tolerance

**Decision**: Relative error < 1e-12 OR absolute difference < 1e-9.

```rust
fn approx_eq(a: f64, b: f64) -> bool {
    let abs = (a - b).abs();
    abs < 1e-9 || abs / b.abs().max(1e-10) < 1e-12
}
```

**Rationale**: Covers large numbers (relative) and near-zero numbers (absolute). Consistent with f64 machine epsilon (~2.2e-16) with safety margin.

## Decision 6: Parent-to-Variable Mapping

**Decision**: `parent[0]` → `a`, `parent[1]` → `b`, `parent[2]` → `c`, etc.

**Rationale**: Deterministic, intuitive, matches RTK's variable binding pattern. Hook scripts document this convention at install time.
