# Data Model: Fix CD Release Automation & Develop-to-Main Promotion

**Feature**: 016-fix-cd-promotion

## Overview

This feature has no runtime data model — it reworks the CI/CD workflow. This document formalizes the workflow state machine and the entities it manages, so the contracts and quickstart can reference concrete states.

## Entities

### Branches

| Branch | Role | Update Trigger |
|---|---|---|
| `develop` | Integration branch (all PRs target this) | PR merges (features, fixes) |
| `main` | Release branch (stable releases only) | Promotion job after release-please creates a release |

### Jobs (in `.github/workflows/cd.yml`)

| Job | Trigger / Needs | Output |
|---|---|---|
| `pre-release` | develop push / workflow_dispatch (non-main) | `tag` (dev-vX.Y.Z-rc.N) |
| `build-prerelease` | needs `pre-release` | dev pre-release assets |
| `release-please` | push / workflow_dispatch | `release_created`, `tag_name` |
| `build-release` | needs `release-please`, if `release_created` | release assets |
| `update-latest-tag` | needs `release-please`, `build-release`, if `release_created` | moves `latest` tag |
| `promote-to-main` (NEW) | needs `release-please`, if `release_created` | merges develop → main |

## State Transitions

### Release Cycle State Machine

```
                    conventional commit merged
                    ┌─────────────────────────────┐
                    ▼                             │
    [idle] ──develop push──► [release-please analyzes develop]
                                      │ nonzero commits
                                      ▼
                             [release PR created/updated (targets develop)]
                                      │ PR merged
                                      ▼
                             [CD run on develop merge]
                                      │ release-please sees release commit
                                      ▼
                              [release created (tag vX.Y.Z)]
                                      │ release_created=true
                                      ▼
                        [promote-to-main: git push develop:main]
                                      │
                                      ▼
                               [main == develop (0 behind)]
                                      │
                                      └──► [idle] (next cycle)
```

### Promotion Decision Table

| Condition | Action |
|---|---|
| `release_created == 'true'` AND main is ancestor of develop | Fast-forward `git push origin develop:main` |
| `release_created == 'true'` AND main has diverged | Open PR `develop` → `main` (no force-push) |
| `release_created == 'false'` | Skip promotion (unreleased work stays on develop) |

### Concurrency

- Group: `cd-${{ github.ref }}`
- `cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}` — develop runs cancel each other; main runs never cancel.

## Validation Invariants

- After a release cycle, `git rev-list --count origin/main..origin/develop` == 0.
- Release-please log for a develop push with conventional commits shows `commits: N` (N>0), never "No commits, skipping".
- The stale `release-please--branches--develop--components--rgt` remote branch from the old configuration is removed; only branches created by the new flow remain.
- `main` never receives a force-push (only fast-forward or PR).
