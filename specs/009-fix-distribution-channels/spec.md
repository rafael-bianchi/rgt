# Feature Specification: Fix Distribution Channels

**Feature Branch**: `fix/009-distribution-channels`

**Created**: 2026-08-01

**Status**: Draft

**Input**: User description: "Investigate and fix distribution system — release assets, CD pipeline, CI clippy, and update all install instructions to reflect simplified Homebrew (main repo) and Cargo (git) channels"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - User Installs via Shell Script (Priority: P1)

A developer runs `curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh`. The script detects their platform, downloads the appropriate binary archive from the latest GitHub Release, verifies the SHA256 checksum, extracts the binary, and installs it. The script's error messages reference the correct Cargo install command (`cargo install --git`, not `cargo install rgt`).

**Why this priority**: The shell installer is the primary distribution channel. Without functioning release assets, the entire distribution pipeline is broken. Additionally, the script's error messages currently suggest `cargo install rgt` which installs an unrelated crate.

**Independent Test**: Run `install.sh` in a clean environment. It must download the correct binary, verify checksums, and install `rgt`. Error paths must show `cargo install --git https://github.com/rafael-bianchi/rgt`.

**Acceptance Scenarios**:

1. **Given** a GitHub Release with macOS arm64 binary archive and valid checksums.txt, **When** a user on Apple Silicon runs `install.sh`, **Then** `rgt` is installed to `~/.local/bin/rgt`.
2. **Given** a GitHub Release with Linux x86_64 musl binary archive, **When** a user on Linux x86_64 runs `install.sh`, **Then** the binary installs successfully.
3. **Given** an unsupported platform, **When** `install.sh` prints the fallback error, **Then** it suggests `cargo install --git https://github.com/rafael-bianchi/rgt`, not `cargo install rgt`.

---

### User Story 2 - User Installs via Homebrew (Priority: P1)

A macOS or Linux user runs `brew install rafael-bianchi/rgt/rgt`. Homebrew reads `Formula/rgt.rb` from the main repository, fetches the binary archive from the GitHub Release, verifies the SHA256 hash, and installs the binary. The formula has accurate SHA256 values.

**Why this priority**: Homebrew is a distribution channel per Constitution V. The formula lives in the main repo (no separate tap), has placeholder SHA256s, and the install instructions in all docs reference the wrong command (`rafael-bianchi/tap/rgt`).

**Independent Test**: Run `brew install rafael-bianchi/rgt/rgt` on macOS. The formula must reference real release assets with valid SHA256 hashes.

**Acceptance Scenarios**:

1. **Given** a GitHub Release with macOS arm64 archive and correct SHA256 in `Formula/rgt.rb`, **When** a user runs `brew install rafael-bianchi/rgt/rgt` on Apple Silicon, **Then** `rgt` is installed and `rgt --version` outputs the correct version.

---

### User Story 3 - User Installs via Cargo from Git (Priority: P2)

A Rust developer runs `cargo install --git https://github.com/rafael-bianchi/rgt`. This clones the repo, compiles RGT from source, and installs the binary. The installed binary functions identically to the pre-built distribution.

**Why this priority**: Cargo install from git is the third distribution channel. The `rgt` crate name on crates.io is taken by an unrelated project, so `cargo install rgt` installs the wrong software. Using `cargo install --git` bypasses crates.io entirely (same approach used by rtk-ai/rtk).

**Independent Test**: Run `cargo install --git https://github.com/rafael-bianchi/rgt` and verify `rgt --version` outputs the correct version.

**Acceptance Scenarios**:

1. **Given** the RGT repo is public on GitHub, **When** a user runs `cargo install --git https://github.com/rafael-bianchi/rgt`, **Then** the RGT binary is compiled and installed, not the unrelated `rgt` syntax tree library.

---

### User Story 4 - CD Workflow Produces Release Artifacts (Priority: P1)

When a release is created by release-please, the CD workflow builds cross-platform binaries for all target platforms, generates SHA256 checksums, uploads all assets to the GitHub Release, and commits updated SHA256 values to `Formula/rgt.rb` in the main repo. The workflow completes without manual intervention.

**Why this priority**: The CD workflow produces all distribution artifacts. The current `main` branch CD run is stuck pending, v0.1.0 has zero assets, and the `homebrew` job dispatches to a non-existent separate tap repo that no longer fits the architecture.

