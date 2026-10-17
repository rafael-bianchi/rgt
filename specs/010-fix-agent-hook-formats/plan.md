# Implementation Plan: Fix Agent Hook Formats (RTK-Informed)

**Branch**: `010-fix-agent-hook-formats` | **Date**: 2026-08-02 | **Spec**: [spec.md](./spec.md)

## Summary

Fix broken hook configurations for all 4 supported AI coding agents. Claude Code and Cursor get corrected JSON schemas matching their official docs (validated against RTK's working production code). Windsurf and Codex CLI are converted from non-functional hook JSON to their correct integration tiers — rules-file (`.windsurfrules`) and instructions-based (`AGENTS.md`), following RTK's confirmed approach.

## Technical Context

**Language/Version**: Rust Edition 2021, `serde_json` for JSON manipulation

**Primary Dependencies**: `serde_json` (already in tree), `serde` (derive), `dirs` (home directory resolution)

**Storage**: No storage impact — configuration files are written to user home directories (`.claude/`, `.cursor/`, `.windsurf/`, `.codex/`)

**Testing**: `cargo test test_hooks_installer -- --test-threads=1`. Integration tests in `tests/integration/test_hooks_installer.rs`.

**Target Platform**: macOS, Linux (hook files are JSON text — no platform-specific logic)

**Project Type**: Rust CLI binary hook installer

**Constraints**: Must merge into existing JSON files for Claude Code (`settings.json` may have other keys). Must follow RTK's exit code contract (exit 0 on all errors) but that applies to hook scripts, not the installer — out of scope here.

**Scale/Scope**: 2 source files changed (`src/hooks/installer.rs`, `src/cli/init.rs`), 1 test file updated. No new dependencies. No database changes.

## Constitution Check

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | Rust-only change, no new dependencies |
| II | Two-Tier Detection & BLAKE3 | ✅ PASS | No impact |
| III | Graph Correctness | ✅ PASS | No graph changes |
| IV | Rich Value Types | ✅ PASS | No type changes |
| V | Multi-Channel Distribution | ✅ PASS | No distribution changes |
| VI | Dual Integration Surface | ✅ PASS | Fixes the passive hooks integration surface |
| VII | Permissive OSS | ✅ PASS | No licensing changes |

**Gate result**: All principles pass.

## Project Structure

### Documentation

```text
specs/010-fix-agent-hook-formats/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── hook-formats.md
└── tasks.md             # Phase 2 output
```

### Source Code

```text
src/hooks/
├── installer.rs         # PRIMARY: fix all 4 agent hook configs
├── parser.rs            # No change
└── mod.rs               # No change

src/cli/
├── init.rs              # SECONDARY: update --agent help text

tests/integration/
├── test_hooks_installer.rs  # UPDATE: assertions on corrected formats
```

## Complexity Tracking

No violations.
