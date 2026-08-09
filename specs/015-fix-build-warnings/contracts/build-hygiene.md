# Contract: Warning-Free Build

**Feature**: 015-fix-build-warnings
**Contract type**: Build-hygiene behavior contract

## Synopsis

The Rust crate MUST compile and lint with zero warnings across all targets after this feature is implemented.

## Build Contract

| Command | Expected Result |
|---|---|
| `cargo build` | Exit 0, zero warnings |
| `cargo build --all-targets` | Exit 0, zero warnings |
| `cargo test --no-run` | Compiles all test targets, zero warnings |
| `cargo clippy --all-targets` | Exit 0, zero warnings, zero errors |
| `cargo test` | 108 tests pass, 0 failures |
| `cargo fmt --all -- --check` | Exit 0 (no diffs) |

## Verification Procedure

```bash
# Must each produce ZERO lines of output:
cargo build --all-targets 2>&1 | grep warning
cargo clippy --all-targets 2>&1 | grep warning
cargo test --no-run 2>&1 | grep warning
```

## Anti-Contract (prohibited)

- No crate-level `#![allow(...)]` attributes
- No `#[allow(dead_code)]` / `#[allow(unused_imports)]` without a justification comment (and none are expected for this feature since all warnings have clean fixes)
- No deletion or modification of tests to hide warnings
- No public signature or behavior changes

## Acceptance Mapping

| Acceptance Scenario | Contract Clause |
|---|---|
| US1/AC1 (bin 0 warnings) | Build contract |
| US1/AC2 (all-targets 0 warnings) | Build contract |
| US1/AC3 (test --no-run clean) | Build contract |
| US1/AC4 (clippy 0 warnings) | Build contract |
| US1/AC5 (108 tests pass) | Build contract |
| US1/AC6 (CI green) | CI jobs: fmt, clippy, test |
| US2/AC1-AC4 (suggestions addressed) | Anti-contract + research.md dispositions |
