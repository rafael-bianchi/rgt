# Feature Specification: Remove MCP Server — CLI-Only Active Surface (RTK Alignment)

**Feature Branch**: `feat/013-remove-mcp-server`

**Created**: 2026-08-08

**Status**: Draft

**Input**: User description: "Remove the MCP server entirely. Replace the active integration surface with CLI commands invoked by hooks, following RTK's architecture (zero MCP, 100% hooks + rules files)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Users Query Provenance via CLI (Priority: P1)

A developer (or hook script) runs `rgt query <node_id>` to trace the derivation lineage of a value node. The command reads the database directly and outputs the lineage tree — no MCP server involved. The command behaves identically to the current MCP-backed `rgt query`.

**Why this priority**: `rgt query` currently delegates to the MCP handler. Without MCP, it needs a direct DB query path. This is the primary CLI command that must continue working.

**Independent Test**: Record a value node, run `rgt query <node_id>`, verify the output matches the current behavior.

**Acceptance Scenarios**:

1. **Given** a node exists in the database, **When** `rgt query <node_id>` is run, **Then** the derivation lineage is printed (text format) and matches the current output.
2. **Given** a node exists, **When** `rgt query <node_id> --json` is run, **Then** JSON lineage output is printed and matches the current output.
3. **Given** a nonexistent node ID, **When** `rgt query nonexistent` is run, **Then** a clear error is printed and exit code is non-zero.

---

### User Story 2 - MCP Server Is Fully Removed (Priority: P1)

The `rgt mcp` command, the `src/mcp/` module, and all MCP protocol code are deleted from the repository. No MCP tools, JSON-RPC handlers, or stdio server logic remain. The `src/lib.rs` no longer exports `pub mod mcp`.

**Why this priority**: This is the core change. Without this, the feature is incomplete. All MCP surface must be removed to align with RTK's zero-MCP architecture.

**Independent Test**: `grep -r "mcp\|MCP\|Model Context Protocol" src/` returns zero matches. `cargo run -- mcp` emits "unknown command" error.

**Acceptance Scenarios**:

1. **Given** the codebase, **When** `grep -r "mcp\|MCP\|Model Context Protocol" src/` is run, **Then** zero matches are returned.
2. **Given** a build of the new code, **When** `rgt mcp` is run, **Then** the command is unrecognized with a help message suggesting `rgt --help`.
3. **Given** `src/lib.rs`, **When** checked, **Then** `pub mod mcp` is not present.

---

### User Story 3 - Direct DB Query Module Replaces MCP Handlers (Priority: P2)

The `rgt query` and `rgt status` CLI commands use a new shared query module (`src/query/`) that reads the database directly, rather than delegating to MCP handlers. The query logic is extracted from the MCP handlers into reusable functions.

**Why this priority**: The MCP handlers contain BFS traversal logic that must be preserved. Extracting it ensures `rgt query` works identically after MCP removal.

**Independent Test**: Run existing integration tests (`test_derivation_chain.rs`) — they must produce identical results.

**Acceptance Scenarios**:

1. **Given** the extracted query logic, **When** `rgt query <node_id>` is run, **Then** the output matches the pre-removal output exactly.
2. **Given** the extracted query logic, **When** integration tests for derivation chains are run, **Then** all tests pass with identical assertions.

---

### User Story 4 - Documentation Reflects Zero-MCP Architecture (Priority: P2)

README, AGENTS.md, and CONTRIBUTING.md no longer mention MCP, "Model Context Protocol", or `rgt mcp`. The active integration surface is described as CLI commands invoked by hooks. The MCP tools table is replaced with CLI commands.

**Why this priority**: Documentation must match actual behavior per Constitution X. Mentioning a removed feature misleads users and violates the "docs match behavior" principle.

**Independent Test**: `grep -ri "mcp\|Model Context Protocol" README.md AGENTS.md CONTRIBUTING.md` returns zero matches.

**Acceptance Scenarios**:

1. **Given** the updated README, **When** the Integration section is read, **Then** no MCP references appear — hooks + CLI are described as the toolkit.
2. **Given** the updated AGENTS.md, **When** the CLI Commands table is read, **Then** `rgt mcp` is removed from the table.

---

### User Story 5 - Constitution Amended (Priority: P2)

Constitution v1.4.0 replaces Principle VI (Dual Passive/Active with MCP) with a CLI-only active surface principle. The "MCP Standards" policy is removed from Integration & Tooling Policy.

**Why this priority**: The constitution governs all specifications and implementations. It must reflect the current architecture to remain authoritative.

**Independent Test**: Read `.specify/memory/constitution.md` — Principle VI no longer mentions MCP.

**Acceptance Scenarios**:

1. **Given** the updated constitution, **When** Principle VI is read, **Then** it describes active CLI surface (not MCP).
2. **Given** the updated constitution, **When** Integration & Tooling Policy is read, **Then** "MCP Standards" is removed.

---

### Edge Cases

