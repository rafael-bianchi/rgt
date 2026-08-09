# Feature Specification: Fix CD Release Automation & Develop-to-Main Promotion

**Feature Branch**: `fix/016-cd-promotion`

**Created**: 2026-08-09

**Status**: Draft

**Input**: User description: "Rework the CD workflow so release-please automation correctly promotes develop to main. The current `target-branch: main` configuration causes release-please to analyze main's commit history (not develop's) when triggered on develop pushes, so it finds zero new commits and never creates a promotion PR. The correct architecture is: release-please targets `develop` (analyzes develop commits, creates release PRs against develop), plus a separate promotion step that merges develop → main after a release."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Release-Please Sees Develop Commits (Priority: P1)

A developer merges feature work to `develop` (e.g., PR #37, #38). The CD workflow runs on the develop push. Release-please must analyze **develop's** commits (not main's) so it can compute the next version and create a release PR. Currently, `target-branch: main` makes release-please analyze main's history — which has no new commits since the last release — so it reports "0 commits, skipping" and never promotes.

**Why this priority**: This is the root cause of the automation failing. Without release-please seeing develop commits, no release PR is ever generated, and the develop → main promotion requires manual intervention (as happened with PRs #35, #39).

**Independent Test**: Merge a conventional-commit change to `develop` (e.g., a `feat:` commit), wait for the CD run, and observe that release-please creates a release PR. The release-please job log must show it collected develop's commits (nonzero commit count) and opened/updated a PR.

**Acceptance Scenarios**:

1. **Given** the CD workflow runs on a `develop` push, **When** release-please executes, **Then** it analyzes develop's commit history and reports a nonzero commit count for the `"."` path.
2. **Given** develop contains conventional commits since the last release, **When** release-please executes, **Then** it creates or updates a release PR (branch `release-please--branches--develop--components--rgt`) targeting `develop`.
3. **Given** the release PR is merged to develop, **When** the merge triggers a CD run, **Then** release-please detects the release commit and creates the GitHub release (tag `vX.Y.Z`).
4. **Given** a CD run on a `main` push, **When** release-please executes, **Then** it does not re-create a stale PR or get confused by leftover release-please state from the old configuration.

---

### User Story 2 - Automatic Develop-to-Main Promotion After Release (Priority: P1)

After release-please creates a release from `develop` (version bump committed to develop, tag created), the changes must automatically reach `main`. Currently there is no promotion step — develop and main drift apart, requiring manual promotion PRs. A dedicated job must merge develop → main after each successful release.

**Why this priority**: The whole point of the CD automation is that main reflects released develop work. Without the promotion step, main stays stale (as observed: main lagged by 11+ commits and required manual PRs #35 and #39).

**Independent Test**: After a release is created on develop, observe that a develop → main merge (or PR) is opened automatically by the CD workflow. `main` becomes 0 commits behind `develop` without manual intervention.

**Acceptance Scenarios**:

1. **Given** release-please has created a release (`release_created == 'true'`), **When** the CD workflow continues, **Then** a promotion job runs that merges `develop` into `main`.
2. **Given** the promotion merge is a fast-forward (main has not diverged), **When** the job runs, **Then** main is updated directly and ends 0 commits behind develop.
3. **Given** main has diverged (e.g., a CHANGELOG conflict), **When** the promotion runs, **Then** it fails gracefully with a clear error rather than silently corrupting main, and a human can resolve.
4. **Given** no release was created, **When** the workflow runs, **Then** the promotion job is skipped (no premature develop → main merge for unreleased work).
5. **Given** the repo setting "Allow GitHub Actions to create and approve pull requests" (per constitution CI/CD Gating), **When** the promotion cannot fast-forward, **Then** the job may open a PR develop → main instead of pushing directly.

---

### User Story 3 - Clean State for Future Releases (Priority: P2)

The leftover release-please state from the transition (the `release-please--branches--develop--components--rgt` branch at an old SHA, stale PR references) must not interfere with the new flow. A fresh release cycle should work end-to-end with no manual cleanup.

**Why this priority**: The transition left stale state that confused release-please (it tracked PR #29/36 incorrectly). Cleaning state prevents recurring confusion.

**Independent Test**: After the rework, trigger a fresh release cycle and confirm no stale PR references or orphaned branches cause errors.

**Acceptance Scenarios**:

1. **Given** the reworked workflow, **When** a release cycle completes, **Then** no orphaned stale release-please branch remains on the remote (the `release-please--branches--develop--components--rgt` branch from the old configuration is removed).
2. **Given** release-please runs, **When** it looks for the latest release PR, **Then** it finds the current PR (#36 or the new one) without confusion from deleted/merged PRs.
3. **Given** a workflow_dispatch on develop, **When** release-please runs, **Then** it produces a clean candidate release PR (nonzero commits, correct version bump).

---

### Edge Cases

- **Release PR merge into develop triggers the promotion**: Merging the release PR into develop fires a CD run on develop. That run's release-please creates the release, and the promotion job then merges develop → main. The sequence must be ordered so promotion happens after the release is created.
- **Concurrent develop pushes**: The `concurrency` group (`cd-${{ github.ref }}`, cancel-in-progress for non-main) must remain so rapid develop pushes don't race the release/promotion sequence.
- **`workflow_dispatch` on develop**: Must behave like a push: analyze develop commits, create/update release PR.
- **Main divergence**: If main has commits not in develop (rare, since all work goes through develop), the promotion must not force-push or destroy them — it should either merge safely or open a PR.
- **No conventional commits since last release**: Release-please correctly does nothing (no new version needed). The promotion job must also skip (no release created).
- **macOS runner scarcity**: The pre-release build jobs already wait on free-tier macOS runners; the promotion job must not be gated behind them (it should run after `release-please`, not after the build jobs).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The release-please job in `.github/workflows/cd.yml` MUST use `target-branch: develop` (not `main`) so it analyzes develop's commit history when triggered on develop pushes.
- **FR-002**: Release-please MUST create/update a release PR targeting `develop` (branch `release-please--branches--develop--components--rgt`) when develop contains conventional commits since the last release.
- **FR-003**: When the release PR merges to develop and release-please creates a release, the CD workflow MUST run a promotion job that brings develop's state to `main`.
- **FR-004**: The promotion job MUST run only when `release_created == 'true'` (after a release exists) — never for unreleased develop work.
- **FR-005**: The promotion job MUST prefer a fast-forward merge of `develop` into `main`; if main has diverged, it MUST open a PR (or fail with a clear message) rather than force-push or overwrite main.
- **FR-006**: The promotion job MUST NOT depend on the pre-release build jobs (which are gated on slow macOS runners) — it should be ordered immediately after the release-please job.
- **FR-007**: The `release-please` job condition MUST cover `push` (develop and main) and `workflow_dispatch` on develop, consistent with current behavior, but with the corrected `target-branch`.
- **FR-008**: The stale `release-please--branches--develop--components--rgt` remote branch (leftover from the old configuration, verified at SHA `c68f702`) MUST be deleted as part of this change so release-please starts from a clean state.
- **FR-009**: A fresh end-to-end release cycle (conventional commit → release PR → merge → release → promote to main) MUST complete with zero manual intervention.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A test `feat:` commit merged to develop produces a release PR within one CD run (release-please reports nonzero commits).
- **SC-002**: After the release PR is merged, `main` reaches 0 commits behind `develop` without any manual promotion PR (within one additional CD run).
- **SC-003**: The CD workflow's release-please job log shows `commits: N` (N > 0) for a develop push with conventional commits, and `Building candidate release pull request` — not "No commits, skipping".
- **SC-004**: The stale `release-please--branches--develop--components--rgt` remote branch is absent after the cleanup.
- **SC-005**: No existing CI behavior regresses: feature PRs still pass CI, and the `pre-release`/`build-prerelease` jobs still produce dev pre-releases on develop.
- **SC-006**: The rework is validated with a real release cycle within the repo (or a documented dry-run) before the feature is marked complete.

## Assumptions

- The release-please-action `target-branch` parameter controls which branch's commits are analyzed for the next version. The pre-fix behavior (`${{ github.ref_name }}` → develop on develop pushes) was correct for analysis; the missing piece was the develop → main promotion, not the analysis target.
- The repo setting "Allow GitHub Actions to create and approve pull requests" is enabled (per constitution CI/CD Gating), so release-please PRs and any promotion PR are allowed.
- The `release.yml` reusable workflow (used for both prerelease and release asset builds) needs no changes — only `cd.yml` changes.
- Merging `develop` into `main` via a fast-forward `git push` is acceptable for the promotion job; opening a PR is the fallback for divergence.
- The manifest `.release-please-manifest.json` (`{"rgt":"0.1.0",".":"0.3.0"}`) is left as-is unless the rework reveals a versioning inconsistency that blocks the release cycle.
