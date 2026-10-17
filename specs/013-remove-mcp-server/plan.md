# Implementation Plan: Remove MCP Server (RTK Alignment)

**Branch**: `013-remove-mcp-server` | **Date**: 2026-08-08 | **Spec**: [spec.md](./spec.md)

## Summary

Remove the MCP server entirely from the RGT codebase. Extract the BFS traversal and lineage-building logic from MCP handlers into a new `src/query/` module that CLI commands (`rgt query`, `rgt status`) call directly. Delete `src/mcp/`, remove `rgt mcp` subcommand, strip all MCP references from documentation, and amend the constitution to reflect a CLI-only integration surface — matching RTK's zero-MCP architecture.

## Technical Context

**Language/Version**: Rust Edition 2021. No new dependencies needed.

**Primary Dependencies**: `serde_json` (still needed for query output), `rusqlite` (already used), `petgraph` (already used).

**Storage**: No changes — SQLite queries remain identical. The query logic moves but the queries don't change.

**Testing**: `cargo test --all -- --test-threads=1`. Existing integration tests updated to use extracted query module.

**Target Platform**: All platforms — removal, no platform-specific code.

**Scale/Scope**: 3 files deleted, 5 files modified, 1 file created, 3 docs updated, 1 constitution amended. Net code removal.

## Constitution Check

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | Still pure Rust, no new deps |
| II | Two-Tier Detection | ✅ PASS | No impact |
| III | Graph Correctness | ✅ PASS | Query logic preserved |
| IV | Rich Value Types | ✅ PASS | No type changes |
| V | Multi-Channel Distribution | ✅ PASS | No impact |
| VI | Dual Passive/Active | ⚠️ **AMENDED** | Replaced with CLI-only active surface (amendment part of this spec) |
| VII | Permissive OSS | ✅ PASS | No license changes |
| VIII | CLI-First Hooks | ✅ PASS | **Reinforced** — this is the core of the change |
| IX | Test-Driven Verification | ✅ PASS | Tests preserved and updated |
| X | Documentation Matches Behavior | ✅ PASS | Docs updated to remove MCP |
| XI | Code Quality | ✅ PASS | Dead code removed, cleaner codebase |

**Gate result**: Principle VI requires amendment (part of this spec). All others pass.

## Project Structure

### Documentation

```text
specs/013-remove-mcp-server/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── cli-query.md
└── tasks.md             # Phase 2 output
```

### Source Code

```text
src/
├── mcp/                 # DELETE ENTIRE DIR
│   ├── mod.rs           # DELETE
│   └── handlers.rs      # DELETE (BFS logic extracted to src/query/)
├── query/               # NEW
│   └── mod.rs           # Extracted: query_provenance(), list_stale_nodes()
├── cli/
│   └── query.rs         # MODIFY: call src/query/ instead of MCP handler
├── lib.rs               # MODIFY: remove pub mod mcp, add pub mod query
└── main.rs              # MODIFY: remove Mcp variant + match arm

tests/
├── contract/
│   └── test_mcp_contract.rs  # DELETE
└── integration/
    └── test_derivation_chain.rs  # MODIFY: replace MCP handler calls

README.md                # MODIFY: remove MCP section
AGENTS.md                # MODIFY: remove MCP tools table, remove rgt mcp
CONTRIBUTING.md          # MODIFY: remove MCP from project structure
.specify/memory/constitution.md  # AMEND: v1.3.0 → v1.4.0
```

## Complexity Tracking

Net code removal — simpler architecture.
