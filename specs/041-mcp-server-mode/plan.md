# Implementation Plan: MCP Server Mode

**Branch**: `feat/041-mcp-server-mode` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/041-mcp-server-mode/spec.md`

## Summary

Add an `rgt mcp` subcommand that runs a long-lived stdio MCP server (`rmcp` 3.1.4, pinned) exposing 5 tools: `query_provenance` (zero-refactor, ships first as the de-risking prototype), `get_status`, `record_values`, `verify_only`, and `derive_and_verify`. The enabling refactor converts all 15 `process::exit` call sites (8 in `execute_derive`, 7 in `run_verify_cli`) into typed `Err` returns with a `VerifyDeriveError` (InvalidInput → exit 2, VerificationFailed → exit 1), so no tool error can terminate the server. CLI exit-code behavior is preserved and asserted. Additive: the 13 existing hook integrations keep working.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021).

**Primary Dependencies**: `rmcp` (pinned `=3.1.4`, features `server,macros,transport-io`) — confirmed on crates.io; its default features include `server` and `macros`. No HTTP transport (out of scope).

**Storage**: SQLite via the existing `DbStore::open_in_project(".")` (server cwd = project root, matching existing hook assumptions).

**Testing**: New contract test target `tests/contract/test_mcp_contract.rs` exercising the stdio protocol (initialize, tools/list, tools/call) against a seeded store; explicit exit-code assertions for every refactored call site.

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary) + MCP server subcommand.

**Performance Goals**: Server startup adds no meaningful latency; per-tool latency equals the existing function call.

**Constraints**: Constitution I (new dep is a pure-Rust SDK). Constitution VI (additive to the CLI surface). Constitution XI (no `process::exit` in logic; typed errors). CLI exit codes byte-preserved (existing contract tests must stay green).

**Scale/Scope**: New `src/cli/mcp.rs` + `src/mcp/` (server) module; refactors in `src/cli/derive.rs`, `src/verify/mod.rs`, `src/cli/status.rs`, `src/cli/record.rs`; `src/main.rs` wiring.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | `rmcp` is a pure-Rust SDK; no runtime engine. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Tools reuse the existing verified functions. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Additive subcommand; binary size measured. |
| VI. Dual passive/active surface | PASS | MCP is a new active surface; hooks untouched. |
| VII. Open-source governance | PASS | `rmcp` is Apache-2.0/MIT. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | MCP contract tests + explicit exit-code assertions. |
| X. Documentation matches behavior | PASS | README gains an MCP section; help text documents `rgt mcp`. |
| XI. Idiomatic Rust, exit codes | PASS | The refactor's entire purpose: typed errors, no `process::exit` in logic, exit codes preserved. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/041-mcp-server-mode/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── mcp-tool-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── mcp.rs           # NEW: Mcp subcommand entry
│   ├── derive.rs        # execute_derive: exit → typed Err (8 sites)
│   ├── record.rs        # RecordOutcome return (replace println-only)
│   └── status.rs        # StatusSummary return (extract json! value)
├── mcp/
│   ├── mod.rs           # NEW: RgtMcpServer + #[tool] impls
│   └── error.rs         # NEW: VerifyDeriveError
├── verify/
│   └── mod.rs           # run_verify_cli → pure run_verify + thin CLI wrapper (7 sites)
├── main.rs              # Mcp subcommand wiring
└── lib.rs               # add pub mod mcp

Cargo.toml               # rmcp pinned, test target registered

tests/
└── contract/
    └── test_mcp_contract.rs   # NEW: stdio protocol tests
```

**Structure Decision**: Single-project CLI. A new `src/mcp/` module owns the server + typed error; `src/cli/mcp.rs` is the thin subcommand entry; the refactored functions stay in their current homes.

## Complexity Tracking

> None — no constitution violations requiring justification.
