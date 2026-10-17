# Tasks: Store Maintenance

**Input**: Design documents from `/specs/029-store-maintenance/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/store-maintenance-invariants.md, quickstart.md

**Tests**: Included — spec "Independent Test" per story + Constitution IX require coverage.

**Organization**: Tasks grouped by user story (US1–US3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US3)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm baseline before any changes.

- [x] T001 Verify baseline: run `cargo build` and `cargo test --test test_store_integrity --test test_invalidation` and confirm they pass; confirm no new dependencies (`PRAGMA user_version` is native SQLite via `rusqlite`).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Schema-version framework every story builds on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T002 Add `SCHEMA_VERSION: i32 = 1` constant and a `stamp_or_migrate_schema(conn)` function (read `PRAGMA user_version`; `0` → stamp to 1; `> SCHEMA_VERSION` → error; `<` → run ordered migrations) in `src/store/schema.rs`
- [x] T003 Call the schema-version check from `DbStore::open` after `initialize_schema` in `src/store/db.rs`
- [x] T004 Register new test target `test_store_maintenance_contract` (path `tests/contract/test_store_maintenance_contract.rs`) in `Cargo.toml`

**Checkpoint**: Store open stamps/validates the schema version; existing tests still pass.

---

## Phase 3: User Story 1 - The Store Survives a Version-Skewed Binary (Priority: P1) 🎯 MVP

**Goal**: A store newer than the binary fails fast with a clear error; a supported forward migration preserves data.

**Independent Test**: `cargo test --test test_store_maintenance_contract` — asserts the version-skew error and the stamp.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T005 [US1] Unit tests in `tests/unit/store_integrity.rs` for `stamp_or_migrate_schema` (unversioned → stamped 1; version 1 → no-op; version 2+ → error naming the version and max)

### Implementation for User Story 1

- [x] T006 [US1] Wire the version-skew error into the CLI so commands surface it as exit code 1 with an actionable message (verify via `DbStore::open` mapping in `src/store/db.rs`), and add an end-to-end contract test in `tests/contract/test_store_maintenance_contract.rs` that initializes a store, bumps `PRAGMA user_version`, and asserts opening it errors

**Checkpoint**: `cargo test --test test_store_maintenance_contract` green.

---

## Phase 4: User Story 2 - Obsolete Nodes Can Be Collected (Priority: P2)

**Goal**: `rgt gc [--vacuum]` removes obsolete nodes transactionally and reports the count.

**Independent Test**: `cargo test --test test_store_maintenance_contract` — asserts removal of obsolete nodes, preservation of dependent/non-stale nodes, and the `--vacuum` flag.

### Tests for User Story 2

- [x] T007 [US2] Contract test in `tests/contract/test_store_maintenance_contract.rs` asserting `rgt gc` removes obsolete nodes (stale, no dependents), leaves dependent/non-stale nodes intact, reports the exact count, and `--vacuum` does not change the row-removal result

### Implementation for User Story 2

- [x] T008 [US2] Add `delete_stale_leaf_nodes(conn) -> usize` (DELETE stale nodes with no `derivation_edges` parent row) in `src/store/queries.rs`
- [x] T009 [US2] Implement `execute_gc(vacuum: bool)` (transactional delete, then `VACUUM` when flagged) in new `src/cli/gc.rs` and expose it via `src/cli/mod.rs`
- [x] T010 [US2] Wire the `Gc` subcommand (with `--vacuum` flag) in `src/main.rs`

**Checkpoint**: `rgt gc` works; `cargo test --test test_store_maintenance_contract` green.

---

## Phase 5: User Story 3 - Staleness Blast Radius Is Counted Once (Priority: P2)

**Goal**: `invalidate_file` reports distinct invalidated nodes (no double-count).

**Independent Test**: `cargo test --test test_invalidation` — asserts the shared-downstream node is counted once.

### Tests for User Story 3

- [x] T011 [US3] Unit test in `tests/unit/test_invalidation.rs` asserting that invalidating a file with two roots sharing one downstream node reports that node exactly once

### Implementation for User Story 3

- [x] T012 [US3] Deduplicate the invalidated node IDs via a `HashSet` and return `set.len()` in `src/graph/invalidation.rs` (`invalidate_file`)

**Checkpoint**: `cargo test --test test_invalidation` green.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation.

- [x] T013 Document `rgt gc` (and `--vacuum`) in `README.md` and `AGENTS.md` (Constitution X)
- [x] T014 Run quickstart.md scenarios, then `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; fix any regressions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS all user stories.
- **User Stories (Phase 3–5)**: Depend on Foundational.
- **Polish (Phase 6)**: Depends on all desired stories.

### User Story Dependencies

- **US1 (P1)**: after Foundational (T002/T003); no other-story deps.
- **US2 (P2)**: after Foundational; independent of US1 (needs the store opened, which T002/T003 provide).
- **US3 (P2)**: after Foundational; independent.

### Within Each User Story

- Tests written first (fail), then implementation.

### Parallel Opportunities

- US2 (T007–T010) and US3 (T011–T012) are independent and can proceed in parallel after Foundational.

### File-Coordination Notes

- `src/store/schema.rs` (T002) and `src/store/db.rs` (T003) are distinct files; T006 revisits `db.rs`.
- `tests/contract/test_store_maintenance_contract.rs` gains tests from US1 (T005 is unit, but contract assertions land in T006) and US2 (T007) — keep sequential.

---

## Parallel Example: US2 + US3 (after Foundational)

```bash
# GC path (touches src/store/queries.rs + src/cli/gc.rs + src/main.rs):
Task: "Contract test for rgt gc in tests/contract/test_store_maintenance_contract.rs"

# Dedup path (touches src/graph/invalidation.rs):
Task: "Unit test for distinct invalidated count in tests/unit/test_invalidation.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002–T004) — CRITICAL
3. Phase 3: US1 schema versioning (T005–T006)
4. **STOP and VALIDATE**: `cargo test --test test_store_maintenance_contract` — version skew is now a clean error

### Incremental Delivery

1. Foundational → store stamps schema version
2. US1 → version-skew error + migration framework
3. US2 → `rgt gc` (+ `--vacuum`)
4. US3 → distinct invalidated count
5. Polish → docs + fmt/clippy/full test

---

## Notes

- `SCHEMA_VERSION = 1` introduces no schema change; future changes bump it and append a migration.
- GC removes only obsolete nodes; a node with dependents or non-stale is never touched.
- US1/US2/US3 are largely independent after Foundational; commit after each phase.
