# Data Model: Fix Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Feature**: 017-fix-release-workflow

## Overview

No runtime data model. This document formalizes the workflow jobs, their triggers, and the build-source invariants.

## Entities

### Workflow Jobs (`.github/workflows/cd.yml`)

| Job | Current Trigger | New Trigger | Change |
|---|---|---|---|
| `pre-release` | develop push OR workflow_dispatch (non-main) | workflow_dispatch (non-main) ONLY | Gate removed push trigger |
| `build-prerelease` | needs `pre-release` | unchanged | — |
| `release-please` | push / workflow_dispatch | unchanged | — |
| `build-release` | needs `release-please`, if `release_created` | unchanged | — |
| `update-latest-tag` | needs `release-please`+`build-release`, if `release_created` | unchanged | — |
| `promote-to-main` | needs `release-please`, if `release_created` | unchanged | — |

### Build Jobs (`.github/workflows/release.yml`)

| Job | Compiles Binary? | Checkout Needs Tag Ref? | Change |
|---|---|---|---|
| `build` (matrix) | YES | YES | Add `ref: ${{ inputs.tag }}` |
| `build-deb` | YES | YES | Add `ref: ${{ inputs.tag }}` |
| `build-rpm` | YES | YES | Add `ref: ${{ inputs.tag }}` |
| `release` (Create GitHub Release) | NO (downloads artifacts) | NO | — |
| `homebrew` | NO (computes SHAs) | NO | — |

## Invariants

- **Build-source invariant**: Every binary compiled for release vX.Y.Z is compiled from commit `refs/tags/vX.Y.Z`, never from a branch head.
- **Pre-release invariant**: A dev pre-release (`dev-vX.Y.Z-rc.N`) is produced ONLY by manual `workflow_dispatch` on a non-main branch.
- **Consumer invariant**: `rgt update` resolves `/releases/latest` (stable only); Homebrew updates only on stable releases. Dev pre-releases are invisible to both.

## State Transitions

### Build Source

```
develop@HEAD ──(before fix)──► binaries built from branch (WRONG)
tag vX.Y.Z ──(after fix)─────► binaries built from tag commit (CORRECT)
```

### Dev Pre-release Trigger

```
push to develop ──(before fix)──► dev pre-release built (WASTE)
workflow_dispatch ──(always)────► dev pre-release built (ON-DEMAND)
```
