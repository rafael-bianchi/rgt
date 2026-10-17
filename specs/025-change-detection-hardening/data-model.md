# Data Model: Change-Detection Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for the detection changes. Existing `MetadataSnapshot`/`DetectionResult` gain fields/variants; `source_documents` gains identity columns.

## Entities

### MetadataSnapshot

The captured fingerprint.

| Attribute | Type | Notes |
|---|---|---|
| `mtime_nsec` | `i64` | Unchanged. |
| `file_size` | `u64` | Unchanged. |
| `dev` | `Option<u64>` | **New** — resolved target device (Unix `dev`; Windows `volume_serial_number`). |
| `ino` | `Option<u64>` | **New** — resolved target inode (Unix `ino`; Windows `file_index`). |

Rules:
- `tier1_check_changed` compares `mtime_nsec`, `file_size`, and (when both present) `dev`/`ino`. A repointed symlink (different inode) is `Changed` even with equal mtime+size (FR-004).
- `None` identity (e.g. a pre-025 row, or a platform without identity) is treated as "don't compare", not "changed" (no false positives on upgrade).

### DetectionResult

The outcome of `evaluate_file_change`.

| Variant | Notes |
|---|---|
| `Unchanged` | Tier 1 match and (when not forced) no hash; or hash matches. |
| `Changed { new_mtime_nsec, new_file_size, new_blake3_hash }` | Content (or identity) differs. |
| `FileNotFound` | `io::ErrorKind::NotFound` only (FR-003). |
| `CheckFailed` | **New** — any other I/O error; treated as "state unchanged", no stale marking (FR-003). |

### FileIdentity

The on-disk identity used to key `source_documents`.

| Attribute | Type | Notes |
|---|---|---|
| `dev` / `ino` | `Option<u64>` | Primary key when available (FR-005). |
| `file_path` | `String` | Human-readable path, retained; used as the fallback key when identity is unavailable. |

Rules:
- The same file via different casings resolves to one identity (same dev/ino), so one source document (FR-005).

### ReHashDecision

The per-check decision for the time-bounded re-hash (FR-001).

| Attribute | Type | Notes |
|---|---|---|
| `last_checked_at` | `DateTime` | From `source_documents`. |
| `force_hash` | `bool` | `now - last_checked_at > REHASH_THRESHOLD` (default 1h). |

Rules:
- `force_hash` skips the Tier-1 early-return and always hashes; otherwise the fast path is taken.
- After a check, `last_checked_at` is updated to now.

## Validation rules (from FR-001..FR-005)

- **Bounded false-negative window** (FR-001).
- **Fingerprint hashes extracted bytes** (FR-002).
- **NotFound vs CheckFailed** (FR-003).
- **Identity in fingerprint** (FR-004).
- **Identity-based dedupe** (FR-005).

## Out of scope

Full `PRAGMA user_version` migrations (029); garbage collection (029); `notify`-style watchers.
