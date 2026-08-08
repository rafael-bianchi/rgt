# Feature Specification: Derivation Verification

**Feature Branch**: `feat/011-derivation-verification`

**Created**: 2026-08-08

**Status**: Draft

**Input**: User description: "Re-compute derivations, date diff validation, and arithmetic verification via a CLI subcommand following Constitution VIII (CLI-First Hook Architecture / RTK Pattern)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Expression-Based Derivation Verification (Priority: P1)

An AI coding agent computes a derived value (e.g., `300 + 150 = 450`) and a hook script calls `rgt verify --parents node_a,node_b --operation EXPRESSION --expression "a + b" --result 450`. The CLI loads parent values from the database, evaluates the expression via `evalexpr`, and exits 0 on match. The hook script translates the CLI output to the agent's expected JSON format. If the value is wrong, the CLI exits non-zero with expected vs actual on stderr, and the hook can either auto-correct the call or deny and let the LLM retry.

**Why this priority**: Expression-based verification replaces 4 separate arithmetic operation types. `evalexpr` handles `+`, `-`, `*`, `/`, compound formulas, parentheses — all in one operation. This is the foundation of verification.

**Independent Test**: Record two number nodes (100, 200), run `rgt verify --parents n1,n2 --operation EXPRESSION --expression "a + b" --result 300`. Verify exit code 0. Run with `--result 500` and verify exit code 1 with expected=300, provided=500 on stderr.

**Acceptance Scenarios**:

1. **Given** parent nodes with values 100 and 200, **When** `rgt verify --parents n1,n2 --operation EXPRESSION --expression "a + b" --result 300` runs, **Then** exit code 0, stdout empty.
2. **Given** parent nodes with values 100 and 200, **When** `rgt verify --parents n1,n2 --operation EXPRESSION --expression "a + b" --result 500` runs, **Then** exit code 1, stderr contains `expected: 300` and `provided: 500`.
3. **Given** parent nodes a=300, b=150, c=2, **When** `rgt verify --parents n1,n2,n3 --operation EXPRESSION --expression "(a + b) * c / 100" --result 9.0` runs, **Then** exit code 0.
4. **Given** parent nodes a=10, b=3, **When** `rgt verify --parents n1,n2 --operation EXPRESSION --expression "a / b" --result 3.3333` runs, **Then** exit code 0 (within float tolerance).

---

### User Story 2 - Date Difference Verification (Priority: P1)

A hook script calls `rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000`. The CLI computes `date_diff(parent[0], parent[1])` (i.e., `date2 - date1`) and compares the resulting `duration_seconds`. Exits 0 on match, 1 on mismatch.

**Why this priority**: Date arithmetic is the second value type in RGT. Off-by-one errors and timezone issues are common in AI-generated date calculations. DATE_DIFF is the only operation that works on non-numeric types and cannot be covered by EXPRESSION.

**Independent Test**: Record two Date nodes (2026-01-01, 2026-01-11). Run with `--result 864000` (10 days). Verify exit 0. Run with `--result 864001` (off by 1 second). Verify exit 1.

**Acceptance Scenarios**:

1. **Given** Date nodes for 2026-01-01 and 2026-01-11, **When** `rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000` runs, **Then** exit code 0.
2. **Given** same Date nodes, **When** `rgt verify --parents d1,d2 --operation DATE_DIFF --result 864001` runs, **Then** exit code 1, stderr shows expected vs actual.
3. **Given** Date nodes in reverse order (2026-01-15, 2026-01-01), **When** `rgt verify --parents d2,d1 --operation DATE_DIFF --result -1209600` runs, **Then** exit code 0 (negative durations are valid).

---

### User Story 3 - Unsupported Operations Pass Through (Priority: P2)

A hook script calls `rgt verify --parents n1 --operation CUSTOM --result 42`. Since CUSTOM is not in the known operations (`EXPRESSION`, `DATE_DIFF`), the CLI exits 0 (no verification performed) and prints a warning to stderr. The derivation is stored without verification.

**Why this priority**: Backward compatibility — existing derivations with custom `operation_type` values must continue to work without modification.

**Independent Test**: Run `rgt verify --parents n1 --operation UNKNOWN --result 42`. Verify exit code 0 and stderr contains "unverifiable" warning.

**Acceptance Scenarios**:

1. **Given** any parent node, **When** `rgt verify --operation CUSTOM --result 42` runs, **Then** exit code 0, stderr: "Operation 'CUSTOM' is not verifiable — skipping verification."
2. **Given** `--operation` is omitted, **When** `rgt verify` runs, **Then** exit code 0 (no verification performed).

---

### Edge Cases

