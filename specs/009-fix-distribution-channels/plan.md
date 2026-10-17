# Implementation Plan: Fix Distribution Channels

**Branch**: `009-fix-distribution-channels` | **Date**: 2026-08-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/009-fix-distribution-channels/spec.md`

## Summary

Fix the CD pipeline so release assets are built and uploaded, replace the Homebrew tap dispatch with in-repo formula updates, fix CI clippy errors, update all documentation to use correct install commands (`brew install rafael-bianchi/rgt/rgt`, `cargo install --git`), and clean up dead code in the updater module.

## Technical Context

**Language/Version**: Rust Edition 2021, Bash (install.sh), YAML (GitHub Actions workflows)

**Primary Dependencies**: `chrono 0.4`, `clap 4.5`, `serde`, `blake3`, `rusqlite`, `ureq`, `sha2`, `flate2`, `tar`, `zip` (updater module)

**Storage**: SQLite via `rusqlite` (bundled, WAL mode) — `.rgt/store.db`

**Testing**: `cargo test --all -- --test-threads=1`

**Target Platform**: GitHub Actions runners (ubuntu-latest, macos-13, macos-latest, windows-latest) + cross-compilation for Linux aarch64

**Project Type**: Single Rust binary CLI tool with GitHub Actions CI/CD, shell install script, and Homebrew formula.

**Performance Goals**: CD build-release must complete within 30 minutes. CI must complete within 10 minutes. All tests pass.

**Constraints**: No new dependencies. No schema changes. The `rgt` crate name conflict on crates.io cannot be resolved — use `cargo install --git` as the documented install path.

**Scale/Scope**: 3 source files changed (value.rs, updater/github.rs, Formula/rgt.rb), 2 workflow files (cd.yml, release.yml), 3 documentation files (README.md, AGENTS.md, install.sh).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | No new dependencies |
| II | Two-Tier Detection & BLAKE3 | ✅ PASS | No impact on detection |
| III | Graph Correctness | ✅ PASS | No graph changes |
| IV | Rich Value Types | ✅ PASS | No type changes |
| V | Multi-Channel Distribution | ✅ PASS | Fixes the distribution channels — this is the core purpose |
| VI | Dual Integration Surface | ✅ PASS | No changes to hooks or MCP |
| VII | Permissive OSS | ✅ PASS | No licensing changes |

**Gate result**: All principles pass. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/009-fix-distribution-channels/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── distribution-contract.md
└── tasks.md             # Phase 2 output (speckit.tasks)
```

### Source Code (repository root)

```text
# Primary changes
tests/unit/test_date_diff.rs          # FIX: clippy erasing_op + identity_op
src/updater/github.rs                # FIX: dead code — wire update or remove
Formula/rgt.rb                        # FIX: SHA256 placeholders (auto-updated by CD)

# Workflow changes
.github/workflows/release.yml         # FIX: homebrew job — replace dispatch with in-repo commit
.github/workflows/cd.yml              # FIX: concurrency on main (investigate stuck runs)

# Documentation changes
README.md                             # UPDATE: Homebrew + Cargo install commands
AGENTS.md                             # UPDATE: Homebrew + Cargo install commands
install.sh                            # UPDATE: error message Cargo install command
```

**Structure Decision**: Single project. No new modules.

## Complexity Tracking

No violations to justify.
