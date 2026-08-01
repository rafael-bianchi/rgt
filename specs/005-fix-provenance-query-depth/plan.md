# Implementation Plan: Fix Provenance Query Traversal Depth

**Branch**: `005-fix-provenance-query-depth` | **Date**: 2026-08-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-fix-provenance-query-depth/spec.md`

## Summary

Fix `handle_query_provenance` in `src/mcp/handlers.rs` to perform full BFS ancestor traversal instead of a single-level parent lookup. Replace the loop at lines 209-218 with a `VecDeque`-based BFS queue and `HashSet` visited set for cycle protection. Add `total_ancestors` and `node_type` fields to the response schema. Extend integration tests to cover 3+ level chains, branching, cycles, and dangling edges. Zero new external dependencies — uses only `std::collections`.

## Technical Context

**Language/Version**: Rust Edition 2021 (consistent with existing codebase)

**Primary Dependencies**: `rusqlite` (bundled), `serde_json`, `chrono` — all already in `Cargo.toml`. No new crates added.

**Storage**: SQLite via `.rgt/store.db` using `rusqlite` with WAL mode (existing). No schema migrations required.

**Testing**: `cargo test` — Rust's built-in test runner. Test files in `tests/integration/`.

**Target Platform**: macOS (x86_64, aarch64), Linux (x86_64, aarch64), Windows (x86_64) — single binary.

**Project Type**: CLI tool + MCP stdio server (single Rust binary).

**Performance Goals**:
- BFS traversal for 1,000-node linear chain: <200ms
- Root node query (0 ancestors): <1ms
- Query for branching graph (5 ancestors in 10,000-node graph): <5ms

**Constraints**:
- Must maintain backward-compatible MCP response schema (additive fields only)
- Zero new external crate dependencies
- No async runtime changes (handlers are synchronous)
- Must not regress existing `query_provenance` behavior for single-level chains

**Scale/Scope**:
- Linear chains: up to 1,000 nodes deep
- Graph width: unlimited (BFS explores all branches)
- Expected common case: <50 nodes per derivation chain

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Initial Gate (Pre-Research)

| Principle | Status | Notes |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | Uses only `std::collections` (`VecDeque`, `HashSet`). No new crates. |
| II. Speed-First Two-Tier Detection & BLAKE3 | N/A | Detection pipeline unchanged by this fix. |
| III. Graph Correctness & Minimal Subgraph Invalidation | ✅ PASS | BFS with visited set preserves DAG invariants. Cycle guard prevents infinite loops from unexpected graph states. |
| IV. First-Class Rich Value Types & Temporal Derivations | N/A | Value types unchanged. BFS traversal is type-agnostic. |
| V. Multi-Channel Distribution & Frictionless UX | N/A | No distribution changes. |
| VI. Dual Passive/Active Integration Surface | ✅ PASS | Fixes the active MCP surface (`query_provenance`). Passive hooks unaffected. |
| VII. Permissive Open-Source Governance | N/A | No licensing changes. |

### Post-Design Re-Check (Phase 1 Complete)

| Principle | Status | Notes |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | Confirmed. `VecDeque` and `HashSet` from std, no crates added. |
| III. Graph Correctness | ✅ PASS | Confirmed. BFS design handles DAGs, cycles, diamonds, dangling edges. |
| VI. Dual Passive/Active Integration | ✅ PASS | Confirmed. Response schema adds `total_ancestors` and `node_type` — additive, backward-compatible. |

**Architecture & Performance Standards**:
- **Memory Safety**: ✅ All traversal uses safe Rust (`VecDeque`, `HashSet`). No `unsafe` blocks.
- **Latency Budget**: ✅ BFS with indexed SQL lookups per node. 1,000 nodes × ~0.2ms per round-trip ≈ 200ms worst case. Standard graphs (<100 nodes) stay under 10ms.
- **Cross-Platform Parity**: ✅ `std::collections` is universally available on all target platforms.

**Integration & Tooling Policy**:
- **Hook Reliability**: N/A — passive hooks unchanged.
- **MCP Standards**: ✅ `total_ancestors` and `node_type` are JSON-compatible types serialized via `serde_json`.

**Result**: ALL GATES PASS. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/005-fix-provenance-query-depth/
├── plan.md              # This file
├── research.md          # Phase 0 output: BFS vs DFS, cycle guard, schema changes
├── data-model.md        # Phase 1 output: existing schema + BFS algorithm pseudocode
├── quickstart.md        # Phase 1 output: validation scenarios
├── contracts/           # Phase 1 output
│   └── mcp-query-provenance.md  # Updated MCP contract for query_provenance
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── mcp/
│   ├── mod.rs           # MCP server dispatch (no changes)
│   └── handlers.rs      # ← PRIMARY CHANGE: handle_query_provenance BFS rewrite
├── store/
│   ├── queries.rs       # get_tracked_node, get_parent_edges (no changes)
│   └── ...
├── types/
│   ├── node.rs          # TrackedNode struct (no changes)
│   └── ...
└── ...

tests/
├── integration/
│   └── test_derivation_chain.rs  # ← EXTEND: multi-level, branching, cycle tests
└── contract/
    └── test_mcp_contract.rs      # ← EXTEND: validate new response fields
```

**Structure Decision**: Single project layout. Only two source files change (`handlers.rs` for the BFS fix, `test_derivation_chain.rs` for test coverage). Contract test gets lightweight additions for schema validation. No new files, modules, or crates.

## Complexity Tracking

> No violations — all gates pass without justification needed.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| *(none)* | | |
