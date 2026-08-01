# Tasks: Fix Provenance Query Traversal Depth

**Input**: Design documents from `/specs/005-fix-provenance-query-depth/`

**Prerequisites**: plan.md, spec.md (3 user stories: P1 full lineage, P2 cycle guard, P3 performance), research.md, data-model.md, contracts/mcp-query-provenance.md, quickstart.md

**Tests**: Included — each user story defines specific testable scenarios in spec.md acceptance criteria. Contract test for schema backwards compatibility also included.

**Organization**: Tasks are grouped by user story to enable independent testing of each story. The core BFS implementation (Phase 1) is shared across all stories; story-specific tests validate each concern independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Core BFS Implementation (Shared Foundation)

**Purpose**: Replace the single-level parent loop in `handle_query_provenance` with a full BFS ancestor traversal. This implementation serves all three user stories (US1 full lineage, US2 cycle guard, US3 performance).

**⚠️ CRITICAL**: This phase MUST complete before any user story validation can begin. All story-specific tests depend on the BFS being functional.

### Test-First (Write FAILING tests before implementation)

- [x] T001 [P] Update `test_multi_level_derivation_chain_query` in `tests/integration/test_derivation_chain.rs` to assert at least 3 lineage steps (node_C + node_B + node_A) for a 3-level chain, replacing the current `>= 2` assertion
- [x] T002 [P] Update MCP contract test in `tests/contract/test_mcp_contract.rs` to validate `total_ancestors` field present in `query_provenance` response and `node_type` field present in each lineage step

### Implementation

- [x] T003 Replace single-level parent loop in `handle_query_provenance` in `src/mcp/handlers.rs` with BFS traversal using `std::collections::VecDeque` queue and `std::collections::HashSet` visited set
- [x] T004 Add `total_ancestors` field (unique visited count minus 1 for queried node) to response object in `src/mcp/handlers.rs:handle_query_provenance`
- [x] T005 Add `node_type` field to each lineage step entry in `src/mcp/handlers.rs:handle_query_provenance` (value: `format!("{:?}", node.node_type)` or equivalent)
- [x] T006 Expand `stale_reason` and `parent_ids` fields to all lineage steps (not just the queried node) in `src/mcp/handlers.rs:handle_query_provenance`
- [x] T007 Handle dangling edges: skip `get_tracked_node` returning `None` without failing the BFS traversal in `src/mcp/handlers.rs:handle_query_provenance`

**Checkpoint**: BFS traversal functional. Failing tests from T001/T002 should now pass. All three user stories can now be validated.

---

## Phase 2: User Story 1 - Full Lineage Query for Deep Derivation Chains (Priority: P1) 🎯 MVP

**Goal**: An AI coding agent queries provenance for any node and receives the complete derivation lineage back to root source files, not just immediate parents.

**Independent Test**: Set up a 3-level derivation chain (root → derived1 → derived2), query the deepest node, verify all four nodes appear with correct `total_ancestors` count.

### Tests for User Story 1

- [x] T008 [P] [US1] Add root-node query test in `tests/integration/test_derivation_chain.rs`: single root node, verify `lineage_steps` length = 1, `total_ancestors` = 0, `parent_ids` = [], `node_type` = "Root", and wall-clock time <1ms
- [x] T009 [P] [US1] Add branching chain query test in `tests/integration/test_derivation_chain.rs`: derived node with two independent parent chains, verify all ancestors from both chains appear without duplication

### Validation for User Story 1

- [x] T010 [US1] Run `cargo test test_multi_level_derivation_chain_query` and verify it passes with exact step count assertion (not just ≥2)
- [x] T011 [US1] Run `cargo test test_root_node_query` and verify root node returns single step with `total_ancestors: 0`
- [x] T012 [US1] Run `cargo test test_branching_chain` and verify all ancestors from both branches appear once each
- [x] T012b [P] [US1] Verify determinism in `tests/integration/test_derivation_chain.rs`: call `query_provenance` twice on the same 3-level chain node, assert identical `lineage_steps` ordering, `total_ancestors` value, and `node_type` fields across both responses

**Checkpoint**: User Story 1 fully functional — complete lineage for chains of any depth. MVP achieved.

---

## Phase 3: User Story 2 - Resilience Against Graph Cycles (Priority: P2)

**Goal**: If the provenance graph contains an unexpected cycle (due to bug or data corruption), the query handler terminates gracefully without entering an infinite loop.

**Independent Test**: Manually insert a self-referencing edge (A→A) in the database, query node A, and verify the handler returns successfully with the node listed (once), not entering an infinite loop.

### Tests for User Story 2

- [x] T013 [P] [US2] Add cycle detection test (self-loop: A→A) in `tests/integration/test_derivation_chain.rs`: directly insert a derivation edge where `parent_node_id == child_node_id`, query the node, assert handler returns without hanging and node appears exactly once
- [x] T014 [P] [US2] Add cross-node cycle test (A→B→A) in `tests/integration/test_derivation_chain.rs`: insert edges forming A→B and B→A, query node B, assert both nodes appear and handler returns without infinite loop
- [x] T015 [P] [US2] Add diamond dependency test (A→B→D and A→C→D) in `tests/integration/test_derivation_chain.rs`: verify node D returns all four nodes without duplication

### Validation for User Story 2

- [x] T016 [US2] Run `cargo test test_cycle` and verify all cycle tests pass (handler returns, no hangs)

**Checkpoint**: User Story 2 fully functional — cycle guard prevents infinite loops in unexpected graph states.

---

## Phase 4: User Story 3 - Performance Under Large Graphs (Priority: P3)

**Goal**: Agents can query provenance on derivation chains up to 1,000 nodes deep without noticeable delay, maintaining RGT's sub-second interactive response guarantees.

