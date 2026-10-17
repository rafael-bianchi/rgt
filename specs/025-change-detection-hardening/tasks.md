# Tasks: Change-Detection Hardening

**Input**: Design documents from `/specs/025-change-detection-hardening/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage — including the previously-untested `tier1`/`tier2`/`evaluate_file_change` algorithm.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4, US5)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/detection/{mod,tier1,tier2_blake3}.rs`, `src/cli/record.rs`, `src/hooks/mod.rs`, `src/graph/invalidation.rs`, `src/store/{schema,queries}.rs`, `src/types/node.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates)

- [x] T001 Confirm no new dependencies are required (identity via `std::os::{unix,windows}::fs::MetadataExt`; `notify`/`same-file` rejected — see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `MetadataSnapshot` dev/ino identity fields + `compute_blake3_hash_from_bytes` primitive that US1/US4/US5 and US2 depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `tests/unit/detection.rs`: `get_metadata_snapshot` on a real temp file reports its size and (on Unix) non-`None` `dev`/`ino`; `compute_blake3_hash_from_bytes(b) == compute_blake3_hash(path)` for a file whose bytes are `b`; `tier1_check_changed` returns `true` when dev/ino differ (both present) and `false` when only a `None` identity is involved

### Implementation for Foundational Primitives

- [x] T003 Add `dev: Option<u64>` and `ino: Option<u64>` to `MetadataSnapshot` in `src/detection/tier1.rs`, populated from `std::os::unix::fs::MetadataExt::{dev,ino}` (and, on Windows, `volume_serial_number`/`file_index`); update `tier1_check_changed` to compare dev/ino when both are present
- [x] T004 Add `compute_blake3_hash_from_bytes(bytes: &[u8]) -> String` to `src/detection/tier2_blake3.rs`

**Checkpoint**: foundational tests pass; identity fields + hash-from-bytes available

---

## Phase 3: User Story 1 - A Same-Size, Same-Mtime Rewrite Is Detected (Priority: P1) 🎯 MVP

**Goal**: Tier 1 no longer confirms "unchanged" indefinitely — a time-bounded forced re-hash (FR-001) detects same-mtime/size content changes while preserving the sub-ms fast path

**Independent Test**: With `force_hash=true`, a same-size/same-mtime content rewrite is `Changed`; with `force_hash=false` and matching Tier-1, `Unchanged` without hashing

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T005 [P] [US1] Unit tests in `tests/unit/detection.rs`: `evaluate_file_change` with `force_hash=true` returns `Changed` for a rewritten-same-mtime-same-size file (write new content, restore mtime via `set_file_times`); with `force_hash=false` and matching mtime/size returns `Unchanged` for an untouched file (the fast-path observable result — no timing assertion)

### Implementation for User Story 1

- [x] T006 [US1] Add a `force_hash: bool` parameter to `evaluate_file_change` in `src/detection/mod.rs`; when `force_hash`, skip the Tier-1 early-return and always compute the hash; otherwise keep the current flow
- [x] T007 [US1] In `evaluate_and_invalidate_all` (`src/graph/invalidation.rs`), `SELECT last_checked_at` for each source document, compute `force_hash` from a `REHASH_THRESHOLD` (default 1h), pass it to `evaluate_file_change`, and update `last_checked_at` to now after each check

**Checkpoint**: US1 tests pass; same-mtime rewrites are detected within the window, fast path preserved

---

## Phase 4: User Story 2 - The Fingerprint Hashes the Exact Extracted Bytes (Priority: P1)

**Goal**: `rgt record` and the hook's disk-read fallback hash the same in-memory bytes they extracted (FR-002)

**Independent Test**: The stored hash equals the hash of the bytes that were parsed for values

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T008 [P] [US2] Unit test in `tests/unit/detection.rs`: `compute_blake3_hash_from_bytes` over a content buffer equals `compute_blake3_hash` over the same file; a contract test in `tests/contract/test_cli_record_derive_contract.rs` records a file and asserts the stored `blake3_hash` equals the hash of the file's bytes

### Implementation for User Story 2

- [x] T009 [US2] In `execute_record` (`src/cli/record.rs`), compute the hash from the already-read `bytes` (before `from_utf8`) via `compute_blake3_hash_from_bytes` instead of re-opening the file
- [x] T010 [US2] In `handle_passive_hook_event` (`src/hooks/mod.rs`), when content is read from disk (the no-`tool_response.content` fallback), hash those bytes; the `tool_response.content` path keeps hashing the file (correct staleness identity)

**Checkpoint**: US2 tests pass; fingerprints hash the extracted bytes

---

## Phase 5: User Story 3 - Deleted vs. Unreadable Files Are Distinguished (Priority: P2)

**Goal**: Only `io::ErrorKind::NotFound` → `FileNotFound`/`FILE_DELETED`; other I/O errors → a new `CheckFailed` (state unchanged) (FR-003)

**Independent Test**: A missing file → `FileNotFound`; a permission-denied file → `CheckFailed`

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T011 [P] [US3] Unit tests in `tests/unit/detection.rs`: `evaluate_file_change` on a nonexistent path → `FileNotFound`; on an existing-but-unreadable path → `CheckFailed` (assert the variant, not a panic)

### Implementation for User Story 3

- [x] T012 [US3] Add `DetectionResult::CheckFailed` in `src/detection/mod.rs`; in `evaluate_file_change`, map `get_metadata_snapshot`/`compute_blake3_hash` errors via `io::Error::kind()` (`NotFound` → `FileNotFound`, else `CheckFailed`); in `invalidation.rs`, treat `CheckFailed` as "no change" (never mark stale)

**Checkpoint**: US3 tests pass; errors are no longer conflated

---

## Phase 6: User Story 4 - A Retargeted Symlink Is Detected (Priority: P2)

**Goal**: A symlink repointed to a different inode (even with matching mtime+size) is `Changed` (FR-004)

**Independent Test**: `tier1_check_changed` returns `true` when dev/ino differ (both present)

### Tests for User Story 4 (write FIRST, expect FAIL) ⚠️

- [x] T013 [P] [US4] Unit test in `tests/unit/detection.rs` (Unix-only, `#[cfg(unix)]`): a symlink repointed from target A to target B (same size, same mtime) yields a `tier1_check_changed` of `true` via the differing inode

