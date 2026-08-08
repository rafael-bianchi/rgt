# Implementation Plan: Documentation Reconciliation

**Branch**: `012-documentation-reconciliation` | **Date**: 2026-08-08 | **Spec**: [spec.md](./spec.md)

## Summary

Reconcile all project documentation (README, AGENTS.md, CONTRIBUTING.md, code doc comments) against the actual codebase. The code and tests are the single source of truth. Primary work: add `///` doc comments to every public function missing them, verify README/AGENTS match the 8 CLI subcommands, gitignore local tooling dirs, and create/verify CONTRIBUTING.md.

## Technical Context

**Language/Version**: Rust Edition 2021. Documentation written in Markdown.

**Primary Dependencies**: None new. `cargo doc` for verification.

**Storage**: No storage changes. Documentation only.

**Testing**: `cargo test --all -- --test-threads=1` (regression check). `cargo doc --no-deps` (verify doc comments build without warnings).

**Target Platform**: All — documentation is platform-independent.

**Scale/Scope**: Documentation-only changes:
- 42 public functions across `src/` missing `///` doc comments
- `README.md` verified against 8 CLI subcommands (already current)
- `AGENTS.md` verified current install paths (already fixed)
- `CONTRIBUTING.md` created (69 lines)
- `.gitignore` add `.kilocode/`

## Constitution Check

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | No code changes |
| II | Two-Tier Detection | ✅ PASS | No impact |
| III | Graph Correctness | ✅ PASS | No impact |
| IV | Rich Value Types | ✅ PASS | No impact |
| V | Multi-Channel Distribution | ✅ PASS | No impact |
| VI | Dual Integration Surface | ✅ PASS | Doc comments cover MCP handlers |
| VII | Permissive OSS | ✅ PASS | No dependency changes |
| VIII | CLI-First Hooks | ✅ PASS | No impact |
| IX | Test-Driven Verification | ✅ PASS | Tests unchanged and passing |
| X | Documentation Matches Behavior | ✅ PASS | **This is the core purpose** |
| XI | Code Quality & Idiomatic Rust | ✅ PASS | Doc comments improve API discoverability |

**Gate result**: All 11 principles pass.

## Project Structure

### Documentation

```text
specs/012-documentation-reconciliation/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── doc-contract.md
└── tasks.md             # Phase 2 output
```

### Files Touched

```text
README.md                 # Verified current; 8 CLI subcommands documented
AGENTS.md                 # Verified current install paths
CONTRIBUTING.md           # Created stub (structure, tests, workflow)
.gitignore                # Added .kilocode/ under Development tooling
src/**/*.rs               # Added /// doc comments to 42 public functions
```

## Complexity Tracking

No violations.
