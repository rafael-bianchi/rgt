# Feature Specification: Fix Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Feature Branch**: `fix/017-release-workflow`

**Created**: 2026-08-10

**Status**: Draft

**Input**: User description: "Implement two workflow cleanups: (1) gate the dev pre-release build so it no longer runs on every develop push (it has zero consumers and burns scarce macOS runners), and (2) make the release asset build checkout the release tag explicitly instead of the caller's branch ref, so binaries are always built from the exact tagged commit."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Release Binaries Always Match the Tag (Priority: P1)

A maintainer merges a release PR (e.g., `chore(develop): release 0.3.2`) into `develop`. The CD workflow's `build-release` job runs the reusable `release.yml` workflow to compile platform binaries and attach them to the GitHub Release. The binaries MUST be compiled from the exact release tag's commit (e.g., `v0.3.2` → `fbeb396`), not from whatever commit `develop` happens to point to when the job runs.

**Why this priority**: Currently `release.yml` uses `actions/checkout@v4` with no `ref`, so in a `workflow_call` it checks out the **caller's ref (develop)**, not the tag. If develop moves ahead of the tag between the release PR merge and the build job running, the binaries are built from the wrong commit — producing binaries that don't match the released version. This is a correctness bug that risks shipping binaries that don't correspond to their tag.

**Independent Test**: Trigger `release.yml` for a tag (e.g., `workflow_dispatch` with `ref: v0.3.2` and `tag: v0.3.2`) after develop has moved ahead of that tag. The build job's checkout MUST resolve to the tag commit, and the resulting release assets must be built from that commit. Verify by inspecting the checkout step's SHA or the binary's embedded version.

**Acceptance Scenarios**:

1. **Given** `release.yml` runs for tag `v0.3.2`, **When** the build job checks out, **Then** it checks out commit `fbeb396` (the tag) even if develop is at a newer commit.
2. **Given** a `workflow_call` from `cd.yml`'s `build-release` job, **When** `release.yml` runs, **Then** the checkout uses the tag passed via `inputs.tag`, not the calling workflow's branch.
3. **Given** the release asset build completes, **When** a user runs `rgt update`, **Then** the downloaded binary is built from the exact tagged commit (verifiable via the binary's reported version).

---

### User Story 2 - Dev Pre-Release Builds Run Only on Demand (Priority: P2)

A developer pushes feature work to `develop`. The CD workflow MUST NOT build a dev pre-release (`dev-vX.Y.Z-rc.N`) on every push, because those artifacts have no consumers — `rgt update` ignores prereleases (`/releases/latest` excludes them), Homebrew only uses stable releases, and nothing downloads the dev archives. Building them burns scarce macOS runner time and causes the concurrency cancel-churn that can interrupt the release asset build. The dev pre-release build should run only when explicitly requested (e.g., `workflow_dispatch`).

**Why this priority**: This is an efficiency fix. Every develop push currently queues 5 platform builds (2 macOS) that produce artifacts nobody uses. On free-tier GitHub Actions where macOS runners are scarce, this delays or cancels legitimate builds (including the release asset build). Gating it to on-demand runs frees runner capacity and removes the cancel-in-progress hazard.

**Independent Test**: Push a commit to `develop`. The CD run's `pre-release` job must be skipped. Then manually trigger the CD workflow via `workflow_dispatch` on develop; the `pre-release` job must run and produce a dev pre-release.

**Acceptance Scenarios**:

1. **Given** a `push` event to `develop`, **When** the CD workflow runs, **Then** the `pre-release` job is skipped (no dev pre-release built).
2. **Given** a `workflow_dispatch` event on `develop`, **When** the CD workflow runs, **Then** the `pre-release` job runs and produces a `dev-vX.Y.Z-rc.N` tag + artifacts.
3. **Given** the pre-release job is gated, **When** the release asset build runs for a stable release, **Then** it is not cancelled by a concurrent dev pre-release build from a newer develop push.

---

### Edge Cases

- **Tag-based workflow_dispatch**: `gh workflow run release.yml --ref <tag> -f tag=<tag>` must work — the `ref` (which branch/tag to check out) and the `tag` input (the release tag to attach assets to) must both resolve correctly.
- **Release PR merge and build ordering**: The release asset build runs on the develop merge that triggers `release_created == true`. Even if `promote-to-main` hasn't fast-forwarded main yet, the checkout must use the tag, not wait for main.
- **Re-running a release build**: Re-triggering `release.yml` for an existing tag (e.g., after a cancellation) must rebuild and re-attach assets idempotently (softprops/action-gh-release overwrites assets).
- **No consumers break**: Gating dev pre-releases must not break any existing workflow that depends on them (verified: none exist).
- **`fetch-depth` for tag checkout**: The build checkout with `ref: <tag>` needs the tag to be resolvable; the CD workflow already passes `fetch-tags: true` where relevant, and `workflow_dispatch` with a tag ref works natively.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: All build jobs in `.github/workflows/release.yml` (the `build` matrix, `build-deb`, `build-rpm`, and the `release` job's checkout) MUST check out the exact release tag passed via `inputs.tag`, using `actions/checkout@v4` with `ref: ${{ inputs.tag }}`.
- **FR-002**: The `pre-release` job in `.github/workflows/cd.yml` MUST NOT run on `push` events to `develop`; it MUST run only on `workflow_dispatch` (manual trigger) for non-main branches.
- **FR-003**: Manual triggering of the CD workflow on `develop` MUST still produce the dev pre-release (the on-demand path preserved).
- **FR-004**: The release asset build (`build-release` in `cd.yml`) MUST continue to run after `release_created == 'true'`, unaffected by the pre-release gating.
- **FR-005**: Re-running `release.yml` for an existing tag MUST rebuild and re-attach release assets without error (idempotent asset upload).
- **FR-006**: No existing consumer behavior changes: `rgt update` still resolves the latest stable release and its assets; Homebrew formula update still works for stable releases.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A release asset build for `v0.3.2` (or any tag) checks out the tag commit even when develop is ahead — verified by the checkout step's resolved SHA matching the tag.
- **SC-002**: A `push` to `develop` produces a CD run where the `pre-release` job is SKIPPED (100% of pushes, measured over a push sample).
- **SC-003**: A `workflow_dispatch` on `develop` produces a dev pre-release (tag + artifacts), confirming the on-demand path works.
- **SC-004**: The v0.3.2 release (and future releases) has all platform binaries attached and available to `rgt update` within one successful release build.
- **SC-005**: No CI regressions: feature PRs still pass CI; stable release builds complete without being cancelled by dev pre-release concurrency.
- **SC-006**: The changes are validated live: a re-triggered release build for an existing tag completes with assets attached.

## Assumptions

- The `pre-release` job's `if` condition (currently `github.ref == 'refs/heads/develop' || (workflow_dispatch && != main)`) is the only change needed for gating — the job body and its `tag` output logic remain as-is.
- `actions/checkout@v4` supports `ref: ${{ inputs.tag }}` in reusable workflows (it does — checkout accepts any valid git ref, including tags).
- The `release.yml` `homebrew` job's checkout (line 220) updates the formula on the current branch; it does not need a tag ref since it only computes SHAs from the released assets. If it does need the tag for URL construction, it already uses `inputs.tag`.
- Dev pre-releases have no current consumers (verified via `rgt update` source: `/releases/latest` skips prereleases; no other workflow references `dev-` tags).
- The v0.3.2 release assets will be built successfully from the tag as part of validating this feature (the earlier attempt was cancelled by concurrency).
