# Contract: CD Release Automation & Develop-to-Main Promotion

**Feature**: 016-fix-cd-promotion
**Contract type**: CI/CD workflow behavior contract

## Synopsis

`.github/workflows/cd.yml` MUST (a) make release-please analyze develop's commits and create release PRs against develop, and (b) automatically promote a released develop state to main via a fast-forward merge (or PR fallback).

## Release-Please Contract

| Requirement | Configuration |
|---|---|
| `target-branch` | `develop` (analyzes develop commits; builds release PR against develop) |
| Trigger | `push` (develop + main) and `workflow_dispatch` on develop |
| Release PR branch | `release-please--branches--develop--components--rgt` |
| Version source | Conventional commits since last release + `.release-please-manifest.json` |

## Promotion Contract

| Condition | Behavior |
|---|---|
| `release_created == 'true'` and main is ancestor of develop | `git push origin develop:main` (fast-forward) |
| `release_created == 'true'` and main diverged | `gh pr create` develop → main (no force-push) |
| `release_created == 'false'` | Job skipped |

**Prohibited**: force-pushing main, promoting unreleased work, blocking on macOS-gated build jobs.

## Job Dependencies

- `promote-to-main` needs: `release-please` (NOT `build-release`/`update-latest-tag`).
- `build-release` needs: `release-please`, if `release_created`.
- `update-latest-tag` needs: `release-please` + `build-release`, if `release_created`.

## Acceptance Mapping

| Acceptance Scenario | Contract Clause |
|---|---|
| US1/AC1 (release-please analyzes develop) | Release-Please Contract (target-branch) |
| US1/AC2 (release PR targets develop) | Release-Please Contract |
| US1/AC3 (release created on PR merge) | Release-Please Contract |
| US1/AC4 (no stale main PR confusion) | Branch cleanup (FR-008) |
| US2/AC1 (promotion after release) | Promotion Contract |
| US2/AC2 (fast-forward → 0 behind) | Promotion Contract |
| US2/AC3 (divergence → clear failure/PR) | Promotion Contract |
| US2/AC4 (skip when no release) | Promotion Contract (skip) |
| US2/AC5 (PR fallback allowed) | Promotion Contract |
| US3/AC1-AC3 (clean state) | Branch cleanup + release-please log |