- What happens to the `record_derivation` / `record_value` / `query_provenance` / `list_stale_values` tools? They are MCP-only — not exposed via CLI. After removal, these functions are dead code. There are two options:
  a. **Pre-existing plan (spec 002)**: Extract them into CLI subcommands (`rgt record-value`, `rgt record-derivation`, `rgt list-stale-values`). This is out of scope for THIS spec but should be tracked in a follow-up.
  b. **Remove them entirely**: They were MCP-only, they become dead code. Remove them. (Recommended — simpler, faster, aligns with "remove MCP").
  → Decision: Remove dead code in this spec. Re-create CLI subcommands in a follow-up spec if needed (constitution already provides for this via Principle VIII — hooks call CLI subcommands).

- What happens to `src/cli/query.rs`? It currently calls `handle_query_provenance` (an MCP handler). It must be rewritten to use the new `src/query/` module directly.

- Integration tests that reference MCP (`test_mcp_contract.rs`) must be updated or removed. The schemas they validate (MCP tool input/output) no longer exist.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The `src/mcp/` module MUST be deleted in its entirety.
- **FR-002**: The `rgt mcp` subcommand MUST be removed from `src/main.rs`.
- **FR-003**: `pub mod mcp` MUST be removed from `src/lib.rs`.
- **FR-004**: The query logic (BFS traversal, lineage building) from `src/mcp/handlers.rs` MUST be extracted into a new `src/query/` module — reusable by CLI commands without MCP.
- **FR-005**: `rgt query` MUST call the new `src/query/` module directly (not MCP handlers).
- **FR-006**: All MCP-related test files and test functions MUST be removed or updated to use the new query module.
- **FR-007**: README, AGENTS.md, and CONTRIBUTING.md MUST remove all MCP references and replace with CLI-based integration description.
- **FR-008**: Constitution MUST be amended (v1.3.0 → v1.4.0): Principle VI replaced, "MCP Standards" policy removed.
- **FR-009**: All existing tests MUST pass after MCP removal — no regressions in graph, verification, hooks, or CLI commands.
- **FR-010**: Dead code from MCP handlers (`handle_record_value`, `handle_record_derivation`, `handle_list_stale_values`) MUST be removed. The `rgt verify` CLI already covers the verification path per spec 011.

### Key Entities

- **Query Module** (`src/query/`): New module extracted from MCP handlers. Contains `query_provenance()` (BFS traversal + lineage building) and `list_stale_nodes()` (stale node listing). Replaces the MCP handler functions.
- **MCP Server** (`src/mcp/`): Removed. The directory, all files, and all references to it are deleted.

### Files Affected

| File | Change |
|---|---|
| `src/mcp/mod.rs` | DELETE |
| `src/mcp/handlers.rs` | DELETE (logic extracted to `src/query/`) |
| `src/lib.rs` | Remove `pub mod mcp`; add `pub mod query` |
| `src/main.rs` | Remove `Mcp` variant from `Commands` enum; remove match arm; remove `mod mcp` |
| `src/cli/query.rs` | Rewrite to call `src/query/` instead of MCP handler |
| `src/query/mod.rs` | NEW — extracted BFS traversal + `query_provenance()` and `list_stale_nodes()` |
| `tests/contract/test_mcp_contract.rs` | DELETE (MCP protocol no longer exists) |
| `tests/integration/test_derivation_chain.rs` | UPDATE — replace MCP handler calls with query module |
| `README.md` | Remove MCP Server section; replace with CLI-based tools |
| `AGENTS.md` | Remove MCP Interface section; remove `rgt mcp` from CLI table |
| `CONTRIBUTING.md` | Remove MCP from project structure |
| `.specify/memory/constitution.md` | Amend Principle VI; remove "MCP Standards" policy |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `grep -r "mcp\|MCP\|Model Context Protocol" src/` returns zero matches in source code.
- **SC-002**: `grep -ri "mcp\|Model Context Protocol" README.md AGENTS.md CONTRIBUTING.md` returns zero matches.
- **SC-003**: `cargo run -- mcp` fails with "unrecognized command" or equivalent error message.
- **SC-004**: `rgt query <node_id>` produces identical output to the pre-removal version.
- **SC-005**: All existing tests pass (`cargo test --all -- --test-threads=1`) with zero failures.
- **SC-006**: The constitution v1.4.0 does not contain "MCP Standards" and Principle VI does not mention MCP.

## Assumptions

- The MCP server was the only active integration surface. Replacing it with CLI commands invoked by hooks follows RTK's proven architecture.
- The `record_derivation` and `record_value` tools were MCP-only. They are removed as dead code. Follow-up specs will re-create CLI equivalents if needed (tracked in spec roadmap).
- The `rgt verify` CLI (spec 011) already covers derivation verification — the MCP verification path in `handle_record_derivation` is redundant and can be removed.
- All integration tests that construct MCP JSON payloads will be rewritten to use the new query module directly (no JSON serialization/deserialization needed for CLI).
