# Implementation Plan: RTK-Aligned Distribution and Installation Channels

**Branch**: `002-distribution-installation` | **Date**: 2026-07-31 | **Spec**: [spec.md](file:///Users/rafaelbianchi/dev/repos/rgt/specs/002-distribution-installation/spec.md)

**Input**: Feature specification from `/specs/002-distribution-installation/spec.md`

## Summary

Implement multi-channel distribution and installation infrastructure for RGT (Rust Graph Tracker) matching RTK's installation pathways:
1. A standalone POSIX shell installer script (`install.sh`) supporting `curl -fsSL https://rgt.dev/install.sh | sh` with OS/arch auto-detection (`darwin`/`linux`, `x86_64`/`aarch64`), SHA-256 checksum verification, and `PATH` validation.
2. An automated GitHub Actions CI/CD release workflow (`.github/workflows/release.yml`) cross-compiling stripped, single-binary assets across 5 target platform triples upon tag push (`v*`).
3. A Homebrew tap formula definition (`Formula/rgt.rb`) dispatches updates on GitHub release tags.
4. Crates.io packaging and `cargo install rgt` support.
5. A built-in binary self-update command (`rgt update` & `rgt update --check`) checking GitHub Releases API, verifying SHA-256 checksums, and atomically replacing the active executable.

## Technical Context

**Language/Version**: Rust 1.75+ (Edition 2021), POSIX Shell (`sh`/`bash`), Ruby (Homebrew formula), YAML (GitHub Actions)

**Primary Dependencies**:
- Rust binary: `clap` (CLI subcommands), `serde` / `serde_json`, `self_update` / `reqwest` or `ureq` (GitHub Release fetching), `flate2` / `tar` / `zip` (archiving)
- Shell installer: POSIX `sh`, `curl`/`wget`, `tar`, `shasum`/`sha256sum`

**Bundled Dependency**: `rusqlite` with `features = ["bundled"]` (SQLite statically linked, zero-system-dependency)

**Storage**: In-place binary replacement (`~/.local/bin/rgt` or `/usr/local/bin/rgt`)

**Testing**: Shell script integration tests in `tests/integration/test_installer_script.rs`, `cargo test`, GitHub Actions workflow dry-run

**Target Platform**:
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-unknown-linux-gnu` (Linux x86_64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-pc-windows-msvc` (Windows x86_64)

**Project Type**: Distribution infrastructure, CI/CD pipeline, Shell installer script, CLI self-update command

**Performance Goals**: <10s full shell installation; <5s binary self-update; <15min GitHub Actions matrix cross-compilation

**Constraints**: Zero external runtime dynamic dependencies (single binary release assets); atomic in-place executable replacement; non-blocking fail-safe error handling

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle I (Rust-Only Single Binary)**: PASS - Distribution artifacts are stripped, zero-runtime dependency single binaries cross-compiled for all 5 target platform triples.
- **Principle II (Speed-First Two-Tier Detection & BLAKE3)**: PASS - Preserved in compiled binaries.
  - *Note*: Release asset integrity uses SHA-256 (`checksums.txt`) for compatibility with standard tooling (`shasum`, `sha256sum`) and GitHub Release conventions. BLAKE3 remains the hashing algorithm for file-change detection (Tier 2) per Principle II.
- **Principle III (Graph Correctness & petgraph Reverse Traversal)**: PASS - Preserved in compiled binaries.
- **Principle IV (Value Types & Chrono Derivations)**: PASS - Preserved in compiled binaries.
- **Principle V (Multi-Channel UX & RTK Alignment)**: PASS - Directly implements all required distribution channels: `curl | sh` script, Homebrew tap formula, `cargo install`, and `rgt init -g` integration.
- **Principle VI (Dual Passive/Active Integration Surface)**: PASS - Preserved in compiled binaries.
- **Principle VII (Permissive Open-Source Licensing)**: PASS - MIT OR Apache-2.0.

## Project Structure

### Documentation (this feature)

```text
specs/002-distribution-installation/
├── plan.md              # Implementation plan
├── research.md          # Distribution technical decisions
├── data-model.md        # Release asset & manifest data structures
├── quickstart.md        # Installer validation guide
├── contracts/           # Interface specifications
│   ├── installer-schema.md  # install.sh script logic & flags
│   ├── homebrew-schema.md   # Formula specification (rgt.rb)
│   └── update-schema.md     # rgt update subcommand API
└── tasks.md             # Task execution list
```

### Source Code & Distribution Artifacts (repository root)

```text
install.sh                # POSIX shell one-liner quick installer script
Formula/rgt.rb            # Homebrew formula template/definition
.github/
└── workflows/
    ├── ci.yml            # Continuous integration build & test matrix
    └── release.yml       # Cross-compilation & GitHub Release automation

src/
├── cli/
│   ├── mod.rs
│   └── update.rs            # rgt update subcommand logic
└── updater/
    ├── mod.rs
    ├── platform.rs          # OS/arch detection + target triple mapping
    ├── checksum.rs          # SHA-256 manifest parser + verification
    ├── github.rs            # GitHub Releases API client
    └── atomic.rs            # Atomic binary in-place replacement

tests/
├── integration/
│   ├── test_installer_script.rs
│   └── test_distribution_quickstart.rs
└── contract/
    └── test_update_contract.rs
```

**Structure Decision**: Place `install.sh` and `Formula/rgt.rb` at repository root for easy GitHub raw download URL access (`raw.githubusercontent.com/...`), with `release.yml` in `.github/workflows/` and `src/cli/update.rs` + `src/updater/` inside the Rust crate.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

*No violations. All constitutional constraints strictly satisfied.*
