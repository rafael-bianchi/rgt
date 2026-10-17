# Feature Specification: RTK-Aligned Distribution and Installation Channels

**Feature Branch**: `002-distribution-installation`

**Created**: 2026-07-31

**Status**: Draft

**Input**: User description: "Let's check how close we can get to rtk (https://github.com/rtk-ai/rtk) deployment and distribution and installing"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Shell One-Liner Quick Installer (`curl | sh`) (Priority: P1) 🎯 MVP

As a developer on macOS or Linux, I want to install `rgt` using a single shell command (`curl -fsSL ... | sh`), so that the installer script auto-detects my operating system and CPU architecture, downloads the correct pre-compiled release binary, verifies its checksum, places it into `~/.local/bin` (or `/usr/local/bin`), and ensures it is accessible in my `PATH`.

**Why this priority**: Mirroring RTK's one-liner installation script is the fastest, lowest-friction entry point for developers and CI/CD pipelines.

**Independent Test**: Execute the install script in clean macOS (x86_64, arm64) and Linux (x86_64, aarch64) container environments, verifying that `rgt --version` executes cleanly afterwards.

**Acceptance Scenarios**:

1. **Given** a developer running `curl -fsSL <install-script-url> | sh` on macOS (M1/M2/M3 or Intel) or Linux (Ubuntu/Debian/Arch/Alpine), **When** the script executes, **Then** it correctly detects OS (`darwin`/`linux`) and architecture (`x86_64`/`aarch64`), downloads the latest release archive, verifies SHA-256 checksum, extracts the `rgt` single binary, and places it in `~/.local/bin` (or `/usr/local/bin`).
2. **Given** an environment where `~/.local/bin` is not in the user's `PATH`, **When** installation completes, **Then** the script outputs explicit shell configuration instructions (for `.zshrc`, `.bashrc`, `.fish`) to update `PATH`.
3. **Given** an unsupported OS or architecture platform, **When** the install script runs, **Then** it exits cleanly with a helpful error message and instructions for building from source via `cargo install`.

---

### User Story 2 - Automated GitHub Actions Cross-Compilation Matrix (Priority: P1)

