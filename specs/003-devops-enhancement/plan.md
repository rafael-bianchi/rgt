# Implementation Plan: CI/CD Pipeline Security & Delivery Hardening

**Branch**: `003-devops-enhancement` | **Date**: 2026-07-31 | **Spec**: [spec.md](file:///Users/rafaelbianchi/dev/repos/rgt/specs/003-devops-enhancement/spec.md)

**Input**: Feature specification from `/specs/003-devops-enhancement/spec.md`

## Summary

Harden the CI/CD pipeline with automated security scanning (dependency vulnerabilities, dangerous code patterns, supply-chain audit), conventional-commit-driven versioning with a single reusable build-and-release pipeline, optimized fast-failure CI gates, universal Linux support via MUSL static binary and native DEB/RPM packages, and automated branch protection enforcement.

## Technical Context

**Language/Version**: YAML (GitHub Actions workflow definitions), Bash (CI scripts). Existing Rust project — minimal Rust code changes (target triple string update in `src/updater/platform.rs` only).

**Primary Dependencies**:
- `cargo-audit` — Rust dependency vulnerability scanner
- `googleapis/release-please-action@v4` — conventional-commit-driven release automation
- `cargo-deb` — DEB package builder for Rust binaries
- `cargo-generate-rpm` — RPM package builder for Rust binaries
- `Swatinem/rust-cache@v2` — CI caching for faster builds
- `actions/create-github-app-token@v3` — elevated permissions for automated releases

**Storage**: N/A — CI/CD configuration files only

**Testing**: Manual workflow validation via PR simulation (test PRs triggering security scans, release tags triggering builds), `cargo deb` and `cargo generate-rpm` package validation in CI

**Target Platform**: GitHub Actions runners (ubuntu-latest, macos-latest, windows-latest) + Linux MUSL cross-compilation target

**Project Type**: CI/CD pipeline configuration, workflow automation

**Performance Goals**:
- Format check fails within 30s, skips test matrix (SC-002)
- Security scan completes within 2 minutes
- Release pipeline builds all targets within 20 minutes
- PR target check completes within 15s

**Constraints**:
- Reusable release workflow (`workflow_call` pattern) — single source of truth for all release builds
- Pre-release tags must never collide with stable tags
- Security scan must not block PR when vulnerability database is unreachable

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle I (Rust-Only Single Binary)**: PASS — Adding a MUSL target and native DEB/RPM packages extends Linux distribution support without changing the binary architecture. The MUSL binary remains a single, standalone executable.
- **Principle II (Speed-First Two-Tier Detection & BLAKE3)**: PASS — CI/CD changes do not affect runtime detection behavior.
- **Principle III (Graph Correctness & petgraph Reverse Traversal)**: PASS — No changes to graph engine.
- **Principle IV (Value Types & Chrono Derivations)**: PASS — No changes to value types.
- **Principle V (Multi-Channel Distribution & Frictionless UX)**: PASS — Enhances existing distribution channels: DEB/RPM packages improve Linux UX; MUSL static binary improves portability across all Linux distributions.
- **Principle VI (Dual Passive/Active Integration Surface)**: PASS — No changes to integration surfaces.
- **Principle VII (Permissive Open-Source Governance)**: PASS — MIT OR Apache-2.0 license unchanged. CI/CD pipeline hardening is standard open-source practice.

## Project Structure

### Documentation (this feature)

```text
specs/003-devops-enhancement/
├── plan.md              # Implementation plan
├── research.md          # Technical decisions (cargo-audit, release-please, MUSL, cargo-deb/rpm)
├── data-model.md        # Workflow configuration entities (jobs, triggers, artifacts)
├── quickstart.md        # Pipeline validation guide
├── contracts/           # Workflow interface schemas
│   ├── ci-schema.md     # CI job dependency chain specification
│   └── cd-schema.md     # CD release orchestration specification
└── tasks.md             # Task execution list
```

### Source Code (repository root)

```text
.github/
├── workflows/
│   ├── ci.yml            # PR quality gates: fmt → clippy → (test | security) in parallel
│   ├── cd.yml            # Continuous delivery: develop pre-releases + master stable releases
│   ├── release.yml       # Reusable build workflow (workflow_call + workflow_dispatch)
│   └── pr-target-check.yml # Branch protection enforcement
│
├── scripts/
│   └── check-test-presence.sh  # CI helper to verify test coverage on Rust modules

Cargo.toml                 # Add [package.metadata.deb] and [package.metadata.generate-rpm]
```

**Structure Decision**: Place all workflows in `.github/workflows/` following GitHub Actions conventions. Existing `ci.yml` and `release.yml` are refactored; `cd.yml` and `pr-target-check.yml` are new. Helper scripts live under `.github/scripts/`. One Rust source change: `src/updater/platform.rs` target triple string update (x86_64-unknown-linux-gnu → x86_64-unknown-linux-musl).

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

*No violations. All constitutional constraints strictly satisfied.*
