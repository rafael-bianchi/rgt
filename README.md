# RGT (Rust Graph Tracker)

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

**RGT (Rust Graph Tracker)** gives AI coding agents (Claude Code, Copilot, Cursor, Gemini CLI, Cline/Roo Code, Windsurf, Codex CLI, OpenCode, Pi, Hermes, Mistral Vibe, Antigravity, Kilo) a persistent, queryable memory of every number and date they read from source files or derive through calculations — then **verifies each derivation is mathematically correct** — ensuring they never silently reason from stale or incorrect data.

---

## Key Features

- **Zero-Dependency Single Binary**: Pure Rust with bundled SQLite, cross-compiled for macOS, Linux, and Windows.
- **Two-Tier File Detection**: Tier 1 (`mtime` + `size`) < 1ms per file; Tier 2 (BLAKE3) only on change.
- **Provenance DAG**: `petgraph` reverse-edge graph tracks how every value was derived. Invalidation cascades update only affected subgraph.
- **Rich Value Types**: Native `Number`, `Date`, and `Duration` types with first-class date arithmetic.
- **Derivation Verification**: `rgt verify` re-computes derived values using expression evaluation (`a + b * c`) and date arithmetic (`date2 - date1`), rejecting incorrect calculations before they enter the graph. `rgt derive` combines verification with automatic recording.
- **Dual Integration**: Passive hooks (`PreToolUse`/`PostToolUse`) for automatic background capture, plus active CLI subcommands (`rgt record`, `rgt derive`, `rgt status`, `rgt query`, `rgt verify`, `rgt graph`) for explicit invocation.
- **One-Command Setup**: `rgt init` auto-detects AI coding tools and configures hooks.
- **Self-Update**: `rgt update` performs atomic in-place binary upgrades from GitHub Releases.

---

## Supported Agents

RGT configures provenance-capture hooks for all 13 agents that RTK covers, using each agent's native mechanism:

| Agent | `--agent` value | Mechanism |
|---|---|---|
| Claude Code | `claude-code` (alias `claude`) | Full tool hook |
| Cursor | `cursor` | Full tool hook |
| Copilot | `copilot` | Full tool hook (VS Code Chat) + Copilot CLI rules |
| Gemini CLI | `gemini` | Full tool hook |
| Mistral Vibe | `vibe` | Full tool hook |
| OpenCode | `opencode` | Thin TypeScript plugin |
| Pi | `pi` | Thin TypeScript extension |
| Hermes | `hermes` | Thin Python plugin |
| Windsurf | `windsurf` | Rules file (`.windsurfrules`) |
| Codex CLI | `codex` | Rules file (`AGENTS.md`) |
| Cline / Roo Code | `cline` (alias `roo-code`) | Rules file (`.clinerules`) |
| Antigravity | `antigravity` | Rules file |
| Kilo | `kilocode` (alias `kilo`) | Rules file |

Hooks are **provenance-capture only**: they record data and never rewrite, filter, or block the agent's tool commands.

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
rgt init                     # auto-detect and configure hooks for all installed agents
rgt init --agent claude-code # configure a specific agent only (13 supported + aliases)
```

### Record and Verify Values

RGT provides CLI subcommands for active provenance tracking. AI coding agents invoke these directly or through passive hooks:

```
rgt record         # extract and track numeric/date values from a file
rgt derive         # verify and record a derived calculation linked to parent nodes
rgt query          # trace a value back to its source files
rgt status         # find values whose sources have changed (stale nodes)
rgt verify         # verify a derived value matches its parent computations
```

**Verification is gated**: `rgt derive` re-computes the result from parent values before inserting. Wrong values are rejected (exit 1) without modifying the graph.

### Example: End-to-End Flow

```bash
# 1. Initialize
rgt init

# 2. Agent reads a data file, then records its values
rgt record budget.csv
# Output: Recorded 5 values from budget.csv

# 3. Check graph state
rgt status

# 4. Agent computes profit = revenue - costs, records the derivation
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
# Output: Derived node: node_drv_Z

# 5. If the agent gets it wrong:
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 99999
# Error: verification failed for EXPRESSION "a - b"
#   expected: 60000, got: 99999
#   hint: retry with the correct result
# exit 1 — rejected, no node inserted

# 6. Date arithmetic verification:
rgt verify --parents d1,d2 --operation DATE_DIFF --result 864000
# exit 0 (10 days verified)

# 7. Query provenance:
rgt query node_drv_Z --json

# 8. Check graph status:
rgt status
rgt status --stale-only --json

# 9. Export graph:
rgt graph --format mermaid
```

---

## CLI Reference

### `rgt init [-g] [--force] [--agent <name>]`

Initialize the `.rgt/store.db` database and configure AI agent hooks. `-g` installs global hooks. Without `--agent`, RGT detects every supported agent installed on the machine and configures all of them in one run. `--agent` targets a single agent by its canonical name (13 values: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`) or alias (`claude`, `roo-code`, `kilo`). Unknown names exit with code 2.

### `rgt verify --parents <ids> --operation <op> --result <val> [--expression <expr>]`

Re-compute a derived value from parent nodes and compare against the claimed result. Exit codes: `0` (match/unknown op), `1` (mismatch), `2` (invalid input). Operations: `EXPRESSION` and `DATE_DIFF`. Variables: `parent[0]=a, parent[1]=b, ...`.

### `rgt status [--stale-only] [--json]`

Inspect the provenance graph: total, active, and stale node counts. `--stale-only` filters to stale nodes only.

### `rgt query <node_id> [--json]`

Query the complete derivation lineage for a node ID. Traces the value back to its source files.

### `rgt graph [-f text|mermaid|dot]`

Export the dependency graph as text, Mermaid diagram, or Graphviz DOT format.

### `rgt hook <pre|post> [--agent <name>]`

Execute a passive hook (reads tool event JSON from stdin). `--agent` selects the agent's stdin dialect (e.g. `copilot`, `gemini`, `vibe`); omitted uses the Claude Code format. Always exits 0 and never alters the tool call.

### `rgt update [--check] [-y] [--version <tag>]`

Check for and install binary updates from GitHub Releases.

---

## Concepts

### Provenance Graph

RGT models values as nodes in a directed acyclic graph (DAG). Each node knows its parents — the source files or prior calculations it depends on.

| Node Type | Created By | Example |
|---|---|---|
| Root | `rgt record` or passive hooks | A number read from a config file |
| Derived | `rgt derive` | `revenue = price * quantity` |

### Staleness

When a source file changes, its root nodes become **stale**. Staleness cascades downstream: every derived node that depends on a stale node is also stale. `rgt status` shows the staleness state. `rgt query` traces the dependency chain to explain *why* a value is stale.

### Verification

Every derivation is **trust-but-verify**. When an agent records `revenue = price * quantity`, RGT reads `price` and `quantity` from the database, re-computes the product, and confirms it matches. If the agent made an arithmetic error, the derivation is rejected before it enters the graph.

---

## License

Licensed under either of [MIT license](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE) at your option.
