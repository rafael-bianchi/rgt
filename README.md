# RGT (Rust Graph Tracker)

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**RGT (Rust Graph Tracker)** gives AI coding agents (Claude Code, Cursor, Codex CLI, Windsurf) a persistent, queryable memory of every number and date they read from source files or derive through calculations — then **verifies each derivation is mathematically correct** — ensuring they never silently reason from stale or incorrect data.

---

## Key Features

- **Zero-Dependency Single Binary**: Pure Rust with bundled SQLite, cross-compiled for macOS, Linux, and Windows.
- **Two-Tier File Detection**: Tier 1 (`mtime` + `size`) < 1ms per file; Tier 2 (BLAKE3) only on change.
- **Provenance DAG**: `petgraph` reverse-edge graph tracks how every value was derived. Invalidation cascades update only affected subgraph.
- **Rich Value Types**: Native `Number`, `Date`, and `Duration` types with first-class date arithmetic.
- **Derivation Verification**: `rgt verify` re-computes derived values using expression evaluation (`a + b * c`) and date arithmetic (`date2 - date1`), rejecting incorrect calculations before they enter the graph.
- **Dual Integration**: Passive hooks (`PreToolUse`/`PostToolUse`) + active CLI subcommands (`rgt query`, `rgt status`, `rgt verify`, `rgt graph`).
- **One-Command Setup**: `rgt init -g` auto-detects AI coding tools and configures hooks.
- **Self-Update**: `rgt update` performs atomic in-place binary upgrades from GitHub Releases.

---

## Quickstart

### Installation

**Shell (macOS/Linux):**
```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

**Homebrew:**
```bash
brew install rafael-bianchi/rgt/rgt
```

**Cargo (from Git):**
```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Initialize

```bash
rgt init -g       # auto-configure hooks for all detected AI tools
rgt init -g --agent claude-code  # configure a specific agent only
```

### Record and Verify Values

RGT operates through passive hooks that call CLI subcommands. AI agent hooks invoke these commands automatically:

```
record_value       # record a root number or date from a source file
record_derivation  # record a derived calculation linked to parent nodes
query_provenance   # trace a value back to its source files
list_stale_values  # find values whose sources have changed
```

**Verification happens automatically**: when the agent calls `record_derivation`, the hook calls `rgt verify` to re-compute the result from parent values. Wrong values are rejected before storage.

### Example: End-to-End Flow

```bash
# 1. Initialize
rgt init

# 2. Agent records values via hooks (automatic)
# 3. Agent computes 100 + 200 = 300, hooks call:
rgt verify --parents node_a,node_b --operation EXPRESSION --expression "a + b" --result 300
# exit 0 — verified

# 4. If the agent gets it wrong:
rgt verify --parents node_a,node_b --operation EXPRESSION --expression "a + b" --result 500
# Error: verification failed for EXPRESSION "a + b"
#   expected: 300, got: 500
#   hint: retry with the correct result
# exit 1 — rejected

# 5. Date arithmetic:
rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000
# exit 0 (10 days verified)

# 6. Query provenance:
rgt query node_drv_a1b2c3d4 --json

# 7. Check graph status:
rgt status
rgt status --stale-only --json

# 8. Export graph:
rgt graph --format mermaid
```

---

## CLI Reference

### `rgt init [-g] [--force] [--agent <name>]`

Initialize the `.rgt/store.db` database and configure AI agent hooks. `-g` installs global hooks. `--agent` targets `claude-code`, `cursor`, `windsurf`, or `codex`.

### `rgt verify --parents <ids> --operation <op> --result <val> [--expression <expr>]`

Re-compute a derived value from parent nodes and compare against the claimed result. Exit codes: `0` (match/unknown op), `1` (mismatch), `2` (invalid input). Operations: `EXPRESSION` and `DATE_DIFF`. Variables: `parent[0]=a, parent[1]=b, ...`.

### `rgt status [--stale-only] [--json]`

Inspect the provenance graph: total, active, and stale node counts. `--stale-only` filters to stale nodes only.

### `rgt query <node_id> [--json]`

Query the complete derivation lineage for a node ID. Traces the value back to its source files.

### `rgt graph [-f text|mermaid|dot]`

Export the dependency graph as text, Mermaid diagram, or Graphviz DOT format.

### `rgt hook <pre|post>`

Execute a passive hook (reads tool event JSON from stdin). Used by agent hook scripts.

### `rgt update [--check] [-y] [--version <tag>]`

Check for and install binary updates from GitHub Releases.

---

## Concepts

### Provenance Graph

RGT models values as nodes in a directed acyclic graph (DAG). Each node knows its parents — the source files or prior calculations it depends on.

| Node Type | Created By | Example |
|---|---|---|
| Root | `record_value` | A number read from a config file |
| Derived | `record_derivation` | `revenue = price * quantity` |

### Staleness

When a source file changes, its root nodes become **stale**. Staleness cascades downstream: every derived node that depends on a stale node is also stale. `rgt status` shows the staleness state. `rgt query` traces the dependency chain to explain *why* a value is stale.

### Verification

Every derivation is **trust-but-verify**. When an agent records `revenue = price * quantity`, RGT reads `price` and `quantity` from the database, re-computes the product, and confirms it matches. If the agent made an arithmetic error, the derivation is rejected before it enters the graph.

---

## License

Licensed under either of [MIT license](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE) at your option.
