# Feature Specification: MCP Server Mode

**Feature Branch**: `feat/041-mcp-server-mode`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Expose `record`/`derive`/`verify`/`query`/`status` as Model Context Protocol tools via `rmcp` (the official Rust SDK). Directly targets the undocumented-per-agent-payload fragility documented across the hooks findings; one documented integration surface instead of 13 fragile, partially-guessed ones."

Source context: `docs/spec-041-mcp-server-mode.md` (the dedicated deep spec — read in full). Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #8, P1.

## Clarifications

### Session 2026-08-31

- Q: The deep spec's Open Questions §7.3 asks whether `derive_and_verify` should be one combined tool or two separate tools (`verify_only` vs. `derive`). → A: Preserve the CLI's existing distinction as two separate MCP tools: `verify_only` (no side effects — reads the graph, never writes) and `derive_and_verify` (verifies-then-records). The CLI exposes `rgt verify` vs `rgt derive` with meaningfully different side effects; the MCP surface mirrors that, so an agent must actively choose to write.
- Q: The deep spec's Risks table mandates a de-risking prototype: build the zero-refactor `query_provenance` tool first, before touching the `execute_derive`/`run_verify_cli` refactor. → A: Follow that sequencing strictly. The `rgt mcp` subcommand ships with `query_provenance` first (validates the `rmcp` macro ergonomics cheaply); the `process::exit` refactor and the remaining 4 tools land in the same feature but after the prototype compiles and the SDK integration is proven.
- Q: Where does the MCP server's working directory come from (deep spec Open Question §7.2)? → A: Adopt the same assumption every existing hook integration makes — the process cwd is the project root (`DbStore::open_in_project(".")` unchanged). Document this in the spec; it is confirmed for the CLI's own invocation and is the standard MCP-client launch model, but a live client-config check is deferred to implementation verification (not a blocker).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An MCP Client Can Query Provenance (Priority: P1)

A developer configures their MCP-capable coding tool (Claude Code, Cursor, VS Code, ChatGPT, ...) with `rgt mcp` as an stdio MCP server. The agent can call a `query_provenance` tool to trace any tracked value's lineage, getting the same JSON `query_provenance` already produces — without any per-agent hook glue, and over a documented, schema-bound protocol.

**Why this priority**: P1 per the plan — the single highest-leverage feature identified in the growth research: one documented integration surface instead of 13 fragile, partially-guessed ones, riding MCP's now-dominant adoption.

**Independent Test**: `cargo test --test test_mcp_contract` — starts the `rgt mcp` server in-process (or via the binary) with stdio, exchanges an `initialize` handshake and a `tools/list`, then calls `query_provenance` against a seeded store and asserts the returned JSON lineage.

**Acceptance Scenarios**:

1. **Given** a project with a recorded node and a derived child, **When** an MCP client sends an `initialize` + `tools/list` + `tools/call query_provenance(node_id)`, **Then** the server responds with the node's full lineage JSON (node_id, is_stale, total_ancestors, lineage_steps) matching `rgt query <id> --json`.
2. **Given** an unknown node ID, **When** `query_provenance` is called, **Then** the tool returns a structured error the client can surface (no server crash, no process exit).
3. **Given** the server is invoked with no subcommand args, **When** stdio carries a valid MCP message, **Then** the server handles it and stays alive for the next message (a long-lived process, not a one-shot CLI).

---

### User Story 2 - Agents Can Record, Verify, and Derive via MCP (Priority: P1)

The full tool surface is available: `record_values` (extract values from file/content), `verify_only` (check a derivation without writing), `derive_and_verify` (verify-then-record), and `get_status` (graph state). Because the underlying functions are refactored to return structured results instead of calling `process::exit`, any invalid input (unknown operation, missing expression, too many parents, missing parent node) returns as an error to the client — never terminating the long-lived server.

**Why this priority**: P1 — this is the refactor that makes an MCP server safe to run at all; `execute_derive`/`run_verify_cli` currently call `process::exit` directly (8+7 call sites), which would crash the server on the first invalid input.

**Independent Test**: `cargo test --test test_mcp_contract` calls `derive_and_verify` with an unknown operation and asserts the server returns an error AND stays alive (a second `tools/list` still succeeds). The CLI exit-code behavior is preserved (exit 2 for invalid input, exit 1 for verification failure) and asserted by the existing contract tests plus new explicit exit-code assertions.

**Acceptance Scenarios**:

