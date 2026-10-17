# Tasks: Remove MCP Server

**Input**: Design documents from `/specs/013-remove-mcp-server/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Integration tests must be updated to use extracted query module instead of MCP handlers. Contract test for MCP is deleted.

**Organization**: Tasks grouped by user story. US3 (extract query) is foundational — US1 and US2 depend on it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths

## Phase 1: Setup

**Purpose**: Verify current state. No initialization needed.

- [x] T001 List all MCP references in source: `grep -rn "mcp\|MCP\|Model Context Protocol" src/` — capture baseline
- [x] T002 [P] List all MCP references in docs: `grep -rin "mcp\|MCP\|Model Context Protocol" README.md AGENTS.md CONTRIBUTING.md` — capture baseline

---

## Phase 2: Foundational — Extract BFS Query Logic (US3, Priority: P2)

**Purpose**: Extract the BFS traversal and lineage-building logic from `src/mcp/handlers.rs` into a new `src/query/mod.rs`. This is the prerequisite for preventing code loss when `src/mcp/` is deleted.

**Independent Test**: `cargo test --lib` — the library still compiles (no broken imports yet; CLI not connected).

### Implementation

- [x] T003 [US3] Create `src/query/mod.rs` — copy `handle_query_provenance` logic from `src/mcp/handlers.rs:200-270`. Convert `params: &serde_json::Value` parameter to direct arguments (`node_id: &str`). Keep the BFS logic identical. Name the function `query_provenance(conn, node_id) -> Result<serde_json::Value, String>`
- [x] T004 [US3] Copy `handle_list_stale_values` logic from `src/mcp/handlers.rs:283-300` into `src/query/mod.rs` as `list_stale_nodes(conn) -> Result<serde_json::Value, String>`
- [x] T005 [US3] Register `pub mod query` in `src/lib.rs` (alongside existing `pub mod mcp` — will be removed later)

**Checkpoint**: `src/query/mod.rs` exists with two extracted functions. Library compiles.

---

## Phase 3: Rewrite CLI Query (US1, Priority: P1)

**Purpose**: `rgt query` now calls `src/query/` directly instead of MCP handler. Output format is preserved.

**Independent Test**: `cargo run -- query <existing_node_id>` produces identical output to pre-removal version.

### Implementation

- [x] T006 [US1] Rewrite `src/cli/query.rs` — replace `use crate::mcp::handlers::handle_query_provenance` + `json!({"node_id": node_id})` with a direct call to `crate::query::query_provenance(db.conn(), node_id)` that returns the JSON result
- [x] T007 [US1] Remove `extern crate serde_json;` from `src/cli/query.rs` if no longer needed (kept for `--json` pretty-print)
- [x] T008 [US1] Manual smoke test: record a node, run `rgt query <id>`, verify text and `--json` output match pre-removal behavior

**Checkpoint**: `rgt query` works without MCP.

---

## Phase 4: Delete MCP Module (US2, Priority: P1)

**Purpose**: Remove `src/mcp/`, `rgt mcp` subcommand, and all references.

**Independent Test**: `grep -rn "mcp\|MCP\|Model Context Protocol" src/` returns zero matches. `cargo run -- mcp` fails.

### Implementation

- [x] T009 [US2] Delete `src/mcp/mod.rs` (the JSON-RPC stdio server)
- [x] T010 [US2] Delete `src/mcp/handlers.rs` (BFS logic already extracted to `src/query/`)
- [x] T011 [US2] Remove `pub mod mcp` from `src/lib.rs`
- [x] T012 [US2] Remove `mod mcp` from `src/main.rs` (binary crate module declaration)
- [x] T013 [US2] Remove `Mcp` variant and match arm from `Commands` enum and main() in `src/main.rs`
- [x] T014 [US2] Verify `cargo build` succeeds — no broken imports or missing modules

**Checkpoint**: Zero MCP in source code. `rgt mcp` unrecognized.

---

## Phase 5: Update Integration Tests (US1 + US2)

**Purpose**: Fix tests that reference MCP. Delete MCP contract test.

**Independent Test**: `cargo test --all -- --test-threads=1` — all tests pass.

### Implementation

- [x] T015 Update `tests/integration/test_derivation_chain.rs` — replace MCP handler calls with `crate::query::query_provenance()` calls. Update import paths. Preserve assertions.
- [x] T016 [P] Delete `tests/contract/test_mcp_contract.rs` — MCP protocol no longer exists
- [x] T017 [P] Update any other tests referencing `rgt::mcp::`, `crate::mcp::`, or MCP handler functions — check compilation errors after T015-T016

**Checkpoint**: All tests pass without MCP references.

---

## Phase 6: Documentation (US4, Priority: P2)

**Purpose**: Remove all MCP mentions from docs. Replace integration surface description.

**Independent Test**: `grep -ri "mcp\|Model Context Protocol" README.md AGENTS.md CONTRIBUTING.md` returns zero matches.

### Implementation

- [x] T018 [US4] Update `README.md` — remove MCP Server section; describe active integration as CLI commands invoked by hooks
- [x] T019 [P] [US4] Update `AGENTS.md` — remove MCP Interface section; remove `rgt mcp` from CLI Commands table
- [x] T020 [P] [US4] Update `CONTRIBUTING.md` — remove MCP from project structure tree

**Checkpoint**: Zero MCP in documentation.

---

## Phase 7: Constitution (US5, Priority: P2)

**Purpose**: Amend v1.3.0 → v1.4.0. Replace Principle VI text. Remove "MCP Standards" policy.

**Independent Test**: `grep "MCP" .specify/memory/constitution.md` returns zero matches.

### Implementation

- [x] T021 [US5] Amend `.specify/memory/constitution.md` — replace Principle VI text with CLI-only version (from research.md Decision 4). Remove "MCP Standards" from Integration & Tooling Policy. Bump version to 1.4.0.

**Checkpoint**: Constitution reflects zero-MCP architecture.

---

## Phase 8: Polish

**Purpose**: Full verification.

- [x] T022 Run `cargo fmt --all -- --check` to verify formatting
- [x] T023 Run `cargo clippy --all-targets` to verify no errors
- [x] T024 Run `cargo test --all -- --test-threads=1` to verify all tests pass
- [x] T025 Run `cargo build` to verify clean build
- [x] T026 Run quickstart.md validation scenarios — confirm all 6 pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **US3 Extract Query (Phase 2)**: No dependencies — start immediately
- **US1 Rewrite CLI (Phase 3)**: Depends on US3 (extracted module exists)
- **US2 Delete MCP (Phase 4)**: Depends on US1 (CLI no longer imports MCP handlers)
- **US1+US2 Tests (Phase 5)**: Depends on US2 (MCP deleted, need to update imports)
- **US4 Docs (Phase 6)**: Independent — can run in parallel with Phases 3-5
- **US5 Constitution (Phase 7)**: Independent — can run in parallel with everything
- **Polish (Phase 8)**: Depends on all phases

### Parallel Opportunities

- **Phase 1**: T001 and T002 in parallel
- **Phase 5**: T015, T016, T017 in parallel (T016-T017 are independent; T015 depends on Phase 4)
- **Phase 6**: T018, T019, T020 in parallel (different files)
- **Phase 6 + Phase 7**: Can run in parallel (docs + constitution)

---

## Implementation Strategy

### MVP First (Query Works, MCP Exists)

1. Complete Phase 2: Extract BFS query
2. Complete Phase 3: Rewrite CLI query
3. **STOP and VALIDATE**: `rgt query` works from extracted module. MCP still exists.

### Incremental Delivery

1. Setup → baseline captured
2. Extract query → `src/query/mod.rs` exists
3. Rewrite CLI → `rgt query` uses extracted module
4. Delete MCP → `src/mcp/` gone, `rgt mcp` gone
5. Fix tests → all tests pass
6. Docs → zero MCP in README/AGENTS/CONTRIBUTING
7. Constitution → v1.4.0

---

## Notes

- The BFS extraction (T003-T004) must preserve the exact logic — copy-paste from `handlers.rs`, only change the function signature.
- `handle_record_value` and `handle_record_derivation` are not extracted — they are MCP-only and removed entirely.
- Integration tests that construct MCP JSON payloads get rewritten to call `src/query/` directly (no JSON serialization needed).
- The `rgt status` command uses `src/graph/invalidation.rs` already — no changes needed.
- Net code deletion expected: more lines removed than added.