- What happens when a parent node is stale? Verification runs against the stored parent values; the derived node inherits the stale flag from parents.
- What happens with floating-point precision? Comparisons use relative error < `1e-12` OR absolute difference < `1e-9`.
- What happens with division by zero in an expression? `evalexpr` returns an error; `rgt verify` exits 1 with `"Division by zero in expression 'a / b'"` on stderr.
- What happens with an unknown variable in an expression (e.g., `"a + d"` when there are only 3 parents)? `evalexpr` returns an error; `rgt verify` exits 1 with `"Unknown variable 'd' — only a, b, c available"` on stderr.
- What happens when parent nodes don't exist in the database? Exit 1 with `"Parent node 'xyz' not found in database"` on stderr.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Verification logic MUST live in a shared Rust module (`src/verify/`) and be exposed as a CLI subcommand (`rgt verify`). Hook scripts and any future integration surfaces MUST delegate to this CLI — thin delegates per Constitution VIII.
- **FR-002**: `rgt verify` MUST accept `--parents <ids>`, `--operation <op>`, `--expression <expr>`, and `--result <value>` as arguments. Supported operations: `EXPRESSION`, `DATE_DIFF`.
- **FR-003**: For `EXPRESSION`, parent nodes MUST be mapped to single-letter variables in order: parent[0]→`a`, parent[1]→`b`, etc. The expression is evaluated via `evalexpr` with these variable bindings. Supports `+`, `-`, `*`, `/`, `%`, `^`, and parentheses.
- **FR-004**: For `DATE_DIFF`, the CLI MUST compute `date_diff(parent[0], parent[1])` from two Date parent nodes and compare the `duration_seconds` against `--result`.
- **FR-005**: The CLI MUST exit 0 on match (verification passed or operation unknown/not needed) and exit non-zero on mismatch. Error output MUST go to stderr. Success MUST produce no stdout.
- **FR-006**: On mismatch, stderr MUST output a structured error with `expected` and `provided` values, plus a `hint` suggesting the fix (e.g., `hint: retry with --result 300`).
- **FR-007**: For unknown operations, the CLI MUST exit 0 and print a warning to stderr: `Operation '<name>' is not verifiable — skipping verification.`
- **FR-008**: Floating-point comparisons MUST use relative error < `1e-12` OR absolute difference < `1e-9`.
- **FR-009**: The unused `date_diff()` function in `src/types/value.rs` MUST be wired into the DATE_DIFF path, replacing `#[allow(dead_code)]`.
- **FR-010**: Hook scripts MUST document the variable mapping convention (`parent[0]=a, parent[1]=b, ...`) and supported operations so the LLM discovers them when the hook is installed (via `rgt init -g` output or hook script comments).

### Key Entities

- **`rgt verify` CLI**: New subcommand. Accepts parent node IDs, operation, expression, and expected result. Loads parent values from SQLite, computes expected value, compares. Exits 0/1.
- **Verification Module** (`src/verify/`): Shared Rust module. `verify_expression(parents: &[ValueData], expression: &str, result: f64) -> Result<(), String>` and `verify_date_diff(parents: &[ValueData], result_seconds: i64) -> Result<(), String>`. Called by the `rgt verify` CLI subcommand.
- **Operation Set**: Two known operations: `EXPRESSION` (evaluates formula via `evalexpr`) and `DATE_DIFF` (computes `date2 - date1` via `date_diff`).

### CLI Error Contract

```
# Match:
$ rgt verify --parents a,b --operation EXPRESSION --expression "a + b" --result 300
[exit 0, no stdout, no stderr]

# Mismatch:
$ rgt verify --parents a,b --operation EXPRESSION --expression "a + b" --result 500
Error: verification failed for EXPRESSION "a + b"
  expected: 300
  provided: 500
  hint: retry with --result 300
[exit 1]

# Unknown operation:
$ rgt verify --parents a --operation CUSTOM --result 42
Warning: operation 'CUSTOM' is not verifiable — skipping verification
[exit 0]
```

### Hook Script Contract (per Constitution VIII)

Hook scripts are thin delegates that translate the CLI exit code and stderr to agent-specific JSON:

| CLI Result | Hook Action |
|---|---|
| Exit 0, no stderr | Verification passed → allow, return `updatedInput` unchanged |
| Exit 1, stderr with `hint` | Mismatch → auto-correct the call if `hint` contains a retry value |
| Exit 0, stderr with `Warning` | Unverifiable → allow, pass through, no rewrite |
| Exit 1, stderr with `not found` | Missing parents → deny with reason, let LLM retry |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of `EXPRESSION` and `DATE_DIFF` derivations are verified before storage — incorrect results are rejected before entering the graph.
- **SC-002**: Incorrect derivations produce exit code 1 with a `hint` containing the correct value, enabling the hook to auto-correct without LLM retry.
- **SC-003**: Unknown operations exit 0 and pass through without blocking the derivation.
- **SC-004**: Existing tests continue to pass after verification is added (backward compatible).
- **SC-005**: Verification via CLI adds no more than 10ms latency per call (load parents from SQLite, evaluate expression, compare).
- **SC-006**: Hook scripts installed via `rgt init -g` include the variable mapping convention and supported operations in their documentation, discoverable by the LLM when the hook fires.

## Assumptions

- `parent_node_ids` order defines variable mapping: parent[0]→`a`, parent[1]→`b`, parent[2]→`c`, etc. This is consistent across the CLI and all hook scripts.
- `evalexpr` (MIT, crates.io) handles expression parsing and evaluation. No native dependencies.
- `chrono::Duration` is sufficient for date difference — no year/month calculations needed.
- Verification happens at derivation recording time only — stored derivations are not re-verified on query.
- Hook scripts are thin delegates per Constitution VIII — they translate CLI output to agent JSON, no business logic.
