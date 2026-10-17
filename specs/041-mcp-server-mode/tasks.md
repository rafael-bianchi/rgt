# Tasks: MCP Server Mode

**Input**: Design documents from `/specs/041-mcp-server-mode/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/mcp-tool-contract.md, quickstart.md

**Tests**: Included — Constitution IX requires protocol + exit-code coverage.

**Organization**: Two user stories; US1 (query_provenance prototype) de-risks US2 (full refactor + remaining tools).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1/US2)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependency and baseline.

- [ ] T001 Add `rmcp = "=3.1.4"` with features `server,macros,transport-io` to `[dependencies]` in `Cargo.toml`; register test target `test_mcp_contract` (path `tests/contract/test_mcp_contract.rs`); confirm `cargo build` succeeds and existing tests are green; record baseline release binary size (`ls -la target/release/rgt`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The MCP server skeleton + the `query_provenance` prototype (de-risking step).

**⚠️ CRITICAL**: No US2 work (the refactor) begins until the prototype compiles and proves the `rmcp` ergonomics.

- [ ] T002 Create `src/mcp/error.rs` with `pub enum VerifyDeriveError { InvalidInput(String), VerificationFailed(String), Internal(String) }` (+ `Display`/`From<String>`); create `src/mcp/mod.rs` with a minimal `RgtMcpServer` struct implementing `rmcp::ServerHandler` with a single `#[tool] async fn query_provenance(&self, node_id: String) -> Result<serde_json::Value, String>` delegating to `crate::query::query_provenance`; add `pub mod mcp;` to `src/lib.rs`; add `mod mcp;` to `src/main.rs`
- [ ] T003 Add the `Mcp` subcommand to `src/main.rs` (clap, no args) calling `mcp::serve_stdio(handler)` (or the rmcp equivalent); add a thin `src/cli/mcp.rs` entry if useful

**Checkpoint**: `cargo build` compiles with `rmcp`; `rgt mcp` starts and responds to `initialize` + `tools/list` + a `query_provenance` call (verified manually or by the contract test in US1).

---

## Phase 3: User Story 1 - An MCP Client Can Query Provenance (Priority: P1) 🎯 MVP

**Goal**: `query_provenance` works over the MCP stdio protocol (proves the SDK ergonomics with zero refactoring).

**Independent Test**: `cargo test --test test_mcp_contract` — initialize + tools/list + query_provenance.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T004 [US1] Contract test in `tests/contract/test_mcp_contract.rs`: spawn `rgt mcp`, write `initialize` + `tools/list` JSON-RPC messages over stdin, assert both succeed and tools/list names `query_provenance`; seed a store, call `query_provenance`, assert the returned lineage matches the seeded values; call `query_provenance` with an unknown id, assert a structured error result and that a follow-up `tools/list` still succeeds (server alive)

### Implementation for User Story 1

- [ ] T005 [US1] Wire the `Mcp` subcommand through `src/cli/mod.rs` (export) and `src/main.rs` dispatch if not done in T003

**Checkpoint**: `cargo test --test test_mcp_contract` green (query_provenance path).

---

## Phase 4: User Story 2 - Agents Can Record, Verify, and Derive via MCP (Priority: P1)

**Goal**: All 15 `process::exit` sites converted to typed `Err`; the remaining 4 tools exposed; CLI exit codes preserved.

**Independent Test**: `cargo test --test test_mcp_contract --test test_cli_record_derive_contract` plus `rg "process::exit" src/cli/derive.rs src/verify/mod.rs` returning nothing.

### Tests for User Story 2

> **NOTE: Write these tests FIRST.**

- [ ] T006 [US2] Add explicit exit-code assertions to `tests/contract/test_cli_record_derive_contract.rs` for EVERY refactored call site: unknown `--operation` (2), missing `--expression` (2), empty parents (2), >26 parents (2), missing parent node (2), verification failure (1) — for BOTH `rgt derive` and `rgt verify` (extend/verify the existing ones)
- [ ] T007 [US2] Extend `tests/contract/test_mcp_contract.rs`: `record_values` returns `RecordOutcome` and values appear in the graph; `verify_only` succeeds without writing on a correct derivation and errors on a wrong one; `derive_and_verify` verifies-then-records on success and errors on failure; invalid input to any tool returns an error AND the server survives (subsequent tools/list OK)

