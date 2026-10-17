# Tasks: Store Staleness and Schema Integrity

**Input**: Feature specification `specs/045-store-staleness-and-schema-integrity/spec.md` and plan `specs/045-store-staleness-and-schema-integrity/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US5] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US3] Add unit test in `tests/unit/store_integrity.rs` verifying that opening an existing v2 database without `duration_secs` cleanly migrates to v3, allows `DURATION` node insertion, and opening `user_version > 3` errors cleanly before DDL execution.
- [x] T002 [US2] Add unit test in `tests/unit/store_integrity.rs` verifying that recording the same file via relative, `./`-prefixed, and absolute paths re-keys `tracked_nodes.source_doc_id` to the surviving row and leaves zero `NULL` `source_doc_id`s.
- [x] T003 [US1] Add unit test in `tests/unit/test_invalidation.rs` verifying that editing a file and re-recording it marks changed/removed root nodes as `is_stale = true` (`SOURCE_FILE_MODIFIED`), cascades to derived children, and keeps identical unchanged values fresh.
- [x] T004 [US4] Add contract test in `tests/contract/test_store_maintenance_contract.rs` verifying that `rgt gc` on a multi-tier stale chain (`root -> d1 -> d2 -> d3`) deletes all levels in a single invocation.
- [x] T005 [US5] Add unit test in `tests/unit/paths.rs` or `tests/contract/test_hooks_contract.rs` verifying that passive hooks run in uninitialized projects exit `0` without creating `.rgt/store.db`, and running commands from subdirectories discovers the parent `.rgt/store.db`.

---

## Phase 2: Implementation

- [x] T006 [US3] In `src/store/schema.rs`, bump `SCHEMA_VERSION = 3`, validate `user_version <= SCHEMA_VERSION` before DDL, and implement v2→v3 migration in `stamp_or_migrate_schema` adding `duration_secs INTEGER` and updating `CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION'))`.
- [x] T007 [US5] In `src/store/db.rs`, update `DbStore::open_in_project` to walk up parent directories to find existing `.rgt/store.db`; update `get_or_create_rdf_project_id` to `SELECT` before `INSERT` to avoid write locks on read paths.
- [x] T008 [US2] In `src/store/queries.rs`, update `upsert_source_document` to re-key `tracked_nodes.source_doc_id = surviving_id` from duplicate `(dev, ino)` rows before deleting duplicates.
- [x] T009 [US1] In `src/store/queries.rs` and `src/cli/record.rs`, implement atomic re-record invalidation: when a file's hash changes, compare incoming root nodes with existing active root nodes for that document, mark removed/modified nodes `is_stale = 1` (`SOURCE_FILE_MODIFIED`), cascade staleness to derived descendants, insert new nodes, and update document hash within a single SQLite transaction.
- [x] T010 [US4] In `src/detection/tier1.rs`, add safe checked timestamp math; in `src/detection/mod.rs` and `src/graph/invalidation.rs`, preserve `last_checked_at` unless Tier 2 hashing was performed, and update `mtime_nsec` when Tier 2 confirms file hash matches.
- [x] T011 [US4] In `src/graph/engine.rs`, optimize `InvalidationCascade::build_from_db` to load nodes and edges using 2 bulk queries and run DAG acyclicity check once (`petgraph::algo::is_cyclic_directed`).
- [x] T012 [US4] In `src/cli/gc.rs`, make `execute_gc` loop `delete_obsolete_nodes` until 0 nodes are removed, and clean up orphaned `source_documents` rows for deleted files.
- [x] T013 [US4] Wire `InvalidationCascade::evaluate_and_invalidate_all` into `src/cli/query.rs`, `src/cli/derive.rs`, `src/cli/graph.rs`, and `src/verify/mod.rs` before reading graph state.
- [x] T014 [US5] In `src/hooks/mod.rs`, ensure passive hooks check for `.rgt/store.db` before opening and exit 0 without auto-creating `.rgt/` in uninitialized directories.

---

## Phase 3: Verification & Parity

- [x] T015 Run the complete test suite (`cargo test </dev/null`) and verify all 45+ test targets pass.
- [x] T016 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
