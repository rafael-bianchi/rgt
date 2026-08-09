# Quickstart Validation: Fix Pre-existing CI Issues

**Feature**: 014-fix-ci-issues

This guide documents runnable validation scenarios that prove the feature works end-to-end. Run these after implementation to verify correctness.

## Prerequisites

- Rust toolchain (`cargo`)
- `gh` CLI authenticated with repo access
- GitHub Actions enabled on the repo

## Scenarios

### Scenario 1: Local Test Harness (all platforms)

**Purpose**: Verify the hook installer tests are platform-correct and poison-tolerant.

**Steps (macOS — current dev machine)**:
```bash
# 1. Run the full suite locally
cargo test --test test_hooks_installer
# Expected: 7 passed; 0 failed

# 2. Run all tests
cargo test
# Expected: all suites pass
```

**Steps (Windows validation)**:
```bash
# The definitive check is the CI job. Push the fix branch and confirm:
# CI → test (windows-latest) → all 7 hook installer tests pass
```

**Poison-tolerance check (local)**:
Temporarily introduce a deliberate failure in one test (e.g., assert on a nonexistent path), run the file, and confirm:
- The failing test reports its genuine failure.
- The other 6 tests still RUN (not blocked by `PoisonError`).
- Revert the deliberate failure.

---

### Scenario 2: PR Target Check Permissions

**Purpose**: Verify the workflow has correct permissions and enforces the develop-targeting convention.

**Steps**:
```bash
# 1. Verify the workflow file declares the permissions block
grep -A3 "^permissions:" .github/workflows/pr-target-check.yml
# Expected:
#   contents: read
#   issues: write
#   pull-requests: write

# 2. Simulate a mis-targeted PR (base=main) — open a test PR targeting main,
#    or trigger via a temporary branch, and observe:
gh pr create --base main --head <some-branch> --title "test wrong-base" --body "temp"
gh run list --workflow "PR Target Branch Check" --limit 1
# Expected: workflow run SUCCEEDS (not "Resource not accessible by integration")
gh pr view <pr-number> --json labels
# Expected: labels includes "wrong-base"
gh pr view <pr-number> --json comments
# Expected: comment mentions targeting develop + CONTRIBUTING.md link

# 3. Close the temp PR
gh pr close <pr-number> --delete-branch
```

---

### Scenario 3: Correctly-Targeted PR Skips Cleanly

**Purpose**: Verify no false positives for PRs targeting develop.

**Steps**:
```bash
# Open a PR targeting develop (normal flow)
gh pr create --base develop --head <feature-branch> --title "normal" --body "work"
gh run list --workflow "PR Target Branch Check" --limit 1
# Expected: workflow run SKIPPED (job skipped by if guard) or success with no label
gh pr view <pr-number> --json labels
# Expected: no "wrong-base" label
```

---

### Scenario 4: CI Green Gate

**Purpose**: Verify the fix PR itself satisfies the CI gate under branch protection.

**Steps**:
```bash
# Push the fix branch and open a PR targeting develop
# Confirm all CI checks pass:
gh pr checks <pr-number>
# Expected: fmt ✅, clippy ✅, test (ubuntu) ✅, test (windows) ✅,
#            test (macos) ✅, security ✅, test presence ✅
```

---

## Success Criteria Mapping

| SC | Verified By |
|---|---|
| SC-001 (Windows tests 100% pass) | Scenario 1 (CI windows job) |
| SC-002 (no cascade from single failure) | Scenario 1 (poison check) |
| SC-003 (PR target check succeeds) | Scenario 2 |
| SC-004 (correct PR skips cleanly) | Scenario 3 |
| SC-005 (no new macOS/Linux failures) | Scenario 1 step 2 + Scenario 4 |
| SC-006 (fmt/clippy/test green before merge) | Scenario 4 |