**Independent Test**: Merge a release-please PR to main, observe the CD workflow completes with all artifacts uploaded to the GitHub Release and `Formula/rgt.rb` updated.

**Acceptance Scenarios**:

1. **Given** a new release is created on `main`, **When** the CD workflow runs, **Then** `build-release` uploads binary archives for all 5 target platforms to the GitHub Release.
2. **Given** the `build-release` job completes, **When** the `homebrew` job runs, **Then** `Formula/rgt.rb` in the main repo is updated with the new release's SHA256 values (committed back to the branch).
3. **Given** the CD workflow on `main`, **When** a new push arrives while a previous run is in progress, **Then** the previous run completes first and the new run follows.

---

### User Story 5 - CI Passes on All Branches (Priority: P1)

All CI checks pass on every PR and push. No clippy errors (`erasing_op`, `identity_op`), no dead-code warnings that block the pipeline. The updater module's dead code is resolved.

**Why this priority**: CI is the gating mechanism for all PRs. The current `feat/008-human-readable-durations` branch fails clippy due to `erasing_op` and `identity_op` errors in `tests/unit/test_date_diff.rs:35`.

**Independent Test**: Push a commit, observe CI runs all jobs and each passes.

**Acceptance Scenarios**:

1. **Given** a commit to any branch, **When** CI runs `cargo clippy --all-targets`, **Then** no clippy errors or warnings are emitted.
2. **Given** a commit to any branch, **When** CI runs `cargo test --all`, **Then** all tests pass on all three platforms.

---

### User Story 6 - Documentation Uses Correct Install Commands (Priority: P2)

All project documentation (README, AGENTS.md, install.sh, spec artifacts) references the correct install commands: Homebrew from main repo (`rafael-bianchi/rgt/rgt`), Cargo from git (`cargo install --git`), and shell script. No references to `rafael-bianchi/tap/rgt`, bare `cargo install rgt`, or `rafael-bianchi/homebrew-tap` remain except in historical spec files.

**Why this priority**: Documentation with wrong install commands misleads users into installing the wrong software or failing to install at all.

**Independent Test**: Grep the repo for `rafael-bianchi/tap`, `cargo install rgt` (without `--git`), and `homebrew-tap` — only historical spec files and this spec may remain.

**Acceptance Scenarios**:

1. **Given** a user reads the README, **When** they follow the Homebrew instructions, **Then** they see `brew install rafael-bianchi/rgt/rgt`.
2. **Given** a user reads the README, **When** they follow the Cargo instructions, **Then** they see `cargo install --git https://github.com/rafael-bianchi/rgt`.

---

### Edge Cases

- What happens when a release artifact build fails for one platform but succeeds for others? The workflow should continue uploading successful artifacts and clearly report which platform failed.
- What happens when two release-please PRs are created in quick succession? Only the latest should produce artifacts.
- What happens if the formula SHA256 update commit conflicts with a concurrent change to `Formula/rgt.rb`? The CD workflow should retry or report the conflict for manual resolution.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CD workflow MUST produce and upload binary archives for all 5 target platforms (macOS x86_64, macOS aarch64, Linux musl x86_64, Linux gnu aarch64, Windows x86_64) to GitHub Releases when a release is created.
- **FR-002**: The CD workflow MUST generate and upload SHA256 checksums (`checksums.txt`) for all release artifacts as part of the release process.
- **FR-003**: The CD workflow MUST build and upload .deb and .rpm packages as part of the release process.
- **FR-004**: The Homebrew formula (`Formula/rgt.rb` in the main repo) MUST contain accurate SHA256 digests for all binary archives. The CD workflow MUST update these values in-place when a release is published.
- **FR-005**: The `homebrew` job in `release.yml` MUST update `Formula/rgt.rb` via a commit to the main repo (not dispatch to a separate tap repository).
- **FR-006**: The shell install script (`install.sh`) MUST reference `cargo install --git https://github.com/rafael-bianchi/rgt` in error messages (not bare `cargo install rgt`).
- **FR-007**: All CI checks (fmt, clippy, test) MUST pass without errors on every branch. No `clippy::erasing_op` or `clippy::identity_op` lints MUST be present.
- **FR-008**: Dead code in `src/updater/github.rs` MUST be resolved — either wired into the CLI `update` subcommand or removed.
- **FR-009**: All project documentation MUST use the correct install commands: `brew install rafael-bianchi/rgt/rgt` (Homebrew from main repo) and `cargo install --git https://github.com/rafael-bianchi/rgt` (Cargo from git).