1. **Given** a seeded store, **When** `record_values` is called with file content, **Then** the tool returns a structured outcome (count, skipped counts) and the values appear in the graph.
2. **Given** two parent nodes, **When** `verify_only` is called with a correct derivation, **Then** it succeeds and records nothing; with a wrong result it fails (verification-failed error) and records nothing.
3. **Given** the same parents, **When** `derive_and_verify` is called with a correct result, **Then** it verifies and records a derived node; with a wrong result it returns an error and records nothing.
4. **Given** invalid input (unknown `--operation`), **When** any verify/derive tool is called, **Then** the server returns a typed error and remains alive for subsequent calls.
5. **Given** the CLI, **When** `rgt derive`/`rgt verify` are run with the same invalid inputs, **Then** the exit codes are unchanged (2 for invalid input, 1 for verification failure) — the refactor is behavior-preserving for the CLI.

---

### Edge Cases

- **Server startup with no project**: `DbStore::open_in_project(".")` fails → the tool returns a clear "store not initialized" error to the client (fail gracefully, no crash).
- **Multiple sequential calls**: the server must not exit after the first call (the current `process::exit` behavior is the exact bug being fixed).
- **Malformed MCP message**: the `rmcp` transport returns a protocol-level error; the server must not panic.
- **Tool-name/arity mismatch**: the `#[tool]` macro's exact ergonomics are unvalidated against a compiling prototype — the spec treats the deep spec's §3.2 shape as illustrative until T002 proves it.
- **Binary size**: `rmcp` (server+macros+transport-io) adds dependency weight — measured and reported (deep spec Risks; the plan requires it).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A new `rgt mcp` subcommand starts a long-lived stdio MCP server (`rmcp`, pinned 3.1.4) exposing the tools in §3.1 of the deep spec: `query_provenance`, `get_status`, `record_values`, `verify_only`, `derive_and_verify`.
- **FR-002**: `execute_derive` (8 call sites) and `run_verify_cli` (7 call sites) MUST be refactored so no `process::exit` remains in the logic: validation failures become typed `Err` returns; the CLI wrappers translate them back to the exact current exit codes (2 invalid input, 1 verification failed).
- **FR-003**: A typed error (e.g. `VerifyDeriveError { InvalidInput, VerificationFailed, Internal }`) MUST separate "bad input" from "verification failed" so the MCP tool surface and the CLI exit codes both stay correct.
- **FR-004**: `get_status` MUST return the existing `json!({...})` summary as a value (not just `println!` it); `record_values` MUST return a structured outcome (count, skipped_limit, skipped_malformed) instead of printing-only.
- **FR-005**: The MCP server MUST be resilient: any tool error returns to the client; the process NEVER exits from tool logic.
- **FR-006**: CLI exit-code behavior MUST be preserved byte-for-byte (existing contract tests + new explicit exit-code assertions).
- **FR-007**: `query_provenance` is exposed with zero refactoring (directly callable), shipped first per the de-risking sequence.

### Key Entities *(include if feature involves data)*

- **McpTool**: one of the five exposed operations, mapped 1:1 to a refactored pure function.
- **VerifyDeriveError**: the typed error separating invalid input (exit 2) from verification failure (exit 1).
- **RecordOutcome**: the structured `{count, skipped_limit, skipped_malformed}` result of `record_values`.
- **StatusSummary**: the existing `json!({...})` graph-state summary, returned as a value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo test --test test_mcp_contract` passes: initialize handshake, tools/list shows the 5 tools, `query_provenance` returns correct lineage, a verify/derive tool with invalid input returns an error and the server stays alive.
- **SC-002**: All 15 `process::exit` call sites (8 derive + 7 verify) are converted; `rg "process::exit" src/cli/derive.rs src/verify/mod.rs` returns nothing.
- **SC-003**: The existing `test_cli_record_derive_contract` suite (exit-code assertions) stays green, plus new explicit exit-code assertions for every refactored call site.
- **SC-004**: Binary size delta from `rmcp` is measured and reported (release build), not assumed.
- **SC-005**: `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test` pass.

## Assumptions

- `rmcp` 3.1.4 (pinned) with `server`+`macros`+`transport-io` features is the SDK; HTTP transport is out of scope for the initial version.
- The server cwd is the project root (`DbStore::open_in_project(".")`), matching every existing hook integration's assumption.
- `rgt graph`/`gc`/`doctor`/`init`/`update` are NOT exposed as MCP tools (operator/maintenance actions; graph needs a larger refactor) — per the deep spec's Non-Goals.
- The 13 existing hook/plugin integrations keep working; MCP mode is additive (a 14th integration surface).
- `#[tool]` macro ergonomics follow the deep spec §3.2's documented pattern; validated by the prototype before the refactor.

## Out of scope

- HTTP/SSE transport (`transport-streamable-http-server`) — stdio only.
- Exposing `graph`/`gc`/`doctor`/`init`/`update` as MCP tools.
- Replacing or deprecating any of the 13 existing hook/plugin integrations.
- A rewrite of the clap dispatch layer (`main.rs`'s arg parsing is untouched).
