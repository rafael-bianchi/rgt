# Technical Research & Architectural Decisions: Provenance Tracking

**Feature Branch**: `001-provenance-tracking` | **Date**: 2026-07-31

## Overview

This document records the architectural research, dependency selections, and design decisions for RGT's numeric and date provenance tracking engine.

---

## 1. Dependency Graph Engine (`petgraph`)

### Decision
Use `petgraph::graph::DiGraph<NodeId, EdgeData>` with explicit node indexing and reverse-edge traversal capabilities.

### Rationale
- **Directional Clarity**: Edges are directed from Parent (`Source Document` or `Parent Value`) to Child (`Derived Value`).
- **Cascade Efficiency**: When a root file is modified, a Reverse Depth-First Search (`petgraph::visit::Dfs`) or Breadth-First Search (`Bfs`) down the outgoing edges visits strictly the downstream affected subgraph. Invalidation cost is $O(k)$ where $k$ is the number of downstream dependent nodes, satisfying Constitution Principle III.
- **Lineage Traversal**: "Why is this value X?" queries walk incoming edges upstream to locate all root source file nodes.
- **Cycle Prevention**: `petgraph::algo::is_cyclic_directed` checks graph validity before committing new derivation edges.

### Alternatives Evaluated
- *Custom Adjacency List*: Prone to edge-case bugs in graph algorithms and lacks optimized graph traversal primitives.
- *`daggy` Crate*: A wrapper around `petgraph`; rejected to avoid unnecessary abstraction layers and maintain tight control over graph indexing.

---

## 2. Two-Tier File Change Detection Pipeline

### Decision
Implement a two-tier verification pipeline:
1. **Tier 1 (Metadata Check)**: Query OS file metadata via `std::fs::metadata`. Compare `mtime` (nanosecond precision) and file size `len()`.
2. **Tier 2 (Cryptographic Hash Check)**: Execute BLAKE3 content hashing (`blake3::Hasher`) ONLY when Tier 1 detects a modification or explicit verification is requested.

### Rationale
- **Sub-Millisecond Speed**: Tier 1 metadata checks complete in <50 microseconds on OS page cache hits, guaranteeing the "no-change" execution path stays well under the 1ms budget required by Constitution Principle II.
- **Cryptographic Security**: BLAKE3 hashes at ~3–6 GB/s on modern CPUs, making Tier 2 checks extremely fast when content verification is actually needed.
- **Storage**: Store `mtime_nsec`, `file_size`, and `blake3_hash` in `.rgt/store.db`.

### Alternatives Evaluated
- *SHA-256 / MD5*: Significantly slower hashing speeds than BLAKE3.
- *Pure `mtime`*: Unreliable if a build script or `touch` command updates modification timestamps without altering content.

---

## 3. Storage Layer (`rusqlite` with Bundled SQLite)

### Decision
Use `rusqlite` with the `bundled` feature flag to embed SQLite directly into the RGT binary, writing state to `.rgt/store.db`.

### Rationale
- **Zero Runtime Dependencies**: `rusqlite` with `bundled` compiles SQLite's C source code directly into the Rust executable. No dynamic C library linking (`libsqlite3.so`/`.dylib`) is required, satisfying Constitution Principle I.
- **ACID Transactions**: Invalidation cascades and batch node writes execute inside single SQLite transactions (`db.transaction()`), guaranteeing consistency.
- **WAL Mode**: Enabling `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;` delivers high throughput and non-blocking reader access.

### Alternatives Evaluated
- *`redb` / `sled`*: Key-value stores lack native SQL indexing and relational join capabilities for complex provenance chain queries.
- *`sqlx`*: Adds heavy async macro overhead unnecessary for embedded local SQLite databases.

---

## 4. Value Types & Temporal Computations (`chrono`)

### Decision
Represent graph node values using a Rust enum wrapped around `chrono` primitives:

```rust
pub enum ValueType {
    Number(f64),
    Date(chrono::DateTime<chrono::Utc>),
    Duration(chrono::Duration),
}
```

### Rationale
- **Native Date-Diffs**: Subtracting two `Date` nodes produces a first-class `Duration` node via `chrono::DateTime::signed_duration_since`.
- **Formatting & Parsing**: Standardized ISO-8601 string parsing and formatting.
- **Precision**: Handles timezone offsets, leap seconds, and calendar calculations accurately.

### Alternatives Evaluated
- *Raw Unix Epoch Timestamps*: Fails to preserve timezone context and formatted calendar representations.
- *Custom Date Structs*: High risk of edge-case bugs around leap years and calendar math.

---

## 5. Agent Integration (Passive Hooks & MCP Server)

### Decision
Provide a dual integration surface within the same single binary:
1. **Passive Hooks**: Subcommand `rgt hook [pre|post]` that receives JSON events from Claude Code, Cursor, Codex CLI, or Windsurf via stdin, extracts file path/value hints, and updates `.rgt/store.db` fail-open with a 500ms timeout.
2. **Active MCP Server**: Subcommand `rgt mcp` starting a stdio JSON-RPC 2.0 server offering MCP tools:
   - `record_value`: Register a root value from a source file location.
   - `record_derivation`: Register a derived value from parent value IDs.
   - `query_provenance`: Retrieve lineage tree for a value ID.
   - `list_stale_values`: List all currently invalid/stale values and root sources.
3. **Environment Setup**: Subcommand `rgt init -g` auto-detects installed coding tools and writes hook configuration snippets to user/project settings.

---

## Summary of Technology Stack

| Component | Selected Technology | Constitutional Justification |
| :--- | :--- | :--- |
| **Language** | Rust 1.75+ (Edition 2021) | Principle I: Rust-only single binary |
| **Graph Engine** | `petgraph` | Principle III: Reverse-edge DAG traversal |
| **Change Detection** | Tier 1 (`mtime`+size) + Tier 2 (`blake3`) | Principle II: Sub-1ms no-change path |
| **Database** | SQLite via `rusqlite` (bundled) | Principle I & Clarification: Transactional ACID storage |
| **Temporal Math** | `chrono` | Principle IV: Native Date and Duration types |
| **CLI & MCP** | `clap`, `tokio`, `serde_json` | Principle VI: Passive hooks & active MCP server |
