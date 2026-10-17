# Implementation Plan: CI Pipeline Hygiene

**Branch**: `feat/037-ci-pipeline-hygiene` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/037-ci-pipeline-hygiene/spec.md`

## Summary

Harden the CI pipeline with three hygiene fixes: (1) implement the no-op "Test Presence Check" job so it actually flags new/modified `src/*.rs` files with no test coverage; (2) add a code-coverage job (`cargo llvm-cov`) that reports the percentage in the job summary without requiring a new secret, plus a README badge/note; (3) pin all third-party GitHub Actions to commit SHA in `release.yml` and `cd.yml` (which hold elevated permissions), with the version tag preserved as a comment.

## Technical Context

**Language/Version**: YAML workflows + bash + Rust tooling (no application code changes).

**Primary Dependencies**: None new in the binary; CI uses `cargo-llvm-cov` (dev tool) and existing actions re-pinned to SHA.

**Storage**: N/A.

**Testing**: The test-presence check logic is run locally against synthetic diffs; coverage is measured via `cargo llvm-cov`; SHA pins verified against the GitHub API.

**Target Platform**: CI runners (ubuntu/windows/macos).

**Project Type**: CI/CD workflows.

**Performance Goals**: Coverage run should stay within the existing 20-minute test budget (coverage instrumentation adds some overhead — acceptable; run it on ubuntu only).

**Constraints**: No new secrets (coverage reports via job summary + artifact). No application code changes. SHA pins must be real commits of the tags they replace.

**Scale/Scope**: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/cd.yml`, README badge, optional `scripts/check-test-presence.sh`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | CI tooling only; no binary change. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Unaffected. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | Supply-chain hardening of the release path. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | The test-presence gate enforces it; coverage measures it. |
| X. Documentation matches behavior | PASS | README badge/note added. |
| XI. Idiomatic Rust, exit codes | PASS | No code change. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/037-ci-pipeline-hygiene/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── ci-hygiene-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
.github/workflows/ci.yml        # test-presence implemented + coverage job
.github/workflows/release.yml   # actions pinned to SHA
.github/workflows/cd.yml        # actions pinned to SHA
README.md                        # coverage badge/note
scripts/check-test-presence.sh   # NEW: the real check (testable locally)
```

**Structure Decision**: The test-presence check lives in a small bash script (`scripts/check-test-presence.sh`) so it's runnable and testable locally, not buried in YAML. CI jobs are thin callers. Coverage uses `cargo llvm-cov` on the ubuntu runner.

## Complexity Tracking

> None — no constitution violations requiring justification.
