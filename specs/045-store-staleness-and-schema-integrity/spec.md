# Feature Specification: Store Staleness and Schema Integrity

**Feature Branch**: `fix/045-store-staleness-and-schema-integrity`

**Created**: 2026-10-17

**Status**: Draft

**PriorityOrder**: 1 of 6 (P1 — Critical Core Provenance & Store Integrity)

**Input**: Critical review findings §1.1–1.7: fix staleness erasure on re-record/hook re-read, fix `(dev, ino)` deduplication orphaning via `ON DELETE SET NULL`, add missing v2→v3 schema migration for `duration_secs` and `DURATION` check constraint, fix `REHASH_THRESHOLD` reset on Tier-1 checks, enforce staleness evaluation before `query`/`derive`/`verify`/`graph`/`gc`, make `rgt gc` iterative to a fixed point, eliminate read-path write locks in `get_or_create_rdf_project_id`, wrap `upsert_source_document` inside the atomic write transaction, and resolve existing `.rgt/store.db` project roots without auto-creating stores in subdirectories or uninitialized projects during passive hooks.

**Strict Non-Overlap Boundary**:
- **Owns**: `src/store/db.rs`, `src/store/queries.rs`, `src/store/schema.rs`, `src/graph/engine.rs`, `src/graph/invalidation.rs`, `src/detection/mod.rs`, `src/detection/tier1.rs`, `src/cli/gc.rs`.
- **Does NOT touch**: value/date regex parsing (`046`), expression/duration math or node ID hashing (`047`), hook payload dialects (`048`), `rgt init`/`rgt doctor` config editors (`049`), or CLI formatting/Turtle rendering/updater (`050`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Re-reading or Re-recording an Edited File Invalidates Obsolete Values (Priority: P1)

When a developer edits a tracked file (e.g., changing `100` on line 1 to `200` or shifting lines) and an agent immediately reads the file (triggering the passive `PostToolUse` hook) or runs `rgt record <file>`, RGT must mark any previously recorded root nodes for that document that are no longer present at the same `(line_number, occurrence, value)` as `STALE` (`SOURCE_FILE_MODIFIED`) and cascade staleness to all downstream derived nodes before updating `source_documents.blake3_hash`.

**Why this priority**: Without this, the most common workflow (read file → edit file → agent reads updated file → check status) overwrites `source_documents.blake3_hash` on the second read and leaves old root nodes and their broken derivations permanently `VALID`.

**Independent Test**: Record `data.csv` containing `100`, derive `node_drv` (`a * 2 = 200`), overwrite `data.csv` with `300`, run `rgt record data.csv` (or passive hook), and verify the `100` root node and `node_drv` are immediately marked `is_stale = true` while `300` is recorded as `is_stale = false`.

**Acceptance Scenarios**:

1. **Given** a tracked file `data.csv` with root node `N1` (`100` at line 1) and derived child `D1`, **When** `data.csv` is modified to `200` and recorded via `rgt record data.csv` or passive hook before `rgt status` runs, **Then** `N1` and `D1` are marked `is_stale = true` with reason `SOURCE_FILE_MODIFIED` and `N2` (`200` at line 1) is inserted with `is_stale = false`.
2. **Given** a tracked file where a value remains unchanged at the same line and occurrence while another line changes, **When** the file is re-recorded, **Then** the unchanged root node and its derivations remain `is_stale = false` while only removed/changed root nodes and their downstream subgraphs are invalidated.

---

### User Story 2 - Path Aliasing and `(dev, ino)` Deduplication Preserve `source_doc_id` Links (Priority: P1)

When the same physical file is recorded via different path spellings (e.g., `data.csv`, `./data.csv`, `/abs/path/to/data.csv`, or case variants on macOS/Windows) that resolve to the same `(dev, ino)` or canonical project-relative path, RGT must normalize paths relative to the project root where possible and re-key `tracked_nodes.source_doc_id` to the surviving `source_documents.id` before removing any duplicate `source_documents` row.

**Why this priority**: Currently, deleting the duplicate `(dev, ino)` row triggers SQLite's `ON DELETE SET NULL` on `tracked_nodes.source_doc_id`, permanently detaching existing root nodes from staleness tracking.

**Independent Test**: Record `data.csv`, then record `./data.csv` and `/abs/path/data.csv`; modify `data.csv` and run `rgt status` — all root nodes from the file must retain a non-null `source_doc_id` and transition to `is_stale = true`.

**Acceptance Scenarios**:

1. **Given** `data.csv` recorded with root nodes referencing `source_doc_id = 1`, **When** the same file is recorded via `/abs/path/data.csv` (same `dev` and `ino`), **Then** all `tracked_nodes` referencing the old `source_documents.id` are updated to the surviving `source_documents.id` prior to deleting the duplicate row, and zero nodes have `source_doc_id = NULL`.
2. **Given** any path inside the project root (`./sub/data.csv` or `/abs/project/sub/data.csv`), **When** normalized for storage, **Then** RGT canonicalizes it to the clean project-relative path `sub/data.csv`.

---

### User Story 3 - Seamless Schema v2→v3 Upgrade for Existing Databases (Priority: P1)

Users upgrading from earlier RGT versions (schema v1 or v2, where `tracked_nodes` lacked `duration_secs` and restricted `value_kind` to `'NUMBER', 'DATE'`) can open their existing `.rgt/store.db` and record/derive `DURATION` values without SQLite errors, and opening a store created by a newer `SCHEMA_VERSION` fails cleanly before executing any DDL mutations.

**Why this priority**: Existing v2 databases currently crash with `no such column: duration_secs` or `CHECK constraint failed` on `v0.7.0`.

**Independent Test**: Create a v2 SQLite database without `duration_secs` and with `CHECK(value_kind IN ('NUMBER', 'DATE'))`, open it with `DbStore::open_in_project`, verify `SCHEMA_VERSION = 3` migration rebuilds `tracked_nodes` preserving all rows/edges/captures, and insert a `DURATION` node.

**Acceptance Scenarios**:

1. **Given** an existing schema v1 or v2 `.rgt/store.db` lacking `duration_secs`, **When** opened by RGT, **Then** `stamp_or_migrate_schema` migrates `tracked_nodes` to schema v3 (adding `duration_secs INTEGER` and `CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION'))`) with all existing nodes, edges, and capture attributions intact.
2. **Given** a database with `PRAGMA user_version > SCHEMA_VERSION`, **When** opened by RGT, **Then** version validation runs *before* `initialize_schema` and returns an error without mutating tables or columns.

---

### User Story 4 - Automatic Staleness Evaluation Across All Active Commands & Multi-Layer GC (Priority: P2)

Running `rgt query`, `rgt derive`, `rgt verify`, `rgt graph`, or `rgt gc` after a source file is modified or deleted automatically evaluates file changes first, and `rgt gc` iteratively removes entire stale derivation chains in a single invocation.

**Why this priority**: Users and agents should never observe stale values reported as `VALID` in `query`/`graph` or have `rgt gc` require $N$ manual runs for a chain of depth $N$.

**Independent Test**: Record a file, build a 3-hop derivation chain `root -> d1 -> d2 -> d3`, delete the file, and run `rgt gc` directly (without running `rgt status` first); all 4 nodes and the orphaned `source_documents` row are removed in one run.

**Acceptance Scenarios**:

1. **Given** a tracked file has been modified or deleted on disk, **When** the user runs `rgt query`, `rgt graph`, `rgt verify`, `rgt derive`, or `rgt gc` directly, **Then** `InvalidationCascade::evaluate_and_invalidate_all` executes first so staleness state is current.
2. **Given** a stale derivation chain of depth $k \ge 2$ with no fresh dependents, **When** `rgt gc` runs once, **Then** `delete_obsolete_nodes` repeats until no further stale leaf nodes remain, removing all $k$ levels and any `source_documents` rows whose files are deleted and have zero remaining nodes.
3. **Given** a tracked file checked via Tier 1 (`mtime` + `size`) within `REHASH_THRESHOLD`, **When** Tier 2 BLAKE3 hashing was not performed, **Then** `last_checked_at` is NOT reset (so the 1-hour forced Tier-2 hash eventually fires), and when Tier 1 differs but Tier 2 BLAKE3 matches, `mtime_nsec` and `last_checked_at` are updated in `source_documents`.

---

### User Story 5 - Read-Only Store Opens, Atomic Hook Transactions & Project Root Discovery (Priority: P2)

Read-only CLI operations (`status`, `query`, `graph`, `doctor`, `verify`) do not issue unconditional SQLite `INSERT` writes on open; passive hooks do not create `.rgt/store.db` in uninitialized projects or subdirectories; and source document upserts occur inside the same SQLite transaction as node insertions.

**Why this priority**: Eliminates multi-process writer lock contention, `SQLITE_READONLY` failures, and stray `.rgt/store.db` directories across the filesystem when global hooks are installed.

**Independent Test**: Install global hooks, run a passive hook in a directory with no `.rgt/` ancestor, and confirm no `.rgt/` directory is created; run `rgt status` from a subdirectory of an initialized repo and confirm it uses the ancestor `.rgt/store.db`.

**Acceptance Scenarios**:

1. **Given** an existing database that already has `rdf_project_id` in `project_settings`, **When** `get_or_create_rdf_project_id` runs, **Then** it reads the existing value via `SELECT` first and only executes `INSERT` when the key is absent.
2. **Given** a passive hook invocation (`rgt hook post`), **When** no `.rgt/store.db` exists in the current directory or any ancestor directory, **Then** the hook exits `0` cleanly without creating `.rgt/store.db`.
3. **Given** an active CLI command or hook executed from a subdirectory of an initialized project, **When** resolving the store path, **Then** RGT walks up ancestor directories to locate the existing `.rgt/store.db` project root.
4. **Given** `handle_passive_hook_event`, `record_pdf_values`, or `execute_record`, **When** recording values for a file, **Then** `upsert_source_document`, prior-node invalidation, and `insert_tracked_node` execute inside a single SQLite transaction.

### Edge Cases

- File with mtime before 1970 or far in the future (`tier1.rs`): `mtime.checked_mul(1_000_000_000).and_then(|v| v.checked_add(mtime_nsec))` prevents `i64` overflow panic.
- Re-recording an identical file (same BLAKE3 hash): fast-path keeps existing root nodes untouched without marking anything stale.
- Large graphs (`InvalidationCascade::build_from_db`): load all nodes and all derivation edges in 2 bulk SQL queries instead of $2N + 1$ queries, and check DAG acyclicity once rather than running `is_cyclic_directed` on every edge insertion (`O(V + E)` instead of `O(E * (V + E))`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `upsert_source_document` and file recording MUST compare incoming root nodes against existing non-stale root nodes for that `source_doc_id` when `blake3_hash` changes, marking absent/modified root nodes `is_stale = 1` (`SOURCE_FILE_MODIFIED`) and cascading invalidation to their downstream derived subgraph within the same transaction.
- **FR-002**: `upsert_source_document` MUST re-key `tracked_nodes.source_doc_id` from any duplicate `(dev, ino)` `source_documents` rows to the surviving `source_documents.id` before deleting duplicate rows.
- **FR-003**: `SCHEMA_VERSION` MUST be bumped to `3` with an automatic migration in `stamp_or_migrate_schema` that upgrades v1/v2 `tracked_nodes` tables lacking `duration_secs` or the `'DURATION'` check constraint, and `user_version > SCHEMA_VERSION` MUST be rejected before `initialize_schema` executes DDL.
- **FR-004**: `evaluate_and_invalidate_all` MUST only update `last_checked_at` when Tier 2 hashing is executed, and MUST update stored `mtime_nsec`/`file_size`/`last_checked_at` when Tier 1 mismatches but Tier 2 BLAKE3 hash matches.
- **FR-005**: `query`, `derive`, `verify`, `graph`, and `gc` MUST run `InvalidationCascade::evaluate_and_invalidate_all` before reading graph state, and `delete_obsolete_nodes` MUST iterate until zero stale leaf nodes are removed.
- **FR-006**: `get_or_create_rdf_project_id` MUST `SELECT` before `INSERT`; passive hooks MUST NOT create `.rgt/store.db` in uninitialized repositories; and store discovery MUST walk up ancestor directories to find `.rgt/store.db`.

### Key Entities

- **`SourceDocument`**: Tracks canonical project-relative `file_path`, `(dev, ino)`, `mtime_nsec`, `file_size`, `blake3_hash`, and `last_checked_at` (last Tier-2 verification timestamp).
- **`TrackedNode`**: Root or derived provenance node linked to `source_doc_id` without accidental nullification on path alias deduplication.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of obsolete root nodes and downstream derivations are marked `STALE` when a modified file is re-read by a hook or re-recorded via `rgt record`.
- **SC-002**: Zero `tracked_nodes` rows have `source_doc_id` set to `NULL` after recording the same file via relative, `./`-prefixed, and absolute paths.
- **SC-003**: Upgrading a pre-043 v2 `.rgt/store.db` succeeds with zero data loss and supports `DURATION` derivations.
- **SC-004**: `InvalidationCascade::build_from_db` on a 5,000-node / 10,000-edge store completes in `< 50ms` using bulk queries.