As a project maintainer, I want a GitHub Actions release workflow that automatically builds stripped, single-binary release assets across five primary target triples (`x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) upon creating a release tag (`v*`), attaching release archives (`tar.gz` and `.zip`) and `checksums.txt` to the GitHub Release.

**Why this priority**: Provides the binary artifacts required by the shell installer, Homebrew tap, and direct GitHub releases.

**Independent Test**: Trigger a release tag push (`v0.1.0`), verify all five platform binaries are compiled, stripped, packaged into compressed archives, signed with SHA-256 checksums, and attached to the GitHub Release.

**Acceptance Scenarios**:

1. **Given** a new git tag matching `v*` pushed to GitHub, **When** the release workflow completes, **Then** compiled single-binary release assets for all 5 target triples are published under the GitHub release with SHA-256 checksum manifests (`checksums.txt`).
2. **Given** compiled binaries, **When** packaging completes, **Then** binaries are stripped of debug symbols to maintain minimal binary file size (<15MB).

---

### User Story 3 - Homebrew Tap Formula (Priority: P1)

As a macOS or Linux developer using Homebrew, I want to install and update `rgt` using `brew install rafael-bianchi/tap/rgt` (or official formula), with an automated workflow updating the Homebrew tap formula version and SHA-256 hashes upon every new release tag.

**Why this priority**: Homebrew is the primary package manager for macOS developers, matching RTK's official distribution channels.

**Independent Test**: Install `rgt` via `brew install` from a tap repository, verify installation completes, and execute `rgt status`.

**Acceptance Scenarios**:

1. **Given** a published GitHub release `vX.Y.Z`, **When** a Homebrew tap formula workflow runs, **Then** the Homebrew formula file `rgt.rb` is automatically updated with the new version string, download URLs, and SHA-256 checksums.
2. **Given** a developer with Homebrew installed, **When** running `brew install <tap>/rgt`, **Then** Homebrew downloads the platform archive, installs the `rgt` binary, and installs shell completions (if present).

---

### User Story 4 - Cargo Crates.io & `cargo install` Distribution (Priority: P1)

As a Rust developer, I want to install `rgt` directly via `cargo install rgt` (from crates.io or git repository), so that I can compile the single-binary directly on any Rust-supported platform.

**Why this priority**: Fulfills Constitution Principle V by ensuring standard Rust tooling distribution pathways are fully functional.

**Independent Test**: Run `cargo install --path .` or `cargo install rgt` in a clean environment, verifying that Cargo compiles and installs `rgt` to `~/.cargo/bin/rgt`.

**Acceptance Scenarios**:

1. **Given** a developer with Rust installed, **When** `cargo install rgt` is executed, **Then** Cargo fetches the crate, builds the single binary executable with bundled SQLite, and installs it to `~/.cargo/bin`.

---

### User Story 5 - Built-In Binary Self-Update Command (`rgt update`) (Priority: P2)

As a developer, I want to run `rgt update` from the command line to check GitHub Releases for newer versions, download the matching binary asset, verify checksums, and safely perform an in-place update of the executable.

**Why this priority**: Enables developers to stay up to date without needing to manually re-run `curl` commands or package manager updates.

**Independent Test**: Run `rgt update` on an older binary version in a test environment, verifying that it identifies the latest GitHub release, downloads the asset, verifies the checksum, and updates the binary in-place.

**Acceptance Scenarios**:

1. **Given** an active `rgt` binary on a user's system, **When** the user runs `rgt update`, **Then** `rgt` checks GitHub Releases API for newer tags. If a newer version exists, it prompts for confirmation, downloads the binary, verifies the SHA-256 hash, and atomically replaces the running executable.
2. **Given** an `rgt` binary that is already at the latest release version, **When** `rgt update` runs, **Then** it prints a message confirming the system is up to date (`rgt is already at the latest version vX.Y.Z`).

---

### Edge Cases

- **Read-Only Binary Directory**: If `rgt update` or the shell installer lacks write permissions to the target binary directory (e.g. `/usr/local/bin`), the tool MUST request `sudo` elevation or fallback to `~/.local/bin`.
- **Network / GitHub API Throttling**: When checking for updates or running the installer without authentication, API rate limits MUST be handled gracefully with clear retry/fallback instructions.
- **Corrupted Asset Downloads**: If a downloaded binary fails SHA-256 checksum verification, the installer/updater MUST delete the corrupted file and abort without modifying the existing executable.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a standalone, POSIX-compliant shell installer script (`install.sh`) downloadable via `curl -fsSL` that auto-detects OS (`darwin`, `linux`) and architecture (`x86_64`, `aarch64`).
- **FR-002**: The shell installer MUST download matching release assets from GitHub Releases, verify SHA-256 checksums against `checksums.txt`, extract the `rgt` binary, and place it into `~/.local/bin` (or `/usr/local/bin`).
- **FR-003**: The shell installer MUST check if the binary directory is present in the current user's `PATH` and provide shell configuration instructions if missing.
- **FR-004**: System MUST maintain a GitHub Actions release pipeline (`.github/workflows/release.yml`) triggered on tag pushes (`v*`).
- **FR-005**: The release pipeline MUST cross-compile stripped, single-binary assets for:
  - `x86_64-apple-darwin` (macOS Intel)
  - `aarch64-apple-darwin` (macOS Apple Silicon)
  - `x86_64-unknown-linux-gnu` (Linux x86_64)
  - `aarch64-unknown-linux-gnu` (Linux ARM64)
  - `x86_64-pc-windows-msvc` (Windows x86_64)
- **FR-006**: The release pipeline MUST generate `.tar.gz` archives for Unix targets, `.zip` for Windows targets, and a unified `checksums.txt` file containing SHA-256 hashes for all assets.
- **FR-007**: System MUST provide a Homebrew formula definition (`Formula/rgt.rb` or dedicated tap repository) supporting `brew install`.
- **FR-008**: System MUST support `cargo install` publishing on crates.io with complete crate metadata and bundled SQLite compilation.
- **FR-009**: System MUST provide a CLI subcommand `rgt update` that checks GitHub Releases for new tag versions.
- **FR-010**: `rgt update` MUST verify SHA-256 checksums before performing atomic in-place executable replacement.
- **FR-011**: System MUST support a `--check` flag on `rgt update` (`rgt update --check`) to verify if updates are available without performing the download/install.
- **FR-012**: System MUST adhere to zero external runtime dependencies across all distribution channels.

### Key Entities

- **Release Artifact**: A compiled, stripped single-binary executable of `rgt` targeted at a specific OS and CPU architecture.
- **Target Triple**: The formal target specification string (e.g. `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`).
- **Installer Script**: A POSIX shell script (`install.sh`) responsible for environment detection, fetching release assets, verifying checksums, and PATH installation.
- **Homebrew Formula**: A Ruby specification (`rgt.rb`) describing how Homebrew downloads, verifies, and installs `rgt` binary releases.
- **Release Manifest (`checksums.txt`)**: A cryptographic text file containing SHA-256 hashes for all published binary release archives.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: One-liner shell installer (`curl -fsSL ... | sh`) completes full installation in under 10 seconds on standard broadband connections.
- **SC-002**: 100% of binary release archives attached to GitHub Releases pass SHA-256 checksum verification before extraction.
- **SC-003**: Cross-compilation GitHub Actions workflow successfully generates release assets for all 5 target platform triples in under 15 minutes per release tag.
- **SC-004**: `brew install` installs the binary and enables execution without security/notarization blocking on macOS.
- **SC-005**: `rgt update` completes in-place binary upgrade in under 5 seconds when an update is available.

## Assumptions

- Target environments have standard command-line tools available (`curl` or `wget`, `tar` or `unzip`, `sha256sum` or `shasum`).
- Public or private GitHub Releases API endpoints are accessible for fetching release tags and binary assets.
- Users have write permissions to `~/.local/bin` or administrative access for global installation.
