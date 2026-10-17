# Feature Specification: Derive & Verify Hardening

**Feature Branch**: `feat/021-derive-verify-hardening`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "rgt verify and rgt derive must be safe and trustworthy: a malformed or adversarial --expression must never crash the CLI, verification must never be silently bypassed by an unrecognized --operation, integer-typed results must verify, expression evaluation must be deterministic (no hidden builtin functions), and parent-count edge cases must error cleanly instead of panicking or silently wrapping. Findings §7.3."

Source context: `docs/findings-hooks-and-value-extraction.md` §7.3 (plan: `docs/spec-plan-from-findings.md`, spec #3, P0). Out of scope per the plan: the §7.3 same-line value collision (spec #4, `022-value-extraction-correctness`) and the 48-bit node-ID truncation (spec #6, `024-store-graph-integrity`).

## Clarifications

### Session 2026-08-29

- Q: How should DATE_DIFF's parent-order ambiguity (a swapped `date2 - date1` passing silently with the opposite sign) be resolved? → A: Keep the positional `--parents id1,id2` interface (no CLI break); enforce the `date2 - date1` convention at verification — when the claimed result matches the opposite sign, fail with a clear error naming the `date2 - date1` convention.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Malformed `--expression` Can Never Crash the CLI (Priority: P1)

A developer (or an AI agent) runs `rgt verify` / `rgt derive` with a deeply-nested or oversized `--expression`. Today a 200k-deep `(((...1...)))` aborts the process with a stack overflow (SIGABRT, exit 134) instead of a clean error. The command must reject expressions that exceed a safe length/nesting bound with a clear, user-recoverable error and a normal non-zero exit — never a crash.

**Why this priority**: CRITICAL per §7.3 — a single CLI argument can kill the process, confirmed empirically against both `rgt verify` and `rgt derive`. This is a trivially-triggerable reliability/availability bug in a tool agents call automatically.

**Independent Test**: Unit + contract tests feed a deeply-nested expression (e.g. `'('*200000 + '1' + ')'*200000`) and a very long expression to the verification entry point and assert a clean `Err` (no process abort). A release-mode binary run must exit non-zero without a SIGABRT.

**Acceptance Scenarios**:

1. **Given** a `--expression` whose nesting depth exceeds the safe limit, **When** `rgt verify` runs, **Then** it prints a clear error naming the limit and exits non-zero (never SIGABRT/stack overflow).
2. **Given** a `--expression` whose length exceeds the safe limit, **When** `rgt derive` runs, **Then** it rejects it with a clear error and records nothing.
3. **Given** a well-formed expression within the limits, **When** verification runs, **Then** behavior is unchanged (still verifies correctly).

---

### User Story 2 - Verification Can Never Be Silently Bypassed (Priority: P1)

A developer runs `rgt verify --operation expression ...` (lowercase) or any unrecognized operation. Today the command prints a warning and **exits 0** ("not verifiable — skipping"), and `rgt derive` with an unknown operation skips verification but still records the derivation. Since the whole point of these commands is to gate agent-claimed values, a casing slip silently turns the safety check into a no-op that reports success. Unrecognized operations must be **invalid input**: a clear error listing valid operations and exit code 2, for both commands; `rgt derive` must record nothing when verification cannot run.

**Why this priority**: HIGH per §7.3, confirmed empirically (`--operation expression --expression "a + 999999" --result 1` exits 0). A plausible LLM casing slip defeats the verification guarantee entirely.

**Independent Test**: Contract tests run `--operation expression`/`Expression`/`date_diff`/unknown against both commands and assert exit 2 with the valid-operation list; `rgt derive` records no node.

**Acceptance Scenarios**:

1. **Given** `--operation expression` (case-mismatched), **When** `rgt verify` runs, **Then** it errors (exit 2) listing `EXPRESSION, DATE_DIFF`, and never reports success.
2. **Given** any operation not in `{EXPRESSION, DATE_DIFF}`, **When** `rgt derive` runs, **Then** it errors (exit 2) and records no derivation.
3. **Given** `--operation EXPRESSION` (correct), **When** the command runs, **Then** verification proceeds normally.

---

### User Story 3 - Verification Results Are Trustworthy and Deterministic (Priority: P1)

A developer runs `rgt verify --expression "2*50" --result 100` — a pure-literal expression with no parent references. Today it fails ("did not evaluate to a number") because evalexpr keeps the result as an integer and `as_float()` refuses to coerce it. It must verify. Separately, the expression evaluator must only expose the documented arithmetic operators (`+ - * / % ^` and parentheses) over bound parent variables — never hidden builtins like `random()` (which breaks determinism) or `str::regex_matches` (which accepts arbitrary caller-supplied regexes nobody intended to expose).

**Why this priority**: MEDIUM per §7.3 on both counts, but they undermine the core "recompute and check" model: a correct result being rejected breaks real workflows, and non-deterministic or arbitrarily-powerful builtins make the check meaningless.

**Independent Test**: Unit tests assert `"2*50" → 100` verifies; `"random()"` and `"str::regex_matches(...)"` are rejected; evaluating the same expression twice returns identical results.

**Acceptance Scenarios**:

1. **Given** `--expression "2*50" --result 100`, **When** verification runs, **Then** it succeeds (integer-typed results are coerced).
2. **Given** `--expression "random()"`, **When** verification runs, **Then** it errors: builtin functions are disabled, only bound variables and `+ - * / % ^` work.
3. **Given** an expression that references no parent variables, **When** verification runs, **Then** the result is still a clean number (not "did not evaluate to a number").

---

### User Story 4 - Parent-Count and Date-Ordering Edge Cases Error Cleanly (Priority: P2)

A developer passes more than 26 parents or an ambiguous DATE_DIFF parent order. Today parent variables are named `a`…`z` with unchecked `u8` arithmetic (`b'a' + i as u8`): more than 159 parents panics in debug builds and silently wraps in release; 27–159 parents produce wrong/duplicated variable names. DATE_DIFF's `date2 - date1` convention is documented but not enforced — a consistently-swapped order passes verification with the opposite sign silently. Both must fail cleanly with actionable messages, or (for DATE_DIFF) be made unambiguous.

**Why this priority**: LOW per §7.3, but unchecked arithmetic on an unbounded, caller-controlled count is a latent panic/wraparound, and a silently-mis-signed derived date is a correctness trap for agents.

**Independent Test**: Unit tests assert >26 parents → clean error (no panic/wrap); a swapped DATE_DIFF order is either rejected or unambiguously diagnosed.

**Acceptance Scenarios**:

1. **Given** 27 or more parents, **When** `rgt verify` runs, **Then** it errors cleanly ("at most 26 parents supported: a–z") without panicking or wrapping.
2. **Given** a DATE_DIFF where the result sign implies the parents may be swapped, **When** verification runs, **Then** it fails with a clear error naming the `date2 - date1` convention (never passes silently with the opposite sign).
3. **Given** a valid DATE_DIFF with the documented order, **When** verification runs, **Then** it behaves as today.

---

### Edge Cases

- What happens with an empty `--expression` or `--expression ""`?
- What happens with an expression containing only whitespace?
- What happens with `--operation` given uppercase, lowercase, or mixed case (`expression`, `Expression`, `EXPRESSION`)?
- What happens when `--operation DATE_DIFF` is given with non-Date parents (numbers)?
- What happens with `--expression "1/0"` (division by zero) — clean error today, must stay clean?
- What happens with integer overflow in a large literal arithmetic (e.g. `9^9^9`)?
- What happens with exactly 26 parents (`a`–`z`)?
- What happens with expressions using whitespace/unicode around operators?
- What happens when the user passes `--operation EXPRESSION` without `--expression` (already exit 2 — keep)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt verify` and `rgt derive` MUST reject an `--expression` that exceeds a documented safe length or nesting-depth limit with a clear, user-recoverable error and a non-zero exit — before any evaluation. A malformed/adversarial expression MUST NEVER abort the process (no stack overflow / SIGABRT).
- **FR-002**: An `--operation` value not in `{EXPRESSION, DATE_DIFF}` — including case variants (`expression`, `Expression`, `date_diff`) — MUST be rejected as invalid input: a clear error listing the valid operations and exit code 2, for BOTH `rgt verify` and `rgt derive`. Verification MUST NOT be silently skipped, and `rgt derive` MUST NOT record a derivation when verification did not run.
- **FR-003**: Expression evaluation MUST coerce integer-typed results to float for comparison (`as_number` semantics), so pure-literal expressions like `"2*50"` verify against `100`.
- **FR-004**: The expression evaluator MUST expose only bound parent variables and the documented arithmetic operators (`+ - * / % ^` and parentheses). All evalexpr built-in functions (`random`, `str::*`, `math::*`, `min`/`max`/`if`/`bitand`/`typeof`, regex helpers, …) MUST be disabled, so evaluation is deterministic and side-effect free.
- **FR-005**: Passing more than 26 parents MUST produce a clean error ("at most 26 parents supported: a–z"), never a panic (debug) or silent `u8` wraparound (release).
- **FR-006**: The DATE_DIFF parent-order convention (`date2 - date1`) MUST be enforced at verification, keeping the positional `--parents id1,id2` interface: when the claimed result matches the opposite sign (a consistently-swapped order), verification MUST fail with a clear error naming the `date2 - date1` convention. A swapped order MUST NOT pass silently (clarified 2026-08-29).

### Key Entities *(include if feature involves data)*

- **Expression**: The `--expression` formula string; carries length and nesting-depth constraints (FR-001).
- **Operation**: `--operation` value; the only valid values are `EXPRESSION` and `DATE_DIFF` (FR-002).
- **ParentBinding**: A parent node value bound to a single-letter variable `a`–`z` (FR-005).
- **EvaluationContext**: The restricted evaluator context exposing only bound variables and the documented operators (FR-004).
- **DerivationRecord**: The node written by `rgt derive`; MUST only be written when verification ran and passed (FR-002).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A nested `--expression` over the depth limit (tens of thousands of parentheses) returns a clean error and exit code 2 — never a SIGABRT/stack overflow — in both debug and release builds (regression-tested).
- **SC-002**: 0 silent pass-throughs: every `--operation` outside `{EXPRESSION, DATE_DIFF}` (including case variants) exits 2 with the valid-operation list; `rgt derive` records nothing in that case.
- **SC-003**: `--expression "2*50" --result 100` verifies successfully (integer coercion).
- **SC-004**: `random()` and other evalexpr builtins are rejected; repeated evaluation of the same expression yields identical results (determinism).
- **SC-005**: 27+ parents → clean error (no panic, no wrap); 26 parents still works.
- **SC-006**: Every new error message names the offending input (`--expression` value or `--operation` value) and how to fix it.

## Assumptions

- Scope is the §7.3 finding family minus the items owned by other specs: stack-overflow crash (CRITICAL), case-mismatched `--operation` bypass, `as_float` vs `as_number`, evalexpr builtin exposure (`random()` determinism), DATE_DIFF parent-order, and `u8` parent-count overflow. The §7.3 same-line value collision and 48-bit node-ID truncation are explicitly out of scope (specs 022/024).
- A documented safe limit for expression length/depth will be chosen in planning (default proposal: a depth limit well below stack-usage risk, e.g. ≤ 256 nesting levels, and a length limit consistent with a CLI argument, e.g. ≤ 4 KiB), with the limit named in error messages. The complementary bounded-stack-thread evaluation is evaluated in planning; the hard requirement is the pre-evaluation limit (FR-001).
- Operation validation happens at argument parsing or at the top of both commands — the exact mechanism (clap `ValueEnum` vs an in-code allow-list) is a planning decision; the observable contract (exit 2, valid-list message, no silent skip) is fixed here.
- DATE_DIFF parent ordering: **resolved 2026-08-29** — keep the positional `--parents id1,id2` interface and enforce `date2 - date1` at verification (a swapped order fails with a convention-naming error). See Clarifications.
- No changes to the value store, hook path, extraction, or graph; `src/verify/*`, `src/cli/derive.rs`, and the `Verify`/`Derive` CLI surfaces are the only touched areas.
- The approx-equal float tolerance (`1e-9` absolute / `1e-12` relative) is unchanged; a dedicated property-based test for it is noted but out of scope.
