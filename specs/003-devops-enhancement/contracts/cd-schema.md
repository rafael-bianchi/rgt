# CD Release Orchestration Contract: `cd.yml`

**Feature Branch**: `003-devops-enhancement` | **Date**: 2026-07-31

## Workflow Triggers

```yaml
on:
  push:
    branches: [develop, main]
  workflow_dispatch:
```

## Concurrency

```yaml
concurrency:
  group: cd-${{ github.ref }}
  cancel-in-progress: ${{ github.ref != 'refs/heads/main' }}
```

## Pre-release Path (`develop`)

1. Compute version from conventional commits since last stable tag:
   - Parse `fix:`, `feat:`, breaking change indicators
   - Pre-1.0: `feat` → minor bump, `fix` → patch bump, breaking → minor bump
   - Post-1.0: breaking → major, `feat` → minor, `fix` → patch
2. Generate tag: `dev-{major}.{minor}.{patch}-rc.{github.run_number}`
3. Call `release.yml` with `prerelease: true`

## Stable Release Path (`main`)

1. Run `release-please-action@v4`:
   - Release type: `rust`
   - Token: GitHub App token (permission-contents: write, permission-pull-requests: write)
2. If release created:
   - Call `release.yml` with `tag: {release_tag}`, `prerelease: false`
   - Update floating `latest` tag to point to new release

## Reusable Release Workflow (`release.yml`) Interface

```yaml
on:
  workflow_call:
    inputs:
      tag:
        required: true
        type: string
      prerelease:
        required: false
        type: boolean
        default: false
  workflow_dispatch:
    inputs:
      tag:
        required: true
      prerelease:
        type: boolean
        default: false
```

### Called Workflow Outputs

- Builds 5 platform binaries + DEB + RPM packages
- Generates `checksums.txt`
- Creates GitHub Release with all assets
- On stable release: updates Homebrew tap, sends Discord notification, updates `latest` tag

## Manual Release Override (FR-012)

Triggered via `workflow_dispatch` on `release.yml`:
- Input: `tag` (e.g., `v0.2.1`) and `prerelease` flag
- Bypasses conventional-commit analysis
- Runs the identical build pipeline