### Files Requiring Updates

The following files contain stale distribution references and MUST be updated as part of this feature. Historical spec files (specs/002/, .kilo/plans/) are noted but optional to update.

#### P1 — Active Documentation (MUST update)

| File | Stale Content | Correct Content |
|---|---|---|
| `README.md:32` | `brew install rafael-bianchi/tap/rgt` | `brew install rafael-bianchi/rgt/rgt` |
| `README.md:37` | `cargo install rgt` | `cargo install --git https://github.com/rafael-bianchi/rgt` |
| `AGENTS.md:238` | `brew install rafael-bianchi/tap/rgt` | `brew install rafael-bianchi/rgt/rgt` |
| `AGENTS.md:239` | `cargo install rgt` | `cargo install --git https://github.com/rafael-bianchi/rgt` |
| `install.sh:139` | `cargo install rgt` | `cargo install --git https://github.com/rafael-bianchi/rgt` |
| `install.sh:152` | `cargo install rgt` | `cargo install --git https://github.com/rafael-bianchi/rgt` |

#### P2 — Workflow & Spec Artifacts (SHOULD update)

| File | Stale Content | Correct Content |
|---|---|---|
| `.github/workflows/release.yml:229` | `repository: rafael-bianchi/homebrew-tap` (dispatch) | Replace with in-repo formula update (commit `Formula/rgt.rb` back) |
| `specs/009-fix-distribution-channels/spec.md` | Multiple `rafael-bianchi/tap/rgt` references | `rafael-bianchi/rgt/rgt` (this file, already done above) |

#### P3 — Historical Spec Files (OPTIONAL to update)

| File | Lines |
|---|---|
| `specs/002-distribution-installation/spec.md` | 46, 55, 61, 65, 69, 109, 110, 131 |
| `specs/002-distribution-installation/plan.md` | 13 |
| `specs/002-distribution-installation/quickstart.md` | 56 |
| `specs/002-distribution-installation/research.md` | 58 |
| `specs/002-distribution-installation/tasks.md` | 88, 90, 101 |
| `specs/002-distribution-installation/contracts/homebrew-schema.md` | 9, 21 |
| `.kilo/plans/agents-md.md` | 112, 113 |

### Key Entities

- **GitHub Release**: A published version tag with attached binary archives, checksums, and package files. Created by release-please, populated by CD `build-release`.
- **Release Artifact**: Platform-specific binary archive (`.tar.gz` for Unix, `.zip` for Windows) containing the `rgt` binary.
- **Checksum File**: `checksums.txt` containing SHA256 digests for all release artifacts. Generated by CD, used by `install.sh` for verification.
- **Homebrew Formula**: `Formula/rgt.rb` in the main repo. References release artifacts with versioned URLs and SHA256 hashes. Served via `brew install rafael-bianchi/rgt/rgt`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh` completes successfully on macOS and Linux within 60 seconds.
- **SC-002**: `brew install rafael-bianchi/rgt/rgt` installs the correct binary with verified SHA256 checksums on macOS.
- **SC-003**: `cargo install --git https://github.com/rafael-bianchi/rgt` compiles and installs the RGT binary, and `rgt --version` outputs the correct version.
- **SC-004**: GitHub Release for v0.1.0 (and subsequent releases) contains assets for all 5 target platforms, plus checksums.txt, .deb, and .rpm.
- **SC-005**: CI pipeline passes (fmt, clippy, test) on all branches with zero clippy errors.
- **SC-006**: Grep for `rafael-bianchi/tap`, bare `cargo install rgt` (without `--git`), and `homebrew-tap` in active documentation files returns zero matches.

## Assumptions

- The stuck CD workflow run on `main` can be cancelled manually and re-triggered.
- `Formula/rgt.rb` already uses main repo URLs (`#{homepage}` resolves to `rafael-bianchi/rgt`) — only SHA256s and the CD update mechanism need changes.
- The `rgt` crate name on crates.io is owned by a different author. Using `cargo install --git` follows the same pattern as rtk-ai/rtk, which also has a crates.io name collision.
- Historical spec files (specs/002/, .kilo/plans/) are snapshots of past work and do not block current distribution functionality — updating them is optional.
