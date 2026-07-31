# RGT (Rust Graph Tracker)

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**RGT (Rust Graph Tracker)** gives AI coding agents (Claude Code, Cursor, Codex CLI, Windsurf) a persistent, queryable memory of every number and date they read from source files or derive through calculations, ensuring they never silently reason from stale data.

---

## Key Features

- **Zero-Dependency Single Binary**: Pure Rust implementation with bundled SQLite (`rusqlite`), cross-compiled for macOS, Linux, and Windows.
- **Speed-First Two-Tier Detection**: Tier 1 (`mtime` + file size) checks complete in **<1ms per file**, with BLAKE3 cryptographic hashing for Tier 2 content verification.
- **Graph Correctness & Minimal Invalidation**: `petgraph` DAG engine with reverse-edge traversal so invalidation cascades update only the affected downstream subgraph.
- **First-Class Temporal Derivations**: Native `Number`, `Date`, and `Duration` types powered by `chrono`.
- **Dual Integration Surface**: Passive background execution hooks (`PreToolUse`/`PostToolUse`) + active Model Context Protocol (MCP) stdio RPC server.
- **One-Command Setup**: `rgt init -g` auto-detects installed AI coding tools and writes hook configurations.
- **Self-Update Mechanism**: `rgt update` checks GitHub Releases and performs atomic in-place binary upgrades with SHA-256 verification.

---

## Quickstart

### 1. Installation

**Shell (macOS/Linux):**
```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

**Homebrew:**
```bash
brew install rafael-bianchi/tap/rgt
```

**Cargo:**
```bash
cargo install rgt
```

**From Source:**
```bash
cargo build --release
```

### 2. Initialization

```bash
# Initialize RGT in project and auto-configure AI agent hooks
rgt init -g
```

### 3. Inspecting Graph Status

```bash
# Check node staleness summary
rgt status

# Output status as JSON
rgt status --json
```

### 4. Querying Value Lineage ("Why is this value X?")

```bash
# Query derivation tree for a node ID
rgt query node_drv_a1b2c3d4 --json
```

### 5. Exporting Dependency Graph

```bash
# Export as Mermaid diagram
rgt graph --format mermaid

# Export as Graphviz DOT format
rgt graph --format dot
```

---

## MCP Server Integration

Start the Model Context Protocol stdio RPC server mode:

```bash
rgt mcp
```

Exposed MCP Tools:
- `record_value`: Register a root number or date read from a source file.
- `record_derivation`: Register a derived value or date-diff calculation linked to parent node IDs.
- `query_provenance`: Retrieve lineage tree and root source file origins for a target value ID.
- `list_stale_values`: List all currently invalid or stale value nodes.

---

## License

Licensed under either of [MIT license](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE) at your option.
