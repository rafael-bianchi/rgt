# Feature Specification: Change-Detection Hardening

**Feature Branch**: `feat/025-change-detection-hardening`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "File-change detection must never silently miss a real change: a same-second/same-size rewrite (Tier 1 false negative) must not be trusted as unchanged forever, the recorded fingerprint must hash the exact bytes that were extracted (no TOCTOU), a missing file must be distinguished from a permission/lock error, a retargeted symlink must be detected, and the same file reached via different path casings must not be double-tracked. Findings §7.4."

Source context: `docs/findings-hooks-and-value-extraction.md` §7.4 (plan: `docs/spec-plan-from-findings.md`, spec #7, P1).

## Clarifications

### Session 2026-08-29

- Q: How should detection close the Tier 1 false-negative gap (same-size/same-mtime rewrite trusted as unchanged)? → A: Time-bounded conditional re-hash — force a BLAKE3 hash when the file's `last_checked_at` is older than a threshold (e.g. 1 hour), even if mtime+size match; otherwise keep the sub-ms fast path. Bounds the undetected window; uses existing DB state.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Same-Size, Same-Mtime Rewrite Is Detected (Priority: P1)

A file is rewritten with identical length and an identical (or coarse-resolution) mtime — e.g. `git checkout`/`stash pop` restoring a prior version, or a same-second edit on a FAT32/exFAT/NFS filesystem. Today Tier 1 (mtime + size) sees no difference and returns "unchanged" without ever hashing, so dependent derived values stay fresh despite a real content change — forever, until the mtime/size happens to change. The detection must not let Tier 1 alone confirm "unchanged" unconditionally.

**Why this priority**: §7.4 HIGH — the staleness guarantee (the tool's core correctness promise) is undermined: a genuinely stale value is reported fresh.

**Independent Test**: Unit tests assert that a same-size, same-mtime rewrite is detected as changed (via the chosen re-hash mechanism), while a genuinely-unchanged file still takes the fast path.

**Acceptance Scenarios**:

1. **Given** a file whose content changed but whose stored mtime+size are identical, **When** detection runs, **Then** it is reported `Changed` (not silently `Unchanged`).
2. **Given** a file that has genuinely not changed, **When** detection runs, **Then** it is reported `Unchanged` on the fast path (no forced full re-hash within the staleness window).
3. **Given** a file checked recently (within the staleness window), **When** detection runs, **Then** the no-change path completes without a full content hash (constitution II <1ms budget).

---

### User Story 2 - The Fingerprint Hashes the Exact Extracted Bytes (Priority: P1)

When `rgt record` (or the hook) records a file, it reads the content once for extraction, then separately re-opens the file for metadata and re-opens it again for hashing — nothing confirms the file didn't change in between, so the stored `(mtime, size, hash)` fingerprint can describe different bytes than were actually extracted. The stored fingerprint must hash the same in-memory bytes that were extracted.

**Why this priority**: §7.4 HIGH — a torn read silently corrupts provenance (the fingerprint says "these bytes" but the extracted values came from "those bytes").

**Independent Test**: Unit tests assert the hash recorded for a file equals the hash of the exact bytes that were extracted (e.g., by hashing the in-memory content buffer, not a separate re-read).

**Acceptance Scenarios**:

1. **Given** `rgt record` on a file, **When** the fingerprint is computed, **Then** the stored BLAKE3 hash is computed from the same bytes that were parsed for values (single read).
2. **Given** the hook path, **When** it captures a file's content, **Then** its stored hash matches the content it extracted (not a separate re-read).

---

### User Story 3 - Deleted vs. Unreadable Files Are Distinguished (Priority: P2)

A file that cannot be read — permission denied, a Windows file lock, a path now pointing at a directory — is today collapsed into `FileNotFound` and labeled `FILE_DELETED`, misleading the agent about why a value went stale. Only a genuine `NotFound` must be labeled `FILE_DELETED`; other I/O errors must be labeled distinctly (e.g. "check failed").

**Why this priority**: §7.4 MEDIUM — wrong staleness reasons misdirect the agent's recovery (it may try to "re-read a deleted file" when the real problem is a permission).

**Independent Test**: Unit tests assert `NotFound` maps to `FileNotFound`, while a permission error and other I/O errors map to a distinct "check failed, state unchanged" result.

**Acceptance Scenarios**:

1. **Given** a genuinely missing file, **When** detection runs, **Then** it is `FileNotFound` (stale reason `FILE_DELETED`).
2. **Given** a file that exists but fails to read (permission denied), **When** detection runs, **Then** it is NOT `FileNotFound`; it is a distinct "check failed" result that does not mark the node stale.

---

### User Story 4 - A Retargeted Symlink Is Detected (Priority: P2)

A symlink is repointed at a different file whose mtime+size coincidentally match the stored values. Today the swap is invisible. The fingerprint must include the resolved target's identity (`st_dev`/`st_ino`), so a repoint to a different inode is a change even when mtime+size match.

**Why this priority**: §7.4 LOW-MEDIUM — symlink swaps are easy to construct and silently defeat detection; inode identity is a cheap, decisive fix on Unix.

**Independent Test**: Unit tests assert the fingerprint includes device/inode of the resolved target, and that a symlink repointed to a different inode (same mtime+size) is detected as changed.

**Acceptance Scenarios**:

1. **Given** a symlink repointed to a different target with the same mtime+size, **When** detection runs, **Then** it is `Changed` (different dev/inode).
2. **Given** a normal file, **When** detection runs, **Then** behavior is unchanged (dev/inode captured alongside mtime+size).

---

### User Story 5 - The Same File Via Different Path Casings Is Not Double-Tracked (Priority: P3)

On case-insensitive filesystems (default macOS/Windows), a file recorded under one casing and reported under another creates two independent tracking rows. Paths must be canonicalized/identified so the same on-disk file is one row regardless of casing.

**Why this priority**: §7.4 LOW — duplicate rows split the graph and double-count values; lower frequency but a real correctness gap on the most common desktop OSes.

**Independent Test**: Unit tests assert that two differently-cased paths resolving to the same file are treated as one identity (not two source documents).

**Acceptance Scenarios**:

1. **Given** a file recorded as `Data.CSV` and re-reported as `data.csv` on a case-insensitive filesystem, **When** detection runs, **Then** they resolve to one source document (no duplicate row).

---

### Edge Cases

- What is the staleness window for the forced re-hash — how long can a same-mtime change go undetected before the next full hash?
- How is the `last_checked_at` timestamp updated, and does the forced re-hash interact with `rgt record` (which always hashes)?
- On non-Unix platforms without `st_ino`, what is the fallback for symlink/file identity?
- A file that is deleted AND the path now points at a directory — is that `NotFound` or `Changed`/"check failed"?
- Does the TOCTOU fix (hash-in-memory) also apply to the `--stdin` path where there is no second read anyway?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Detection MUST NOT let Tier 1 alone confirm "unchanged" indefinitely: a real content change with identical mtime+size MUST be detected within a bounded staleness window, while the common no-change path MUST remain sub-millisecond (constitution II). Mechanism (clarified 2026-08-29): force a content hash when the file's `last_checked_at` is older than a threshold (e.g. 1 hour), even if mtime+size match; otherwise take the fast path.
- **FR-002**: The fingerprint stored for a recorded file MUST hash the exact in-memory bytes that were extracted (single read), not a separate re-read — in both `rgt record` and the hook path.
- **FR-003**: Detection MUST distinguish `io::ErrorKind::NotFound` (→ `FileNotFound`/`FILE_DELETED`) from other I/O errors (→ a distinct "check failed, state unchanged" result that does not mark the node stale).
- **FR-004**: The file-change fingerprint MUST include the resolved target's device and inode (`st_dev`/`st_ino`) on platforms that expose them, so a symlink repointed to a different inode (even with matching mtime+size) is detected as changed.
- **FR-005**: Source-document identity MUST be based on on-disk file identity (not byte-exact path strings), so the same file reached via different path casings is a single tracking row.

### Key Entities *(include if feature involves data)*

- **MetadataSnapshot**: The captured fingerprint (mtime_nsec, file_size, and now dev/inode where available).
- **DetectionResult**: The outcome — `Unchanged`, `Changed{...}`, `FileNotFound`, and now a distinct `CheckFailed` for non-NotFound I/O errors.
- **FileIdentity**: The on-disk identity used to key `source_documents` (canonical path + dev/inode).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A same-size, same-mtime content rewrite is reported `Changed` within the staleness window (unit-tested).
- **SC-002**: A genuinely unchanged file still completes the no-change path without a full content hash (sub-millisecond; asserted).
- **SC-003**: The recorded hash equals the hash of the extracted bytes (unit-tested via an in-memory buffer).
- **SC-004**: A `NotFound` error maps to `FileNotFound`; a permission error maps to a distinct non-stale result (unit-tested).
- **SC-005**: A symlink repointed to a different inode with matching mtime+size is `Changed` (unit-tested on Unix).
- **SC-006**: Two differently-cased paths to the same file produce one source document (unit-tested).

## Assumptions

- Constitution II's two-tier design and <1ms no-change budget are preserved; the Tier 1 false-negative fix is the **time-bounded conditional re-hash** (clarified 2026-08-29): hash when `last_checked_at` exceeds a threshold (default 1 hour), otherwise fast path. The exact threshold value is a plan-level default.
- `st_dev`/`st_ino` are captured on Unix; non-Unix platforms keep the current mtime+size behavior (documented fallback).
- The `notify` crate (OS event watching) is NOT adopted: RGT is a stateless per-invocation CLI with no long-lived watcher process, so event-driven watching is a poor fit (noted in research).
- `rgt record` always hashes (it's reading the file anyway), so the forced-re-hash window applies to `rgt status`'s staleness evaluation, not to record's fingerprinting.
- The `FileNotFound`→`CheckFailed` split changes the `stale_reason` surfaced to agents; the existing `FILE_DELETED` string is preserved for genuine deletions, and a new reason (e.g. `CHECK_FAILED`) is added.
