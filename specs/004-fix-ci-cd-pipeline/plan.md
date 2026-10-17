# Implementation Plan: Fix CI/CD Pipeline Post-Deployment Issues

**Branch**: `004-fix-ci-cd-pipeline` | **Date**: 2026-07-31 | **Spec**: [spec.md](file:///Users/rafaelbianchi/dev/repos/rgt/specs/004-fix-ci-cd-pipeline/spec.md)

**Input**: Feature specification from `/specs/004-fix-ci-cd-pipeline/spec.md`

## Summary

Three targeted fixes to the CI/CD pipeline deployed in feature 003: add push trigger to CI workflow, replace hardcoded `origin/main` references in the security job with a dynamic base branch, and enable GitHub Actions pull request creation in repository settings for release-please.

## Technical Context

**Language/Version**: YAML (GitHub Actions workflow definitions), Bash (CI scripts). Repository settings change (no code).

**Primary Dependencies**: None — all changes are to existing workflow files and a repository setting.

**Storage**: N/A

**Testing**: Manual push to develop verifies CI triggers; PR to develop verifies security job uses correct base branch; push to main verifies release-please creates PR after setting change.

**Target Platform**: GitHub Actions runners

**Project Type**: CI/CD pipeline configuration fix

**Performance Goals**: CI must trigger within 30s of push event (SC-001). Release PR created within 2 minutes (SC-003).

**Constraints**: Must preserve all existing CI job behavior (dependency chain, timeouts, security scans) — only fix what's broken.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle I (Rust-Only Single Binary)**: PASS — No Rust code changes.
- **Principle II (Speed-First Two-Tier Detection & BLAKE3)**: PASS — No runtime behavior changes.
- **Principle III (Graph Correctness & petgraph Reverse Traversal)**: PASS — No graph engine changes.
- **Principle IV (Value Types & Chrono Derivations)**: PASS — No value type changes.
- **Principle V (Multi-Channel Distribution & Frictionless UX)**: PASS — Fixes CD pipeline so releases continue working automatically.
- **Principle VI (Dual Passive/Active Integration Surface)**: PASS — No integration changes.
- **Principle VII (Permissive Open-Source Governance)**: PASS — No license changes.

## Project Structure

### Documentation (this feature)

```text
specs/004-fix-ci-cd-pipeline/
├── plan.md              # Implementation plan
├── research.md          # Technical decisions
├── quickstart.md        # Validation guide
├── contracts/           # N/A — no new interfaces
└── tasks.md             # Task execution list
```

### Source Code (repository root)

```text
.github/
└── workflows/
    └── ci.yml            # Edit: add push trigger, fix base branch references
```

**Structure Decision**: Single file edit (`ci.yml`) plus a repository settings change. No new files needed. `cd.yml` and `pr-target-check.yml` are unchanged; the permission fix is in GitHub repository settings, not the workflow file.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

*No violations.*
