# Spec 047: Verify Arithmetic, Tolerance, and Duration Logic

## 1. Overview & Problem Statement

In RGT's verification and derivation engine:
1. `verify_expression` binds variables `a..` for all parent nodes, but never verifies that the expression actually references each bound parent variable. As a result, formulas like `"42"` or `"a + 0"` ignoring parent `b` succeed, creating phantom provenance edges.
2. `approx_eq` uses an absolute tolerance `abs < 1e-9`, which accepts `0.0` or even opposite-signed numbers when checking small scientific values (`< 1e-9`).
3. `execute_derive` records the caller's raw claim (`expected`) rather than the verified evaluated result (`computed_val`), introducing rounding errors into the provenance graph.
4. `ValueData::Duration::to_string_repr()` omits `secs` whenever `days > 0`, causing day-scale durations with seconds (e.g. 86,401s) to truncate to `"1d"`.

---

## 2. User Stories

- **[US1] Variable Reference Enforcement in Expressions**:
  `verify_expression` must verify that every bound parent variable `a`.. for the given parent count is actually referenced as an identifier token in the expression. If any parent variable is missing, verification must reject the derivation with an error (exit code 2).
- **[US2] Robust Floating-Point Relative Tolerance**:
  `approx_eq` must use scaled relative tolerance `diff <= (1e-9 * largest).max(f64::EPSILON * 8.0)` and require sign agreement when values are above machine epsilon. Small values (e.g. `1e-10` vs `0.0` or `-1e-10`) must not falsely match.
- **[US3] Evaluated Result Precision in `execute_derive`**:
  `execute_derive` for `EXPRESSION` must record `ValueData::Number(computed_val)` — the exact result evaluated from parent values — rather than the caller's rounded claim.
- **[US4] Duration String Representation Completeness**:
  `ValueData::Duration::to_string_repr()` must not drop non-zero seconds when `days > 0`. A duration with days and seconds (e.g. 86,401s) must include seconds in its string representation.

---

## 3. Non-Goals & Boundaries

- Does not modify store schema or migrations (Spec 045).
- Does not modify value extraction or `rgt record --stdin` (Spec 046).
- Does not modify hook JSON payloads or agent registrations (Specs 048, 049).
- Does not modify Turtle export IRI formats (Spec 050).
