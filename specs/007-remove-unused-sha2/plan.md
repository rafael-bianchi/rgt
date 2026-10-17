# Implementation Plan: Remove Unused sha2 Dependency

**Branch**: `007-remove-unused-sha2` | **Date**: 2026-08-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/007-remove-unused-sha2/spec.md`

## Summary

Investigate whether the `sha2` crate declared in `Cargo.toml` is unused and can be safely removed. However, the investigation already confirmed sha2 is actively used by `src/updater/checksum.rs` for SHA-256 checksum verification during `rgt update`. No code changes are required — the dependency is legitimate. The plan documents the finding and provides a closure recommendation for GitHub issue #4.

## Technical Context

**Language/Version**: Rust Edition 2021
**Primary Dependencies**: `sha2 = "0.10"` (actively used), `blake3` (separate concern for file-change detection)
**Storage**: N/A (no schema changes)
**Testing**: `cargo test --all`
**Target Platform**: Linux (musl/gnu), macOS, Windows (cross-compiled)
**Project Type**: CLI tool (single binary)
**Performance Goals**: N/A (no behavioral changes)
**Constraints**: N/A (no changes)
**Scale/Scope**: Investigation-only; 0 files to modify

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | sha2 is a compile-time dep producing one binary; no runtime deps added |
| II | Two-Tier Detection & BLAKE3 | ✅ PASS | sha2 is separate from blake3; blake3 remains for Tier 2 file detection |
| III | Graph Correctness | ✅ PASS | No graph changes |
| IV | Rich Value Types | ✅ PASS | No type changes |
| V | Multi-Channel Distribution | ✅ PASS | sha2 used for release checksum verification (essential for CD integrity) |
| VI | Dual Integration Surface | ✅ PASS | No interface changes |
| VII | Permissive OSS | ✅ PASS | No licensing changes; sha2 is MIT/Apache-2.0 compatible |

**Gate Result**: All principles pass. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/007-remove-unused-sha2/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0: investigation findings
├── quickstart.md        # Phase 1: verification commands
└── tasks.md             # Phase 2: `/speckit.tasks` (not created here)
```

### Source Code (repository root)

No changes required. Affected files for reference only:

```text
src/
├── updater/
│   └── checksum.rs      # Uses sha2::{Digest, Sha256} — active consumer
├── main.rs              # mod updater
└── lib.rs               # pub mod updater

Cargo.toml               # sha2 = "0.10" — retained
Cargo.lock               # Unchanged (no dependency removal)
```

**Structure Decision**: No structural changes. The updater module legitimately depends on sha2 for release artifact checksum verification.

## Complexity Tracking

> No violations to justify.