### Implementation for User Story 2

- [ ] T008 [US2] Refactor `src/verify/mod.rs`: split `run_verify_cli` into a pure `run_verify(...) -> Result<(), VerifyDeriveError>` (no `process::exit`, 7 sites → typed errors) + a thin CLI wrapper that translates `InvalidInput`→exit 2, `VerificationFailed`/`Internal`→exit 1, success→exit 0; `main.rs` Verify arm calls the wrapper
- [ ] T009 [US2] Refactor `src/cli/derive.rs`: `execute_derive` returns `Result<(), VerifyDeriveError>` (8 sites → typed errors, exit-2 for input, exit-1 for verification failure); `main.rs` Derive arm inspects the error to decide exit 2 vs 1 (matching today's direct-exit behavior)
- [ ] T010 [US2] Refactor `src/cli/record.rs`: introduce `RecordOutcome { count, skipped_limit, skipped_malformed }`, return it from a core function; `execute_record` (CLI wrapper) prints from the returned outcome (existing output text preserved)
- [ ] T011 [US2] Refactor `src/cli/status.rs`: extract the `json!({...})` summary (lines 57-62) into a returned value; `execute_status_with`/`execute_status` return it; the text branch unchanged
- [ ] T012 [US2] Add the 4 remaining `#[tool]` handlers to `src/mcp/mod.rs`: `get_status` (wraps the status core), `record_values`, `verify_only` (calls pure `run_verify`), `derive_and_verify` (calls refactored `execute_derive`) — each mapping `VerifyDeriveError` to an MCP error result

**Checkpoint**: `rg "process::exit" src/cli/derive.rs src/verify/mod.rs` is empty; MCP + CLI contract tests green.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Docs and final validation.

- [ ] T013 Document `rgt mcp` in `README.md` (an MCP section: what it is, how to configure Claude Code/Cursor/VS Code against it, the 5 tools) and add the command to the README Commands table (Constitution X)
- [ ] T014 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, full `cargo test`; measure and report the release binary size delta from `rmcp`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; the prototype BLOCKS US2 (de-risking).
- **US1 (Phase 3)**: Depends on Foundational (T002/T003).
- **US2 (Phase 4)**: Depends on US1 (prototype proven) + T002.
- **Polish (Phase 5)**: Depends on US2.

### User Story Dependencies

- **US1 (P1)**: query_provenance only — zero refactor.
- **US2 (P1)**: depends on US1's prototype proving the SDK; then the refactor + remaining tools.

### Within Each User Story

- Tests first (T004/T006/T007), then implementation (T005/T008-T012).

### Parallel Opportunities

- T008 (verify refactor) and T009 (derive refactor) touch different files; T010/T011 (record/status) are independent of both. T012 depends on T008/T009/T010/T011.

---

## Implementation Strategy

### De-risked delivery (per the deep spec Risks table)

1. Phase 1: Setup (T001)
2. Phase 2: Prototype (T002–T003) — `query_provenance` over stdio, ZERO refactoring
3. Phase 3: US1 (T004–T005) — prove the SDK ergonomics cheaply
4. Phase 4: US2 (T006–T012) — the process::exit refactor + remaining 4 tools
5. Phase 5: Polish (T013–T014)

**STOP and VALIDATE after Phase 3**: `cargo test --test test_mcp_contract` green before touching the refactor.

### Incremental Delivery

1. Prototype → MCP server + query_provenance
2. US1 → protocol contract proven
3. US2 → typed errors (no process::exit) + 4 more tools
4. Polish → docs + binary-size report

---

## Notes

- The 15 process::exit sites (8 derive + 7 verify) are the correctness blocker; convert ALL of them before exposing the remaining tools.
- CLI exit codes (2 invalid input / 1 verification failure) must be preserved exactly — asserted, not assumed.
- Server cwd = project root (`DbStore::open_in_project(".")`); document, don't change.
- `rmcp` pinned `=3.1.4`; SDK upgrades are their own reviewed change.
- HTTP transport, graph/gc/doctor/init/update tools, and hook replacement are out of scope.
