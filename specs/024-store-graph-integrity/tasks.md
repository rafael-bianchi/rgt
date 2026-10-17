# Tasks: Store & Graph Integrity

**Input**: Design documents from `/specs/024-store-graph-integrity/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/store/{db,schema,queries}.rs`, `src/graph/invalidation.rs`, `src/cli/status.rs`, `src/types/node.rs`, `src/hooks/mod.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates)

- [x] T001 Confirm `rusqlite` (bundled SQLite) and `petgraph` are already present in `Cargo.toml` (no new dependencies — the cycle guard is a SQL recursive CTE; see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The widened node-ID format (root + derived) that every story's tests depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `src/types/node.rs` `#[cfg(test)]`: `generate_root_id` and `generate_derived_id` each return a string at least 32 hex characters long (128 bits), and are deterministic

### Implementation for Foundational Primitives

- [x] T003 Change the ID truncation in `src/types/node.rs` from `&hash_hex[..12]` to `&hash_hex[..32]` in BOTH `generate_root_id` and `generate_derived_id` (FR-005)

**Checkpoint**: ID-length tests pass; all IDs are ≥ 32 hex

---

## Phase 3: User Story 1 - A Derivation Cycle Is Rejected At Insert Time (Priority: P1) 🎯 MVP

**Goal**: `insert_derivation_edge` rejects self-loops and any cycle-inducing edge at the DB write path, and `build_from_db` never silently drops an edge (FR-001/FR-002)

**Independent Test**: Unit/contract tests attempt self-loop and multi-node cycle inserts and assert rejection with nothing persisted

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T004 [P] [US1] Contract tests in `src/store/queries.rs` (or a store test module): inserting a self-loop (`parent == child`) returns `Err` and the table is unchanged; inserting an edge that closes a 2-node cycle (A→B exists, B→A attempted) returns `Err`; a multi-node cycle (A→B, B→C, C→A) is rejected; a non-cyclic edge still succeeds

### Implementation for User Story 1

- [x] T005 [US1] In `insert_derivation_edge` (`src/store/queries.rs`), reject when `parent_id == child_id`, and run a `WITH RECURSIVE` reachability query (is `parent` already reachable from `child`?) before the INSERT, returning `Err` if a cycle would form (FR-001)
- [x] T006 [US1] Add `CHECK (parent_node_id <> child_node_id)` to the `derivation_edges` `CREATE TABLE` in `src/store/schema.rs` (applies to new databases only — defense-in-depth; existing DBs are guarded by T005)
- [x] T007 [US1] In `InvalidationCascade::build_from_db` (`src/graph/invalidation.rs`), propagate (do not `let _ =`) the error from `add_derivation_edge` (FR-002)

**Checkpoint**: US1 tests pass; cycles are rejected at insert and never silently dropped on reload

---

## Phase 4: User Story 2 - Concurrent Access Never Mis-Reports Staleness (Priority: P1)

**Goal**: Every connection carries a non-zero `busy_timeout`, and `rgt status` propagates invalidation failures (FR-003/FR-004)

**Independent Test**: A unit test asserts the connection has a `busy_timeout`; a test asserts `execute_status` surfaces an invalidation failure

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T008 [P] [US2] Tests in `src/store/db.rs` (or a store test module): after `DbStore::open`/`open_in_memory`, the connection reports a non-zero busy timeout. For the `execute_status` failure path, add a test seam in `src/cli/status.rs` — extract the build-cascade-and-invalidate step into a helper that `execute_status` calls (e.g. `run_invalidation(db) -> Result<()>`), then unit-test that `execute_status` returns `Err` (non-zero) when that step fails, rather than printing "all fresh" (FR-004)

### Implementation for User Story 2

- [x] T009 [US2] In `DbStore::open` and `open_in_memory` (`src/store/db.rs`), call `conn.busy_timeout(std::time::Duration::from_secs(5))` after opening (FR-003)
- [x] T010 [US2] In `execute_status` (`src/cli/status.rs`), propagate the result of `evaluate_and_invalidate_all` (return `Err` instead of `let _ =`) (FR-004)

**Checkpoint**: US2 tests pass; concurrency-safe reads and honest status output

---

## Phase 5: User Story 3 - Node IDs Wide and Re-Records Update Values (Priority: P1)

**Goal**: Re-recording an existing node ID updates the value-bearing columns to the latest write (FR-006) — the ID-width half (FR-005) was completed in Foundational (T002/T003)

