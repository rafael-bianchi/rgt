# Implementation Plan: Verify Arithmetic, Tolerance, and Duration Logic

**Branch**: `fix/047-verify-arithmetic-and-duration-hardening` | **Spec**: `specs/047-verify-arithmetic-and-duration-hardening/spec.md`

## Summary of Changes

1. **Parent Variable Reference Verification** (`src/verify/expression.rs`):
   - In `verify_expression`, for each variable `a`.. up to `parents.len()`, verify that the variable exists as an identifier in the expression.
   - Use regex or word-boundary token matching to ensure `a` is matched as a standalone identifier token, not part of a larger name.
   - If any parent variable is missing, return an error indicating which variable is unused.
2. **Tolerance Hardening** (`src/verify/expression.rs`):
   - Update `approx_eq(a: f64, b: f64)`:
     - Exact equality `a == b` returns `true`.
     - Non-finite or NaN checks.
     - Relative tolerance `diff <= (1e-9 * largest).max(f64::EPSILON * 8.0)`.
     - Sign agreement check when `largest > 1e-14`.
3. **Store Evaluated Value in Derive** (`src/cli/derive.rs`, `src/verify/mod.rs`):
   - Expose evaluated value from `verify_expression_eval` or have `verify` return the computed `f64`.
   - Store `ValueData::Number(computed_val)` in `execute_derive`.
4. **Complete Duration String Formatting** (`src/types/value.rs`):
   - In `ValueData::Duration::to_string_repr()`, when `days > 0`:
     - If `hours == 0 && mins == 0 && secs == 0`: `format!("{}{}d", sign, days)`
     - If `secs == 0`:
       - `if mins == 0`: `format!("{}{}d {}h", sign, days, hours)`
       - `else`: `format!("{}{}d {}h {}m", sign, days, hours, mins)`
     - If `secs > 0`:
       - `if hours == 0 && mins == 0`: `format!("{}{}d {}s", sign, days, secs)`
       - `else if mins == 0`: `format!("{}{}d {}h 0m {}s", sign, days, hours, secs)`
       - `else`: `format!("{}{}d {}h {}m {}s", sign, days, hours, mins, secs)`
   - Update tests to verify full precision without losing seconds.
