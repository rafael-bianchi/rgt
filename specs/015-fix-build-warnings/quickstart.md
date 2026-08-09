# Quickstart Validation: Eliminate Build Warnings & Cargo Suggestions

**Feature**: 015-fix-build-warnings

This guide documents runnable validation scenarios that prove the feature works end-to-end.

## Prerequisites

- Rust toolchain (`cargo`, `cargo clippy`)
- Clean `develop` checkout

## Scenarios

### Scenario 1: Zero rustc Warnings (all targets)

**Steps**:
```bash
# 1. Library + binary + tests
cargo build --all-targets 2>&1 | grep warning
# Expected: no output (exit code 1 from grep = zero matches)

# 2. Binary only
cargo build 2>&1 | grep warning
# Expected: no output

# 3. Test targets compile
cargo test --no-run 2>&1 | grep warning
# Expected: no output
```

**Pass**: All three greps return zero matches.

---

### Scenario 2: Zero Clippy Warnings

**Steps**:
```bash
cargo clippy --all-targets 2>&1 | grep -E "warning:|error:"
# Expected: no output
```

**Pass**: Zero matches. (The `generated N warnings` summary line is excluded by the grep pattern; the actual warning lines are what matter.)

---

### Scenario 3: Full Test Suite Green

**Steps**:
```bash
cargo test 2>&1 | grep "^test result"
# Expected: 14 test-result lines, all ending in "0 failed"
cargo test 2>&1 | grep -c "0 failed"
# Expected: 14 (all suites pass)
```

**Pass**: 108+ tests pass, 0 failures.

---

### Scenario 4: Formatting Clean

**Steps**:
```bash
cargo fmt --all -- --check
# Expected: exit 0, no output
```

**Pass**: No formatting diffs.

---

### Scenario 5: CI Green (final gate)

**Steps**:
```bash
# After pushing the fix branch and opening a PR:
gh pr checks <pr-number>
# Expected: fmt ✅, clippy ✅, test (ubuntu) ✅, test (windows) ✅,
#            test (macos) ✅, security ✅, test presence ✅
```

**Pass**: All checks green with zero warnings in job logs.

---

## Success Criteria Mapping

| SC | Verified By |
|---|---|
| SC-001 (0 of 8+ warnings remain) | Scenarios 1-2 |
| SC-002 (108+ tests, 0 warnings) | Scenario 3 |
| SC-003 (CI clippy green) | Scenario 5 |
| SC-004 (100% disposition coverage) | research.md (all 9 warnings have dispositions) |
| SC-005 (no signature/behavior change) | Scenario 3 (tests pass unchanged) |
| SC-006 (fmt clean) | Scenario 4 |