**Independent Test**: A contract test re-inserts the same ID with a different value/source and asserts the value columns changed

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T011 [P] [US3] Contract test in a store test module: insert a node, then re-insert the same `id` with a different `value` (and `source_doc_id`/`line_number`), and assert `number_val`/`date_val`/`duration_secs`/`source_doc_id`/`line_number` reflect the second write

### Implementation for User Story 3

- [x] T012 [US3] In `insert_tracked_node` (`src/store/queries.rs`), extend the `ON CONFLICT(id) DO UPDATE SET` clause to also set `number_val`, `date_val`, `duration_secs`, `source_doc_id`, `line_number` from `excluded.*` (FR-006)

**Checkpoint**: US3 tests pass; re-records update value columns (no silently-stale values)

---

## Phase 6: User Story 4 - Hook-Driven Inserts Are Transactional (Priority: P2)

**Goal**: The hook capture path commits its extracted nodes in a single transaction (FR-007)

**Independent Test**: A unit test verifies the hook insert path is one transaction (all-or-nothing on failure)

### Tests for User Story 4 (write FIRST, expect FAIL) ⚠️

- [x] T013 [P] [US4] Unit test in `src/hooks/mod.rs` `#[cfg(test)]`: a hook capture that fails partway through rolls back (no partial rows persisted); a successful capture commits all rows

### Implementation for User Story 4

- [x] T014 [US4] In `handle_passive_hook_event` (`src/hooks/mod.rs`), wrap the per-value `insert_tracked_node` loop in `conn.unchecked_transaction()`; commit on success, and on any error roll back and still return `Ok(())` (fail-open per constitution VIII) (FR-007)

**Checkpoint**: US4 tests pass; hook inserts are atomic and still non-blocking

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final validation

- [x] T015 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T016 Run the manual scenarios in `specs/024-store-graph-integrity/quickstart.md` (cycle rejection; ID width)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational — `src/store/queries.rs` + `schema.rs` + `invalidation.rs`
- **US2 (Phase 4)**: Depends on Foundational — `src/store/db.rs` + `src/cli/status.rs`
- **US3 (Phase 5)**: Depends on Foundational — `src/store/queries.rs` (shares the file with US1, run sequentially)
- **US4 (Phase 6)**: Depends on Foundational — `src/hooks/mod.rs`
- **Polish (Phase 7)**: Depends on all

### User Story Dependencies

- **US1 (P1)**: after Foundational — cycle guard
- **US2 (P1)**: after Foundational — busy_timeout + status
- **US3 (P1)**: after Foundational — value-column update (shares `queries.rs` with US1)
- **US4 (P2)**: after Foundational — hook transaction

### Within Each User Story

- Tests written first, must FAIL before implementation
- Foundational ID widening (T003) before the stories
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 (node.rs) ∥ T004 (queries/store) ∥ T008 (db/status) ∥ T011 (store) ∥ T013 (hooks) — all different files
- US1 and US2 are independent (different files); US1 and US3 both edit `src/store/queries.rs` → sequential

---

## Parallel Example: Foundational + Story Tests

```bash
Task: "ID-length tests (src/types/node.rs)"
Task: "cycle-guard tests (src/store/queries.rs)"
Task: "busy_timeout + status tests (src/store/db.rs, status test module)"
Task: "value-column update test (store test module)"
Task: "hook transaction test (src/hooks/mod.rs)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 + 2 (Foundational: ID widening)
2. Phase 3: User Story 1 (cycle guard at insert) — the §7.2 HIGH correctness fix
3. **STOP and VALIDATE**: cycles are rejected at the write path
4. Then US2/US3/US4 incrementally

### Incremental Delivery

1. Foundational → IDs widened
2. US1 → cycle/self-loop guard at insert (MVP)
3. US2 → busy_timeout + honest status
4. US3 → value-column re-record update
5. US4 → transactional hook inserts
6. Polish → fmt/clippy/full-suite + manual validation

---

## Notes

- [P] tasks = different files, no dependencies; US1/US3 both edit `src/store/queries.rs`, so their implementation tasks run sequentially
- [Story] label maps task to user story for traceability
- The self-loop `CHECK` (T006) applies only to new databases; existing DBs are guarded by the pre-insert CTE (T005) — no migration required here
- ID widening changes the format for NEW writes only; existing 48-bit IDs remain valid (re-keying deferred to spec 029)
- Commit after each task or logical group; stop at each checkpoint
