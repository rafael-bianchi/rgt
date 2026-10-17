# Research: Remove MCP Server

**Feature**: `013-remove-mcp-server` | **Date**: 2026-08-08

## Decision 1: Remove MCP Entirely

**Decision**: Delete `src/mcp/`, remove `rgt mcp` subcommand, strip all MCP references.

**Rationale**:
- RTK has zero MCP — confirmed via repository audit (no MCP files, no MCP in docs, no `mcp` in source).
- The user's explicit direction: "follow RTK closely, no MCP."
- Constitution VIII (CLI-First Hooks) already mandates hooks → CLI architecture. The MCP server is a parallel surface that duplicates the CLI functionality and violates the single-source-of-truth principle.
- Simpler architecture: one integration surface (hooks calling CLI subcommands) instead of two.

**Alternatives considered**:
1. **Keep MCP alongside hooks**: Violates the user's RTK-alignment direction and creates two maintenance surfaces. Rejected.
2. **Deprecate MCP gradually, remove later**: Adds no value — the codebase is pre-release. Rejected.

## Decision 2: Extract BFS Logic, Don't Rewrite

**Decision**: Extract the BFS traversal and lineage-building logic from `src/mcp/handlers.rs` into a new `src/query/mod.rs`. Do NOT rewrite it.

**Rationale**:
- The BFS logic is tested and correct (fix for issue #1 — provenance recursion).
- Rewriting risks introducing bugs in a proven algorithm.
- Extraction preserves test coverage — integration tests update their import path, not their assertions.

**Alternatives considered**:
1. **Rewrite from scratch**: Unnecessary risk for zero benefit. Rejected.
2. **Inline query logic into cli/query.rs**: Would duplicate the same logic for status and query commands. Rejected — shared module is cleaner.

## Decision 3: Remove Dead MCP Tools Without CLI Replacement

**Decision**: `handle_record_value`, `handle_record_derivation`, and `handle_list_stale_values` are MCP-only. Remove them. Do not create CLI equivalents in this spec.

**Rationale**:
- Record operations (`record_value`, `record_derivation`) are triggered by hooks, not by users. Recreating them as CLI commands is a separate feature decision.
- The `rgt verify` CLI (spec 011) already covers derivation verification — the MCP verification path is redundant.
- `list_stale_values` is already accessible via `rgt status --stale-only --json`.
- Follow-up specs can reintroduce CLI record/value commands if needed.

**Alternatives considered**:
1. **Create CLI equivalents immediately**: Increases scope of this spec. Better handled as a follow-up with its own specification. Rejected.

## Decision 4: Constitution Amendment

**Decision**: Amend v1.3.0 → v1.4.0. Replace Principle VI text to describe CLI-only active surface. Remove "MCP Standards" from Integration & Tooling Policy.

**Rationale**:
- Constitution VI currently mandates MCP as the "Active" surface. After removal, the active surface is CLI commands invoked by hooks.
- Constitution must remain authoritative — if it mandates removed features, it loses credibility.
- MINOR bump: material change to an existing principle.

**New Principle VI text**:
```
### VI. Dual Passive/Active Integration Surface (CLI-First)
RGT MUST expose two complementary operational interfaces:
1. Passive: PreToolUse and PostToolUse execution hooks for automatic background state capture.
2. Active: CLI subcommands (query, status, graph, verify) invoked by hooks or directly by users.
```
