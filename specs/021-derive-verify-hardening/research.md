# Research: Derive & Verify Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Resolves the plan's Technical Context unknowns by inspecting the vendored `evalexpr` 13.1.0 source (no external fetch needed) and hand-tracing the current `src/verify/*` behavior.

## 1. Disabling evalexpr builtin functions

**Decision**: In `verify_expression`, after building the `HashMapContext`, call `context.set_builtin_functions_disabled(true)?` before evaluation (FR-004).

**Rationale**: Verified in the vendored source (`evalexpr-13.1.0/src/context/mod.rs`): the `Context` trait exposes `set_builtin_functions_disabled(&mut self, disabled: bool) -> EvalexprResult<()>` (line 43), and `HashMapContext` implements it (line 333). The evaluator's operator path checks `context.are_builtin_functions_disabled()` before resolving function identifiers (operator/mod.rs:429). Today `verify_expression` never calls it, so the full builtin set (`random`, `str::regex_matches`, `math::*`, `min`/`max`/`if`/`bitand`/`typeof`, …) is reachable — confirming §7.3.

**Alternatives considered**:
- `EmptyContext` (builtins permanently disabled, cannot be enabled) — verified in context/mod.rs:111-146; but it cannot bind variables via `set_value` without `ContextWithMutableVariables`, and the current code needs variable binding, so `HashMapContext` + the disable toggle is the direct fit.
- Per-expression operator allow-listing (pre-parse AST walk) — rejected: evalexpr's toggle already does it.

## 2. Integer result coercion

**Decision**: Replace `computed.as_float()` with `computed.as_number()` in `verify_expression` (FR-003).

**Rationale**: evalexpr's `Value::as_number()` silently coerces `IntType` → `FloatType`, while `as_float()` returns `Err` for `Value::Int`. A pure-literal expression like `"2*50"` evaluates to `Value::Int(100)`, so `as_float()` returns "Expression did not evaluate to a number" despite a correct result — the §7.3 confirmed bug. (`as_number` is the documented API; no hand-rolled match needed.)

## 3. Pre-evaluation depth/length guard (CRITICAL)

**Decision**: Add `validate_expression(expr) -> Result<(), String>` called at the top of `verify_expression`, enforcing `MAX_EXPRESSION_LEN = 4096` bytes and `MAX_EXPRESSION_DEPTH = 256` (maximum parenthesis nesting), returning a clean error before any evalexpr parse/eval (FR-001).

**Rationale**: The 200k-deep nested expression crashes in evalexpr's **recursive evaluator** (parse succeeds — the parens are balanced — then `eval` recurses once per level → stack overflow/SIGABRT). A length/depth bound checked up front (O(n), no recursion) prevents the evaluator from ever being handed an over-deep tree. 256 nesting levels is far below any stack-exhaustion risk; 4096 bytes is generous for legitimate formulas. The limits are named in the error message.

**Alternatives considered**:
- `std::thread::Builder::stack_size(...)` isolation — the findings' complementary fix. Rejected as the *primary* fix: it only raises the ceiling and turns a SIGABRT into a joinable-thread panic; it doesn't eliminate unbounded recursion. Noted as optional hardening, not required.
- A catchable-depth guard inside evalexpr — evalexpr has no per-expression depth limit hook in its public API; the pre-check is the reliable, crate-agnostic approach.

## 4. Operation allow-list (exit 2)

**Decision**: A case-sensitive allow-list `matches!(operation, "EXPRESSION" | "DATE_DIFF")`, checked at the top of both `run_verify_cli` and `execute_derive`; anything else (including `expression`/`Expression`/`date_diff`) prints the valid list and exits 2. Remove `verify()`'s `_ => Ok(())` pass-through and `derive.rs`'s `_ => warning + Ok`. `derive` records nothing on rejection (FR-002).

**Rationale**: §7.3 confirmed `--operation expression` exits 0 as a silent no-op. A case-sensitive allow-list keeps the documented `EXPRESSION`/`DATE_DIFF` contract and treats casing slips as the invalid input they are. This is simpler and equally correct vs. converting `--operation` to a clap `ValueEnum` (which would also change the `Verify`/`Derive` command definitions in `main.rs`); the observable contract (exit 2 + valid list) is identical.

**Alternatives considered**: clap `ValueEnum` (`#[arg(value_enum)]`) — auto-generates the valid-list error at parse time; rejected for scope (no CLI-type change needed to satisfy the contract), noted as a clean future refactor.

## 5. Parent-count cap

**Decision**: In `verify_expression`, if `parents.len() > 26`, return `Err("at most 26 parents supported (variables a–z)")` before the binding loop; replace `((b'a' + i as u8) as char)` with a bounded `b"abcdefghijklmnopqrstuvwxyz"[i]` (FR-005).

**Rationale**: The current `b'a' + i as u8` is unchecked — 159+ parents panics in debug, wraps silently in release (the §7.3 finding). A hard cap at 26 (the documented single-letter variable space) makes the failure deterministic and documented.

## 6. DATE_DIFF sign enforcement

**Decision**: In `verify_date_diff`, after computing `date2 - date1`, if the duration is **negative**, return `Err("DATE_DIFF computed date2 - date1 = -Ns (negative); parents may be swapped — convention is --parents <date1>,<date2>")`. Update `test_date_diff_negative` to expect `Err` (FR-006, clarified 2026-08-29 Option A).

**Rationale**: A negative duration is the observable signal of a swapped `--parents` order (the convention is `date2 - date1` with `date2` chronologically after `date1`). Rejecting negatives makes the previously-silent opposite-sign pass a hard error that names the convention, matching the clarified FR-006 without a CLI-breaking flag change.

**Alternatives considered**: named `--date1`/`--date2` flags (Option B) — rejected per the clarify decision; hint-only (Option D) — rejected (still passes silently).
