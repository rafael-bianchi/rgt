# Data Model: CI Pipeline Hygiene

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data. Transient CI entities:

## 1. TestPresenceDiff

The set of changed Rust source files in a PR plus the test-coverage mapping. A source file is "covered" when some file under `tests/` references it.

| Input | Outcome |
|-------|---------|
| new/modified `src/cli/foo.rs`, no `tests/` reference | FAIL (name file + suggest test path) |
| modified `src/verify/mod.rs` + updated `tests/` | PASS |
| docs/specs/workflows only | PASS |

## 2. CoverageReport

`cargo llvm-cov --all` output: line-coverage percentage (job summary) + HTML/`lcov` artifact.

## 3. ActionPin

`uses: owner/repo@<40-hex-sha> # <tag>` — an immutable reference replacing a mutable tag in elevated-permission workflows.
