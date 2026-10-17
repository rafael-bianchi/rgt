# Implementation Plan: Numeric and Date Provenance Tracking

**Branch**: `001-provenance-tracking` | **Date**: 2026-07-31 | **Spec**: [spec.md](file:///Users/rafaelbianchi/dev/repos/rgt/specs/001-provenance-tracking/spec.md)

**Input**: Feature specification from `/specs/001-provenance-tracking/spec.md`

## Summary

Implement numeric and date provenance tracking in RGT (Rust Graph Tracker) as a single, zero-dependency Rust binary. The system intercepts or receives tool execution events, captures root numbers/dates from source files, records derived calculations/date-diffs, and maintains a persistent directed acyclic graph (DAG) in SQLite (`.rgt/store.db`). Using two-tier change detection (`mtime` + `size` before BLAKE3) and `petgraph` reverse-edge traversal, file edits trigger sub-millisecond invalidation cascades marking downstream derived values as stale. The system exposes both passive hooks (`PreToolUse`/`PostToolUse`) and an active MCP server for AI coding agents (Claude Code, Cursor, Codex CLI, Windsurf).

## Technical Context

**Language/Version**: Rust 1.75+ (Edition 2021)

**Primary Dependencies**: `petgraph` (DAG graph engine), `chrono` (Date & Duration types), `blake3` (Tier 2 hashing), `rusqlite` (bundled SQLite persistence), `clap` (CLI parser), `tokio` (async runtime for MCP stdio RPC), `serde` / `serde_json` (de/serialization)

**Storage**: Embedded SQLite database in `.rgt/store.db` within the project root directory

**Testing**: Standard `cargo test` harness (unit tests in `src/`, integration & contract tests in `tests/`)

**Target Platform**: macOS (x86_64, aarch64), Linux (x86_64, aarch64), Windows (x86_64)

**Project Type**: Single-binary CLI application with embedded MCP server and hook handler

**Performance Goals**: Sub-1ms per file for Tier 1 no-change detection checks; <5ms passive hook latency; <10ms graph query latency for repositories up to 100k nodes

**Constraints**: Zero runtime dynamic dependencies (bundled SQLite, static libc linkage where applicable); single binary executable distribution; non-blocking fail-open behavior on passive hooks

**Scale/Scope**: Up to 100,000 active nodes per project repository

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle I (Rust-Only Single Binary)**: PASS - Built entirely in Rust 1.75+ using bundled SQLite (`rusqlite` with `bundled` feature flag) yielding a single executable binary with no external runtime dependencies.
- **Principle II (Speed-First Two-Tier Detection & BLAKE3)**: PASS - Tier 1 (`mtime` + `size`) avoids disk reads and cryptographic hashing for unchanged files, ensuring sub-1ms evaluation. BLAKE3 is invoked only when Tier 1 detects modifications.
- **Principle III (Graph Correctness & petgraph Reverse Traversal)**: PASS - DAG engine uses `petgraph` with reversed edge direction so invalidation cascades walk strictly downstream dependents from modified source files.
- **Principle IV (Value Types & Chrono Derivations)**: PASS - `Number`, `Date`, and `Duration` types are natively defined using `chrono` dates/durations, making date differentials first-class derived values.
- **Principle V (Multi-Channel UX & RTK Alignment)**: PASS - Single binary packaged for shell installer, Homebrew tap, and `cargo install`, with a one-command `rgt init -g` auto-configuring AI agent hooks.
- **Principle VI (Dual Passive/Active Integration Surface)**: PASS - Implements passive `PreToolUse`/`PostToolUse` hook processing alongside an active MCP stdio RPC server.
- **Principle VII (Permissive Open-Source Licensing)**: PASS - Dual-licensed under MIT OR Apache-2.0.

## Project Structure

### Documentation (this feature)

```text
specs/001-provenance-tracking/
├── plan.md              # Implementation plan
├── research.md          # Technical research & decisions
├── data-model.md        # Database schema & entity definitions
├── quickstart.md        # Feature validation guide
├── contracts/           # Interface specifications
│   ├── cli-schema.md    # CLI commands & flags
│   ├── mcp-schema.md    # MCP tools & JSON-RPC schema
│   └── hooks-schema.md  # PreToolUse / PostToolUse hook payloads
└── tasks.md             # Execution task list
```

### Source Code (repository root)

```text
src/
├── main.rs              # Entry point dispatching CLI, MCP server, or hook modes
├── cli/                 # CLI subcommand handlers (init, status, query, graph)
│   ├── mod.rs
│   ├── init.rs
│   ├── status.rs
│   ├── query.rs
│   └── graph.rs
├── graph/               # Petgraph DAG management & reverse invalidation cascade
│   ├── mod.rs
│   ├── engine.rs
│   └── invalidation.rs
├── store/               # SQLite persistence layer via rusqlite
│   ├── mod.rs
│   ├── db.rs
│   ├── schema.rs
│   └── queries.rs
├── detection/           # Two-tier file modification detection pipeline
│   ├── mod.rs
│   ├── tier1.rs
│   └── tier2_blake3.rs
├── types/               # Value types (Number, Date, Duration via chrono), Node, Edge
│   ├── mod.rs
│   ├── value.rs
│   ├── node.rs
│   └── edge.rs
├── hooks/               # Passive hook event handlers and installer
│   ├── mod.rs
│   ├── parser.rs
│   └── installer.rs
└── mcp/                 # MCP JSON-RPC stdio server implementation
    ├── mod.rs
    ├── protocol.rs
    └── handlers.rs

tests/
├── contract/            # MCP tool & Hook execution contract tests
├── integration/         # Invalidation cascade and multi-step derivation tests
└── unit/                # Core graph engine, two-tier detection, and chrono tests
```

**Structure Decision**: Single Rust crate layout with modular domain subdirectories under `src/` to separate CLI parsing, graph math (`petgraph`), storage (`rusqlite`), change detection (`blake3`), value types (`chrono`), passive hooks, and active MCP server logic.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

*No violations. All constitutional constraints strictly satisfied.*
