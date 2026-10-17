# Quickstart Validation: Fix CD Release Automation & Develop-to-Main Promotion

**Feature**: 016-fix-cd-promotion

This guide documents runnable validation scenarios that prove the feature works end-to-end.

## Prerequisites

- `gh` CLI authenticated with repo access
- GitHub Actions enabled
- Repo setting "Allow GitHub Actions to create and approve pull requests" enabled
- Access to push to `develop` and `main`

## Scenarios

### Scenario 1: Release-Please Analyzes Develop Commits

**Purpose**: Verify the `target-branch: develop` fix makes release-please see develop's commits.

**Steps**:
```bash
# 1. Merge a conventional commit to develop (e.g., a small fix or chore)
#    (via PR targeting develop)

# 2. Find the CD run triggered by the merge
gh run list --workflow=CD --limit 1

# 3. Inspect the release-please job log
RUN_ID=$(gh run list --workflow=CD --limit 1 --json databaseId --jq '.[0].databaseId')
JOB_ID=$(gh api repos/rafael-bianchi/rgt/actions/runs/$RUN_ID/jobs --jq '.jobs[] | select(.name=="Release Please") | .id')
gh api repos/rafael-bianchi/rgt/actions/jobs/$JOB_ID/logs | grep -E "commits:|candidate release pull request|No commits"
```

**Pass**: The log shows `commits: N` (N > 0) and `Building candidate release pull request` — NOT "No commits, skipping".

---

### Scenario 2: Release PR Created Against Develop

**Purpose**: Verify release-please creates a PR targeting develop.

**Steps**:
```bash
gh pr list --base develop --state open
# Expected: a PR titled "chore(develop): release vX.Y.Z" from
#   release-please--branches--develop--components--rgt
```

**Pass**: Release PR exists, base = develop.

---

### Scenario 3: Auto Promotion to Main (full cycle)

**Purpose**: Verify the end-to-end cycle: merge release PR → release created → main promoted automatically.

**Steps**:
```bash
# 1. Merge the release PR (Scenario 2)
gh pr merge <release-pr-number> --merge

# 2. Wait for the CD run on the develop merge to complete
#    (the release-please job creates the release; promote-to-main job runs)

# 3. Verify main caught up
git fetch origin main develop
git rev-list --count origin/main..origin/develop
# Expected: 0

# 4. Verify the release exists
gh release list --limit 1
# Expected: vX.Y.Z release present
```

**Pass**: `0` commits behind, release present, no manual promotion needed.

---

### Scenario 4: No Promotion for Unreleased Work

**Purpose**: Verify unreleased develop changes are never pushed to main.

**Steps**:
```bash
# 1. Push a conventional commit to develop WITHOUT merging a release PR
# 2. Check the CD run's jobs
gh run view <run-id> --json jobs --jq '.jobs[].name'
```

**Pass**: `promote-to-main` job appears SKIPPED (or absent) when `release_created == 'false'`; main unchanged.

---

### Scenario 5: Clean State — No Stale Branches

**Purpose**: Verify no stale release-please branches remain.

**Steps**:
```bash
git ls-remote --heads origin | grep -i "release-please"
```

**Pass**: The stale `release-please--branches--develop--components--rgt` branch (SHA `c68f702`) from the old configuration is **absent**. Only a branch re-created by the new flow may exist (e.g., if a release PR is currently open), and it must point at a current SHA, not the stale one.

---

## Success Criteria Mapping

| SC | Verified By |
|---|---|
| SC-001 (conventional commit → release PR) | Scenarios 1-2 |
| SC-002 (main 0 behind automatically) | Scenario 3 |
| SC-003 (release-please shows commits > 0) | Scenario 1 |
| SC-004 (no stale main branch) | Scenario 5 |
| SC-005 (no CI regression) | Feature PRs still pass CI; prerelease jobs unchanged |
| SC-006 (real release cycle validated) | Scenario 3 |