### Implementation for User Story 4

- [x] T014 [US4] Store dev/ino in `source_documents`: add nullable `dev`/`ino` columns in `src/store/schema.rs` (new DBs) + an idempotent `ALTER TABLE ... ADD COLUMN` guarded by `PRAGMA table_info` (existing DBs); update `SourceDocument` (`src/types/node.rs`) and **every** query that reads or writes `source_documents` (`upsert_source_document`, `get_source_document_by_path`, `list_all_nodes`/`list_source_documents`, and any other `SELECT`/row-mapping over that table) in `src/store/queries.rs` to persist/load the columns, and `evaluate_and_invalidate_all` to pass the stored identity into the change check

**Checkpoint**: US4 tests pass; symlink repoints are detected

---

## Phase 7: User Story 5 - Same File Via Different Casings Is Not Double-Tracked (Priority: P3)

**Goal**: Source-document identity uses on-disk identity (dev/ino) when available, so case-differing paths to one file dedupe (FR-005)

**Independent Test**: Two differently-cased paths to the same file produce one source document

### Tests for User Story 5 (write FIRST, expect FAIL) ⚠️

- [x] T015 [P] [US5] Unit test in `tests/unit/detection.rs`: `upsert_source_document` called twice with differently-cased paths resolving to the same file (same dev/ino) yields one row (on platforms with identity; skip elsewhere)

### Implementation for User Story 5

- [x] T016 [US5] In `upsert_source_document` (`src/store/queries.rs`), dedupe on `(dev, ino)` when both are present (with `file_path` as the fallback key), so casing variants merge into one row

**Checkpoint**: US5 tests pass; no duplicate rows for casing variants

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation

- [x] T017 [P] Update `README.md`/`--help`-adjacent docs for the new `CHECK_FAILED` stale reason and the detection changes
- [x] T018 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T019 Run the manual scenarios in `specs/025-change-detection-hardening/quickstart.md` (same-mtime rewrite; unreadable vs deleted)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational — `detection/mod.rs` + `invalidation.rs`
- **US2 (Phase 4)**: Depends on Foundational — `record.rs` + `hooks/mod.rs` (uses `compute_blake3_hash_from_bytes`)
- **US3 (Phase 5)**: Depends on Foundational — `detection/mod.rs` (shares the file with US1)
- **US4 (Phase 6)**: Depends on Foundational — `schema.rs`/`queries.rs`/`types/node.rs`
- **US5 (Phase 7)**: Depends on US4 (dev/ino columns exist) — `queries.rs`
- **Polish (Phase 8)**: Depends on all

### User Story Dependencies

- **US1 (P1)**: after Foundational — forced re-hash
- **US2 (P1)**: after Foundational — TOCTOU (independent of US1)
- **US3 (P2)**: after Foundational — error distinction
- **US4 (P2)**: after Foundational — dev/ino persistence
- **US5 (P3)**: after US4 — dedupe on identity

### Within Each User Story

- Tests written first, must FAIL before implementation
- Foundational identity fields + hash-from-bytes (T003/T004) before the stories
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 ∥ T005 ∥ T008 ∥ T011 ∥ T013 ∥ T015 (all `tests/unit/detection.rs`, but different test functions — write them together, then implement)
- T003 ∥ T004 (independent files)
- US1/US2/US3/US4 are independent (different files); US4/US5 both touch `queries.rs` → sequential
- T017 ∥ T018

---

## Parallel Example: Foundational + Story Tests

```bash
Task: "identity + hash-from-bytes tests (tests/unit/detection.rs)"
Task: "forced re-hash tests (US1)"
Task: "fingerprint-hash tests (US2)"
Task: "NotFound/CheckFailed tests (US3)"
Task: "symlink identity tests (US4)"
Task: "casing dedupe tests (US5)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + 2 (Foundational: dev/ino + hash-from-bytes)
2. Phase 3: User Story 1 (forced re-hash) — the §7.4 HIGH false-negative fix
3. **STOP and VALIDATE**: same-mtime rewrites are detected; fast path preserved
4. Then US2/US3/US4/US5 incrementally

### Incremental Delivery

1. Foundational → identity + hash-from-bytes
2. US1 → time-bounded re-hash (MVP)
3. US2 → TOCTOU fix
4. US3 → error distinction
5. US4 → dev/ino persistence (symlink)
6. US5 → identity dedupe (casing)
7. Polish → docs + full-suite validation

---

## Notes

- [P] tasks = different files, no dependencies; US1/US3 both edit `src/detection/mod.rs`, US4/US5 both edit `src/store/queries.rs` → run sequentially within those files
- [Story] label maps task to user story for traceability
- `dev`/`ino` are `Option` — pre-025 rows have `NULL` identity and must NOT be treated as changed on upgrade (tier1 guards this)
- `notify`/`same-file` crates are rejected (see research); no new dependencies
- Commit after each task or logical group; stop at each checkpoint
