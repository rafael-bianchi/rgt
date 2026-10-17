# Contract: Change-Detection Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the unit tests for `evaluate_file_change`, `tier1_check_changed`, `get_metadata_snapshot`, `compute_blake3_hash*`, and the record/hook fingerprint paths.

## Detection invariants

1. **Bounded false negative (FR-001)**: with `force_hash = true` (stale `last_checked_at`), a file whose mtime+size match but whose content differs is reported `Changed`. With `force_hash = false` and matching Tier-1, the result is `Unchanged` without hashing.
2. **Error conflation (FR-003)**: `io::ErrorKind::NotFound` → `FileNotFound`; any other I/O error → `CheckFailed`. `CheckFailed` never marks a node stale; `FileNotFound` maps to `FILE_DELETED`.
3. **Identity (FR-004)**: `tier1_check_changed` returns `true` when `dev`/`ino` differ (both present). A `None` identity is never a change by itself.

## Fingerprint invariants (FR-002)

- `rgt record`'s stored BLAKE3 hash equals the hash of the exact bytes parsed for values.
- The hook's disk-read fallback path stores the hash of the bytes it parsed (not a separate re-read).

## Source-document identity (FR-005)

- Two paths to the same on-disk file (same dev/ino, e.g. different casings) produce a single `source_documents` row.

## Acceptance linkage

- US1/FR-001/SC-001/SC-002 → invariant 1.
- US2/FR-002/SC-003 → fingerprint invariants.
- US3/FR-003/SC-004 → invariant 2.
- US4/FR-004/SC-005 → invariant 3.
- US5/FR-005/SC-006 → source-document identity.
