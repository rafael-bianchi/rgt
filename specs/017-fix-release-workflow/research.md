# Research: Fix Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Feature**: 017-fix-release-workflow
**Date**: 2026-08-10

## Decision: `release.yml` binary-build jobs checkout `ref: ${{ inputs.tag }}`

**Decision**: Add `with: { ref: ${{ inputs.tag }} }` to the `actions/checkout@v4` step in the three binary-compiling jobs of `.github/workflows/release.yml`:
- `build` matrix job (line 58)
- `build-deb` job (line 130)
- `build-rpm` job (line 154)

**Rationale**: In a reusable workflow (`workflow_call`), `actions/checkout@v4` with no `ref` resolves to the **caller's** triggering ref — which for `cd.yml`'s `build-release` job is the `develop` branch, not the release tag. If develop moves ahead of the tag between the release PR merge and the build job running, binaries get compiled from the wrong commit. This was confirmed when the v0.3.2 assets were cancelled mid-build and a manual `--ref develop` trigger would have built from a develop commit ahead of the tag. Pinning `ref: ${{ inputs.tag }}` guarantees the build uses the exact tagged commit (FR-001, SC-001).

**Which checkouts need it**: Only the 3 jobs that compile Rust binaries. The `release` job (line 179) only downloads artifacts and uploads them — its checkout ref does not affect binary content. The `homebrew` job (line 220) only computes SHA256s from already-uploaded assets — same. Adding the ref there would be harmless but unnecessary; keeping the change minimal reduces review surface.

**Alternatives considered**:
- Use `${{ github.ref }}`/`${{ github.sha }}` in the build jobs: Rejected — in `workflow_call` these resolve to the caller's context, not the reusable workflow's input. `inputs.tag` is the explicit, correct source.
- `git checkout ${{ inputs.tag }}` as a run step after checkout: Rejected — the checkout action's `ref` input is the idiomatic way; a manual git step adds a second checkout pass.
- Rebuild via a dedicated release-only workflow: Rejected — over-engineering; the tag ref on the existing 3 jobs is the minimal correct fix.

## Decision: Gate `pre-release` job to `workflow_dispatch` only

**Decision**: Change the `pre-release` job `if` condition in `.github/workflows/cd.yml` from:

```yaml
if: >-
  github.ref == 'refs/heads/develop'
  || (github.event_name == 'workflow_dispatch' && github.ref != 'refs/heads/main')
```

to:

```yaml
if: >-
  github.event_name == 'workflow_dispatch' && github.ref != 'refs/heads/main'
```

**Rationale**: Dev pre-releases (`dev-vX.Y.Z-rc.N`) have zero consumers: `rgt update` resolves `/releases/latest` which excludes prereleases, Homebrew only uses stable releases, and no workflow downloads `dev-` tags. Building them on every develop push consumes scarce free-tier macOS runner time and, via the `concurrency` cancel rule, can cancel the legitimate release asset build (this is exactly how the v0.3.2 assets were cancelled). Gating to `workflow_dispatch` preserves the on-demand path (FR-002/FR-003) while eliminating the waste and the cancel hazard (SC-002/SC-005).

**Alternatives considered**:
- Remove pre-release builds entirely: Rejected — the on-demand path is useful for manually testing a pre-release from a branch; gating (not deleting) preserves it.
- Build dev pre-releases only on tags: Rejected — dev pre-releases are meant for branch-state testing; tag events are already covered by stable release builds.
- Rate-limit by time: Rejected — GitHub Actions has no native cron-like push throttling; `workflow_dispatch`-only is the clean gate.

## Decision: Idempotent re-runs for existing tags (no workflow change needed)

**Decision**: Re-running `release.yml` for an existing tag re-uploads assets via `softprops/action-gh-release@v2`, which overwrites existing assets for the same tag.

**Rationale**: `softprops/action-gh-release` updates the release for `tag_name` and overwrites files with the same name. This makes re-triggering after a cancellation (e.g., the v0.3.2 case) safe and idempotent (FR-005). No additional workflow logic is needed; the feature validation will exercise exactly this path (SC-006).

**Alternatives considered**:
- Delete-then-recreate release: Rejected — destructive; softprops overwrite is sufficient and non-destructive.
- Dedup guard in workflow: Rejected — softprops already handles it; extra logic adds complexity without benefit.

## Decision: No consumer-side changes

**Decision**: `rgt update` and the Homebrew formula update require no changes.

**Rationale**: `rgt update` calls `/releases/latest` (`src/updater/github.rs:8`), which GitHub defines as the latest non-prerelease, non-draft release. Dev pre-releases are excluded automatically. Homebrew only updates on stable (`!prerelease`) releases. Gating dev pre-releases and pinning tag refs does not alter these contracts (FR-006).

**Alternatives considered**:
- Update `rgt update` to also check dev pre-releases: Rejected — out of scope; dev pre-releases are not meant for end-user distribution.
- Change Homebrew formula logic: Rejected — unaffected by these changes.

## Decision: Validate live with a real tag build

**Decision**: After implementing, validate by re-triggering `release.yml` for an existing tag (e.g., `v0.3.2`) and confirming (a) the checkout resolves to the tag commit and (b) assets attach successfully.

**Rationale**: The earlier v0.3.2 asset build was cancelled by concurrency; re-running it from the tag both fixes the immediate gap and validates FR-001/FR-005/SC-001/SC-004/SC-006 in one live exercise.

**Alternatives considered**:
- Static YAML review only: Rejected — workflow behavior is only proven by running; the tag-ref correctness is empirically testable via the resolved checkout SHA.
