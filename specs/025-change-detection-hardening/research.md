# Research: Change-Detection Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Resolves the plan's Technical Context unknowns. No external crates needed; the mechanics below are from `std` and the findings doc §7.4.

## 1. Time-bounded conditional re-hash (FR-001)

**Decision**: `evaluate_file_change` gains a `force_hash: bool` parameter. The invalidation loop (`evaluate_and_invalidate_all`) computes it from the source document's `last_checked_at`: `force_hash = now - last_checked_at > REHASH_THRESHOLD` (default 1h). When `force_hash`, skip the Tier-1 early-return and always compute the BLAKE3 hash; otherwise keep the current two-tier flow. After each check, update the document's `last_checked_at` to now.

**Rationale**: `source_documents` already carries `last_checked_at`, so no new state is needed. Hashing only stale-checked files bounds the same-size/same-mtime undetected window to the threshold while preserving the sub-ms no-change path for files checked recently (constitution II). A 1h threshold is generous for the interactive/agent loop and keeps amortized cost negligible.

**Alternatives considered**:
- `notify` OS event watching — rejected: RGT is a stateless per-invocation CLI with no long-lived watcher process; event subscriptions can't be held across invocations.
- Detect low mtime-resolution filesystems and hash every check — rejected: resolution detection is not portable and would hash the hot path on coarse filesystems.
- Always hash on `rgt status` — rejected: violates the <1ms budget (constitution II).

## 2. TOCTOU — hash the extracted bytes (FR-002)

**Decision**: Add `compute_blake3_hash_from_bytes(bytes: &[u8]) -> String` to `tier2_blake3.rs`. In `record.rs`, hash the already-read `bytes` vector (before `from_utf8`) instead of re-opening the file. In `hooks/mod.rs`, when content is read from disk (the no-`tool_response.content` fallback), hash those bytes; the `tool_response.content` path already hashes the file (which is the correct staleness identity), so it is unchanged.

**Rationale**: The `blake3` `Hasher::update(&[u8])` accepts an in-memory slice directly (docs.rs/blake3), so the bytes read for extraction can be hashed with no second `open()`. This removes the window where the file changes between extraction and hashing. `std::fs::File::metadata` on an open handle (the alternative) only narrows the window — hashing the buffer eliminates it for the extraction path.

**Alternatives considered**: single-handle read+stat via `File::metadata` — rejected as primary (narrows, doesn't eliminate, and doesn't apply to `--stdin`).

## 3. `NotFound` vs other I/O errors (FR-003)

**Decision**: Add `DetectionResult::CheckFailed`. In `evaluate_file_change`, map `get_metadata_snapshot`/`compute_blake3_hash` errors by `io::Error::kind()`: `ErrorKind::NotFound` → `FileNotFound`; anything else → `CheckFailed`. `invalidation.rs` treats `CheckFailed` as "state unchanged" (no stale marking), while `FileNotFound` keeps the `FILE_DELETED` stale reason.

**Rationale**: `io::Error::kind()` is the idiomatic discriminator; `ErrorKind` is `#[non_exhaustive]` so new kinds fall through to `CheckFailed` safely (std docs confirm). This stops permission/lock errors from masquerading as deletions.

**Alternatives considered**: none — `Error::kind()` is the one correct approach.

## 4. On-disk identity — dev/ino (FR-004) + dedupe (FR-005)

**Decision**: `MetadataSnapshot` gains `dev: Option<u64>`, `ino: Option<u64>` (Unix via `std::os::unix::fs::MetadataExt::{dev, ino}`); on Windows use `volume_serial_number()` + `file_index()` (both `Option<u64>`) as the analog. `tier1_check_changed` compares dev/ino so a symlink repoint to a different inode is a change. `source_documents` gains nullable `dev`/`ino` columns; `upsert_source_document` stores them, and source-document identity/dedupe keys on (dev, ino) when available instead of the raw path string (falling back to the path on platforms without identity). This single mechanism serves both FR-004 (symlink swap) and FR-005 (case-insensitive casing — the same file has the same inode regardless of casing).

**Rationale**: inode is the canonical "is this the same on-disk file" signal (docs.rs `MetadataExt`); it is cheaper and dependency-free versus the `same-file` crate, and it also fixes the case-insensitive duplication that `fs::canonicalize` cannot (its docs don't case-fold).

**Alternatives considered**:
- `same-file` crate (`is_same_file`) — rejected: a new dependency for functionality `dev`/`ino` already provides via std.
- `fs::canonicalize` for path identity — rejected: resolves symlinks/`.`/`..` but does not case-fold (std docs), so it cannot dedupe casing on macOS/Windows.

## 5. Additive schema columns (dev/ino)

**Decision**: Add `dev`/`ino` via the `CREATE TABLE` for new DBs and an idempotent `ALTER TABLE source_documents ADD COLUMN dev INTEGER` / `... ino INTEGER` guarded by `PRAGMA table_info` for existing DBs.

**Rationale**: SQLite supports additive `ALTER TABLE ... ADD COLUMN`; a `table_info` guard makes it idempotent without the full `PRAGMA user_version` migration framework (which stays in spec 029). Existing rows keep `NULL` dev/ino (no false "changed" on upgrade — `tier1` must treat a `None` identity as "don't compare", not "changed").