**Independent Test**: Generate a synthetic linear chain of 1,000 nodes and measure the total query wall-clock time.

### Tests for User Story 3

- [x] T017 [P] [US3] Add deep-chain performance test in `tests/integration/test_derivation_chain.rs`: create a linear chain of 100 nodes via repeated `record_derivation`, query the deepest node, assert wall-clock time <200ms and lineage_steps length = 101
- [x] T018 [P] [US3] Add shallow branching performance test in `tests/integration/test_derivation_chain.rs`: create a graph with 8,000 total nodes (shallow, branching structure), query a node with 5 ancestors, assert wall-clock time <200ms

### Validation for User Story 3

- [x] T019 [US3] Run `cargo test test_deep_linear_chain` and verify <200ms traversal time
- [x] T020 [US3] Run `cargo test test_shallow_branching_performance` and verify <200ms for shallow queries in large graphs

**Checkpoint**: User Story 3 fully functional — BFS traversal meets latency budget for all expected graph sizes.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, contract compliance, and cleanup.

- [x] T021 Run `cargo build` to verify zero compiler warnings in the BFS implementation
- [x] T022 Run full test suite `cargo test` to confirm no regressions in existing tests (hooks, invalidation, distribution, installer)
- [x] T023 [P] Verify quickstart.md Scenario 1 (multi-level chain) manually or via automated validation
- [x] T024 [P] Verify backward compatibility: existing `query_provenance` consumers that ignore unknown JSON fields still function correctly (validate via contract test from T002)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Core BFS)**: No prerequisites — project already initialized. Must complete before all subsequent phases.
- **Phase 2 (US1 - Full Lineage)**: Depends on Phase 1 completion. Core MVP.
- **Phase 3 (US2 - Cycle Guard)**: Depends on Phase 1 completion. Can run in parallel with Phase 2 tests.
- **Phase 4 (US3 - Performance)**: Depends on Phase 1 completion. Can run in parallel with Phase 2 and Phase 3.
- **Phase 5 (Polish)**: Depends on Phase 2+3+4 completion.

### User Story Dependencies

- **User Story 1 (P1)**: Depends only on Phase 1. No dependencies on other stories.
- **User Story 2 (P2)**: Depends only on Phase 1. Independently testable (cycle tests use isolated DB). No dependencies on US1.
- **User Story 3 (P3)**: Depends only on Phase 1. Independently testable (perf tests use isolated DB). No dependencies on US1 or US2.

### Within Each User Story

- Phase 1: Test-first tasks (T001, T002) can run in parallel before implementation → Implementation (T003-T007)
- Phase 2: Test tasks (T008, T009, T012b) can run in parallel → Validation tasks (T010-T012)
- Phase 3: Test tasks (T013, T014, T015) can run in parallel → Validation (T016)
- Phase 4: Test tasks (T017, T018) can run in parallel → Validation (T019, T020)
- Phase 5: T021-T024 all [P], can run in parallel

### Parallel Opportunities

```text
Phase 1 - Parallel tests:
  Task: T001 (update derivation chain test)
  Task: T002 (update MCP contract test)

Phase 2 - Parallel user story tests:
  Task: T008 (root node test with timing)
  Task: T009 (branching chain test)
  Task: T012b (determinism validation)

Phase 3 - Parallel user story tests:
  Task: T013 (self-loop test)
  Task: T014 (cross-node cycle test)
  Task: T015 (diamond test)

Phase 4 - Parallel user story tests:
  Task: T017 (1000-node perf test)
  Task: T018 (shallow branching perf test)

Phase 5 - All Polish tasks:
  Task: T023 (quickstart validation)
  Task: T024 (backward compatibility check)
```

---

## Parallel Example: Phase 1 (Core BFS)

```bash
# Launch test-first tasks together:
Task: "Update test_multi_level_derivation_chain_query in tests/integration/test_derivation_chain.rs"
Task: "Update MCP contract test in tests/contract/test_mcp_contract.rs"

# After tests are written (and FAIL), implement the BFS:
Task: "Replace single-level loop with BFS traversal in src/mcp/handlers.rs"
Task: "Add total_ancestors to response in src/mcp/handlers.rs"
Task: "Add node_type to each lineage step in src/mcp/handlers.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Core BFS Implementation (T001-T007)
2. Complete Phase 2: User Story 1 validation (T008-T012)
3. **STOP and VALIDATE**: `cargo test test_multi_level_derivation_chain_query` passes with full chain
4. Core bug is fixed — provenance queries return complete lineage

### Incremental Delivery

1. Complete Phase 1 → BFS functional, tests passing
2. Complete Phase 2 → Full lineage verified (MVP!)
3. Complete Phase 3 → Cycle resilience verified
4. Complete Phase 4 → Performance verified
5. Complete Phase 5 → Polish and full test suite green

### Single Developer Strategy

Since all implementation is in `src/mcp/handlers.rs` (shared across all stories), a single developer should:
1. Phase 1: Write tests (T001, T002) → Implement BFS (T003-T007) → Verify tests pass
2. Phase 2-4: Write story-specific tests and validate (all tests are additive in `tests/integration/test_derivation_chain.rs`)
3. Phase 5: Run full suite and cleanup

---

## Notes

- The core implementation (T003-T007) is ~25 lines of code replacing ~10 — everything else is test coverage
- Zero new crate dependencies — `VecDeque` and `HashSet` from `std::collections`
- No schema migrations — existing SQLite tables are read-only for BFS traversal
- The `visited` set serves both US1 (diamond dedup) and US2 (cycle guard) — implemented once in T003
- All story-specific tests in `tests/integration/test_derivation_chain.rs` can be reviewed and run independently
- Run `cargo test` after each phase to catch regressions early
