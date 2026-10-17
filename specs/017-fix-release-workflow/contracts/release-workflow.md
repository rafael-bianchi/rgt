# Contract: Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Feature**: 017-fix-release-workflow
**Contract type**: CI/CD workflow behavior contract

## Synopsis

`.github/workflows/release.yml` MUST compile release binaries from the exact release tag. `.github/workflows/cd.yml` MUST build dev pre-releases only on manual dispatch.

## Tag-Based Build Contract

| Job | Requirement |
|---|---|
| `build` (matrix) | `actions/checkout@v4` with `ref: ${{ inputs.tag }}` |
| `build-deb` | `actions/checkout@v4` with `ref: ${{ inputs.tag }}` |
| `build-rpm` | `actions/checkout@v4` with `ref: ${{ inputs.tag }}` |
| `release` / `homebrew` | No tag ref needed (don't compile binaries) |

**Invariant**: For release tag `vX.Y.Z`, all binaries are compiled from `refs/tags/vX.Y.Z`.

## Pre-Release Gating Contract

| Event | `pre-release` job |
|---|---|
| `push` to `develop` | SKIPPED |
| `workflow_dispatch` on non-main branch | RUNS (dev pre-release produced) |
| `workflow_dispatch` on `main` | SKIPPED |

## Idempotency Contract

- Re-running `release.yml` for an existing tag re-attaches assets via `softprops/action-gh-release@v2` (overwrites same-named assets) — no error, no duplicate release.
- Cancelled asset builds can be re-run safely.

## Consumer Contracts (unchanged)

- `rgt update` → `/releases/latest` → latest stable (non-prerelease) release and its assets.
- Homebrew formula update → runs only for stable releases (`!prerelease`).

## Acceptance Mapping

| Acceptance Scenario | Contract Clause |
|---|---|
| US1/AC1 (tag commit checked out) | Tag-Based Build Contract |
| US1/AC2 (workflow_call uses inputs.tag) | Tag-Based Build Contract |
| US1/AC3 (rgt update binary matches tag) | Consumer Contracts |
| US2/AC1 (push skips pre-release) | Pre-Release Gating Contract |
| US2/AC2 (dispatch runs pre-release) | Pre-Release Gating Contract |
| US2/AC3 (no cancel interference) | Pre-Release Gating + Idempotency |
