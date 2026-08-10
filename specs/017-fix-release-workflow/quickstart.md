# Quickstart Validation: Fix Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Feature**: 017-fix-release-workflow

This guide documents runnable validation scenarios that prove the feature works end-to-end.

## Prerequisites

- `gh` CLI authenticated with repo access
- GitHub Actions enabled
- Access to `develop` and an existing release tag (e.g., `v0.3.2`)

## Scenarios

### Scenario 1: Release Binaries Built From the Tag

**Purpose**: Verify the build checkout resolves to the tag commit, not a branch head.

**Steps**:
```bash
# 1. Ensure develop is ahead of the tag (it is — develop has post-tag commits)
# 2. Trigger a release build for the existing tag
gh workflow run release.yml --ref v0.3.2 -f tag=v0.3.2 -f prerelease=false

# 3. Find the build job and inspect its checkout step's resolved SHA
RUN_ID=$(gh run list --workflow=Release --limit 1 --json databaseId --jq '.[0].databaseId')
gh run view $RUN_ID --log-failed --job <build-job-id> 2>/dev/null | grep -i "checkout" | head -5
```

**Pass**: The checkout step resolves to commit `fbeb396` (the v0.3.2 tag), NOT a newer develop commit. The release ends with all platform binaries attached.

---

### Scenario 2: Push to Develop Skips Pre-Release

**Purpose**: Verify the dev pre-release build does not run on pushes.

**Steps**:
```bash
# 1. Push any commit to develop
# 2. Find the CD run it triggers
gh run list --workflow=CD --limit 1
RUN_ID=$(gh run list --workflow=CD --limit 1 --json databaseId --jq '.[0].databaseId')

# 3. Check the pre-release job
gh api repos/rafael-bianchi/rgt/actions/runs/$RUN_ID/jobs --jq '.jobs[] | select(.name=="Pre-release") | {name, status, conclusion}'
```

**Pass**: The `Pre-release` job shows SKIPPED (no dev pre-release built on push).

---

### Scenario 3: Manual Dispatch Still Produces Dev Pre-Release

**Purpose**: Verify the on-demand path is preserved.

**Steps**:
```bash
gh workflow run CD --ref develop
sleep 30
RUN_ID=$(gh run list --workflow=CD --limit 1 --json databaseId --jq '.[0].databaseId')
gh api repos/rafael-bianchi/rgt/actions/runs/$RUN_ID/jobs --jq '.jobs[] | select(.name=="Pre-release") | {name, status, conclusion}'
```

**Pass**: The `Pre-release` job RUNS (workflow_dispatch) and produces a `dev-vX.Y.Z-rc.N` tag.

---

### Scenario 4: Re-run of Existing Tag Is Idempotent

**Purpose**: Verify re-running `release.yml` for an existing tag re-attaches assets without error.

**Steps**:
```bash
gh workflow run release.yml --ref v0.3.2 -f tag=v0.3.2 -f prerelease=false
# Wait for completion, then:
gh release view v0.3.2 --json assets --jq '[.assets[].name]'
```

**Pass**: The release has all platform assets; the run completed without a "release already exists" error.

---

### Scenario 5: `rgt update` Resolves the Stable Release

**Purpose**: Verify consumers are unaffected.

**Steps**:
```bash
# After assets are attached to v0.3.2:
gh release view v0.3.2 --json assets --jq '.assets | length'
# Expected: >= 6 platform archives + checksums
```

**Pass**: `rgt update` on macOS/Linux/Windows finds its platform asset for the latest stable release.

---

## Success Criteria Mapping

| SC | Verified By |
|---|---|
| SC-001 (tag commit checked out) | Scenario 1 |
| SC-002 (push skips pre-release) | Scenario 2 |
| SC-003 (dispatch produces dev pre-release) | Scenario 3 |
| SC-004 (v0.3.2 binaries available) | Scenario 4-5 |
| SC-005 (no CI regression) | Feature PR CI + scenarios |
| SC-006 (re-triggered build completes) | Scenario 4 |
