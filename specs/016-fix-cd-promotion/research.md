# Research: Fix CD Release Automation & Develop-to-Main Promotion

**Feature**: 016-fix-cd-promotion
**Date**: 2026-08-09

## Decision: Revert `target-branch` to `develop`

**Decision**: Change `target-branch: main` → `target-branch: develop` in `.github/workflows/cd.yml` line 99.

**Rationale**: The release-please-action analyzes the commit history of the **target branch** when determining the next version and building the release PR. With `target-branch: main`, a CD run triggered by a `develop` push analyzes **main's** history — which has no new commits since the last release — so it reports "Splitting 0 commits by path" / "No commits for path: ., skipping" and never creates a promotion PR. This was confirmed empirically in run 31338105676's release-please log:

```
Fetching merge commits on branch main ...
release for path: ., version: 0.3.1, sha: e957531...
❯ commits: 0
✔ No commits for path: ., skipping
```

With `target-branch: develop`, the action analyzes develop's commits (the correct source of new work) and builds the release PR against develop. This matches the pre-fix behavior that correctly produced PR #29 (`chore(develop): release 0.3.0`).

**Alternatives considered**:
- Keep `target-branch: main` and add a pre-merge of develop → main before release-please runs: Rejected — complex ordering, and release-please would still analyze main (which after the merge WOULD have the commits, but this couples the promotion before the release PR exists, which is backwards).
- `target-branch: ${{ github.ref_name }}`: Equivalent to `develop` on develop pushes, but explicit `develop` is clearer and deterministic for `workflow_dispatch`.

## Decision: Add a `promote-to-main` job gated on `release_created`

**Decision**: Add a new job `promote-to-main` that runs `needs: release-please`, `if: needs.release-please.outputs.release_created == 'true'`, and merges `develop` into `main`.

**Rationale**: Once release-please creates a release (version bump committed to develop + tag created), main must catch up. Gating on `release_created` ensures only *released* work is promoted — never unreleased develop state (FR-004).

**Flow**:
```
develop push → release-please → release PR (against develop) → merged
  → CD run on develop merge → release-please creates release (release_created=true)
  → promote-to-main job → git push origin develop:main (fast-forward)
```

**Alternatives considered**:
- Promote on every develop push: Rejected — would push unreleased work to main, violating FR-004 and the "releases only" convention.
- Promote via release-please's own config (no separate job): release-please-action does not natively merge develop → main; a separate step is required.

## Decision: Fast-forward merge with PR fallback

**Decision**: The `promote-to-main` job first checks whether `main` is an ancestor of `develop` (fast-forward possible). If yes, `git push origin develop:main`. If main has diverged, it creates a PR `develop` → `main` via `gh pr create` instead of force-pushing.

**Rationale**: All work flows through develop (constitution trunk-based), so main is normally an ancestor of develop → fast-forward works and is clean. If main ever diverges (e.g., a manual hotfix), force-pushing would destroy that work; opening a PR is the safe fallback (FR-005). The `gh` CLI is available on GitHub-hosted ubuntu runners, and the workflow already has `pull-requests: write` permission.

**Alternatives considered**:
- Always force-push `develop:main`: Rejected — destroys any main-only commits (violates FR-005, spec Edge Case "Main divergence").
- Always open a PR: Rejected — adds review friction for the common fast-forward case; direct push is appropriate when safe.

## Decision: Promotion job independent of build jobs

**Decision**: `promote-to-main` has `needs: release-please` only (not `needs: build-release`).

**Rationale**: The `build-release` job uses the reusable `release.yml` which builds release assets; those asset-build jobs include macOS builds that stall on free-tier macOS runner scarcity (observed: CD runs queued 30-45+ min waiting for macOS runners). The promotion is a pure git operation that doesn't depend on artifacts, so gating it behind the macOS build would needlessly delay the develop → main merge (FR-006, spec Edge Case).

**Alternatives considered**:
- `needs: [release-please, build-release]`: Rejected — couples a fast git op to slow asset builds; delays main promotion.
- `needs: [release-please, update-latest-tag]`: Rejected — update-latest-tag also depends on build-release.

## Decision: Clean up stale release-please branch

**Decision**: Delete the `release-please--branches--develop--components--rgt` remote branch (SHA `c68f702`) as part of implementation (FR-008).

**Rationale**: This branch is leftover from the pre-fix configuration. The manifest version references PR #29's release; the stale branch confused release-please during the transition (it reported "Found latest release pull request: 29 version: 0.3.0" in later runs). Starting fresh with only the new flow's branches avoids recurring confusion.

**Note**: There is currently NO `release-please--branches--main--...` branch on the remote (PR #36's branch was deleted on merge). The only stale branch is the develop one.

**Alternatives considered**:
- Leave it: Rejected — stale state already caused confusion in release-please runs (spec US3).
- Re-point it: Not applicable — release-please recreates its branch as needed.

## Decision: Validate with a real release cycle

**Decision**: After implementing, verify with a real (or documented dry-run) release cycle per SC-006: confirm a develop push with conventional commits produces a release PR (SC-001/SC-003), and after merging, main reaches 0 behind develop automatically (SC-002).

**Rationale**: Workflow changes are only truly validated by exercising them. The repo's live CD runs provide the deterministic evidence (release-please logs, PR creation, branch sync counts).

**Alternatives considered**:
- Static YAML review only: Rejected — the empirical failure (0 commits bug) was only caught by running; static review is insufficient for workflow behavior.
