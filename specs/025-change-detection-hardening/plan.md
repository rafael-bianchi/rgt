# Implementation Plan: Change-Detection Hardening

**Branch**: `025-change-detection-hardening` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/025-change-detection-hardening/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Close the change-detection gaps (findings §7.4):

1. **Tier-1 false negative (FR-001)**: `evaluate_file_change` can no longer trust Tier 1 forever. A time-bounded conditional re-hash (clarified 2026-08-29): when a source document's `last_checked_at` is older than a threshold (default 1h), force a BLAKE3 hash even if mtime+size match; otherwise keep the sub-ms fast path.
2. **TOCTOU (FR-002)**: the recorded fingerprint hashes the same in-memory bytes that were extracted (`rgt record` and the hook's disk-read fallback), via a new `compute_blake3_hash_from_bytes`.
3. **Error conflation (FR-003)**: `DetectionResult` gains `CheckFailed`; only `io::ErrorKind::NotFound` maps to `FileNotFound`/`FILE_DELETED`, other I/O errors → `CheckFailed` (state unchanged, no stale marking).
4. **On-disk identity (FR-004/FR-005)**: the fingerprint captures the resolved target's `dev`/`ino` (Unix `MetadataExt`) — and volume-serial + file-index on Windows — so a symlink repointed to a different inode is a change, and the same file reached via different casings is one identity. This also drives source-document dedupe.

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: `std` only — `std::os::unix::fs::MetadataExt` (`dev`/`ino`) and `std::os::windows::fs::MetadataExt` (`volume_serial_number`/`file_index`). **No new crates** (the `same-file` crate is unnecessary because dev/ino gives the same identity natively; the `notify` crate is rejected — RGT is a stateless per-invocation CLI).

**Storage**: `source_documents` gains two nullable identity columns (`dev INTEGER`, `ino INTEGER`) via an idempotent additive `ALTER TABLE` for existing DBs and the `CREATE TABLE` for new DBs (SQLite supports additive columns; the full `PRAGMA user_version` migration framework remains in spec 029).

**Testing**: `cargo test` — new unit tests for `tier1`/`tier2`/`evaluate_file_change` (the previously-untested algorithm, addressing §7.4's "no test coverage" note), TOCTOU hash-from-bytes, `NotFound` vs `CheckFailed`, dev/ino change, and the time-bounded re-hash. `cargo fmt`/`clippy`.

**Target Platform**: macOS, Linux, Windows (identity via platform `MetadataExt`; non-Unix uses volume-serial + file-index where available, else falls back to current behavior).

**Project Type**: CLI tool (Rust single binary).

**Performance Goals**: the no-change path stays sub-ms (constitution II): the forced re-hash only triggers when `last_checked_at` is stale; unchanged recently-checked files never hash.

**Constraints**:
- FR-001: re-hash threshold default 1h, configurable via a constant; fast path preserved.
- FR-002: `record` + hook disk-fallback hash the extracted bytes.
- FR-003: `FileNotFound` only for `NotFound`; `CheckFailed` for other errors (no stale marking).
- FR-004: `MetadataSnapshot` gains dev/ino; `tier1_check_changed` compares them; `source_documents` stores them.
- FR-005: on-disk identity (dev/ino) is the dedupe key for source documents, not the raw path string.

**Scale/Scope**: edits to `src/detection/{mod,tier1,tier2_blake3}.rs`, `src/cli/record.rs`, `src/hooks/mod.rs`, `src/graph/invalidation.rs` (select/update `last_checked_at`, pass `force_hash`), `src/store/schema.rs` (columns), `src/store/queries.rs` (`upsert_source_document` stores/returns identity), and the `SourceDocument` type. Tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new dependencies (std only). |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Preserved — two-tier kept; the re-hash is conditional/time-bounded, no-change path stays sub-ms. |
| III. Graph Correctness | ✅ PASS | Correctness improved (stale detection). |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Unaffected. |
| IX. Test-Driven Verification | ✅ PASS | Adds the previously-missing tier1/tier2 test coverage; TDD for each finding. |
| X. Documentation Matches Behavior | ✅ PASS | New `CHECK_FAILED` stale reason documented. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy; platform-gated identity via `#[cfg]`. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/025-change-detection-hardening/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (detection design)
├── data-model.md        # Phase 1 output (detection domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (detection invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── detection/
│   ├── mod.rs          # MODIFIED: DetectionResult::CheckFailed; force_hash path; NotFound distinction
│   ├── tier1.rs        # MODIFIED: MetadataSnapshot gains dev/ino; tier1 compares them
│   └── tier2_blake3.rs # MODIFIED: compute_blake3_hash_from_bytes
├── cli/record.rs       # MODIFIED: hash the extracted bytes (TOCTOU)
├── hooks/mod.rs        # MODIFIED: hash the disk-read bytes (TOCTOU fallback)
├── graph/invalidation.rs  # MODIFIED: select/update last_checked_at; pass force_hash
└── store/
    ├── schema.rs       # MODIFIED: dev/ino columns (additive)
    └── queries.rs      # MODIFIED: upsert_source_document stores/returns identity

tests/
└── unit/detection.rs   # MODIFIED/EXPANDED: tier1/tier2/evaluate_file_change unit tests
```

**Structure Decision**: Single-project layout; the changes are localized to the detection module, its two callers (`record`/`hooks`), and the store identity columns.

## Complexity Tracking

None — no constitution violations; no new dependencies.
