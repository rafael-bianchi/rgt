# Implementation Plan: Derivation Verification

**Branch**: `011-derivation-verification` | **Date**: 2026-08-08 | **Spec**: [spec.md](./spec.md)

## Summary

Add a `rgt verify` CLI subcommand that loads parent node values from SQLite, re-computes the expected result using `evalexpr` for formulas or `date_diff()` for date arithmetic, and exits 0 on match / 1 on mismatch with a structured error on stderr. Hook scripts call this CLI as thin delegates per Constitution VIII. Two operation types: `EXPRESSION` and `DATE_DIFF`.

## Technical Context

**Language/Version**: Rust Edition 2021, `chrono 0.4`, `rusqlite`, `clap 4`

**New Dependency**: `evalexpr` (MIT, crates.io) — expression parsing and evaluation with variable binding.

**Storage**: SQLite via `rusqlite` — parent values are loaded for verification, no schema changes.

**Testing**: `cargo test --all -- --test-threads=1`. Unit tests for each operation type, integration tests for CLI exit codes and stderr output.

**Target Platform**: All platforms (pure Rust, no platform-specific deps).

**Performance**: Verification loads N parent rows from SQLite (~2ms), evaluates via `evalexpr` (~0.1ms), compares (~0ms). Target: <10ms per call.

**Scale/Scope**: 1 new CLI subcommand (`src/cli/verify.rs`), 1 new module (`src/verify/`), 1 dependency, changes to `src/cli/mod.rs` and `src/lib.rs`.

## Constitution Check

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | Pure Rust, `evalexpr` is MIT |
| II | Two-Tier Detection & BLAKE3 | ✅ PASS | No impact |
| III | Graph Correctness | ✅ PASS | Enhances integrity — incorrect data rejected |
| IV | Rich Value Types | ✅ PASS | Verifies Number and Duration derivations |
| V | Multi-Channel Distribution | ✅ PASS | No changes |
| VI | Dual Integration Surface | ✅ PASS | Passive hooks call `rgt verify` as thin delegates |
| VII | Permissive OSS | ✅ PASS | `evalexpr` is MIT-licensed |
| VIII | CLI-First Hook Architecture | ✅ PASS | Verification is a CLI subcommand; hooks are thin delegates |

**Gate result**: All 8 principles pass.

## Project Structure

### Documentation

```text
specs/011-derivation-verification/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── cli-verify.md
└── tasks.md             # To be generated
```

### Source Code

```text
src/
├── verify/              # NEW: shared verification logic
│   ├── mod.rs           # dispatch by operation_type
│   ├── expression.rs    # EXPRESSION: eval with evalexpr
│   └── date_diff.rs     # DATE_DIFF: compute date2 - date1
├── cli/
│   ├── mod.rs           # UPDATE: register verify subcommand
│   └── verify.rs        # NEW: CLI args + DB access + delegate to verify/
├── types/
│   └── value.rs         # UPDATE: remove #[allow(dead_code)] from date_diff()
└── lib.rs               # UPDATE: register verify module

tests/
├── unit/
│   └── test_verification.rs  # NEW: unit tests for each operation
├── contract/
│   └── test_verify_cli.rs    # NEW: CLI exit code + stderr contract tests
└── integration/
    └── test_derivation_chain.rs # UPDATE: verify chain with verification
```

## Complexity Tracking

No violations.
