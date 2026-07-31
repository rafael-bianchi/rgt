# CLI Interface Contract: `rgt`

**Feature Branch**: `001-provenance-tracking` | **Date**: 2026-07-31

## Command Overview

```text
rgt [OPTIONS] <COMMAND>

Subcommands:
  init      Initialize RGT project tracking and configure AI agent hooks
  status    Inspect provenance graph status, active nodes, and stale values
  query     Query provenance lineage or value status by node ID or file path
  graph     Export or visualize the dependency graph structure
  hook      Process passive execution hooks (PreToolUse / PostToolUse) from stdin
  mcp       Start the Model Context Protocol stdio RPC server
```

---

## 1. `rgt init`

Auto-configures RGT for the current repository and initializes `.rgt/store.db`.

```text
Usage: rgt init [OPTIONS]

Options:
  -g, --global    Detect and configure global hooks for Claude Code, Cursor, Codex CLI, and Windsurf
      --force     Overwrite existing hook configurations if present
  -h, --help      Print help information
```

### Exit Codes
- `0`: Initialization successful.
- `1`: Directory write error or unsupported tool detection issue.

---

## 2. `rgt status`

Lists all recorded values and highlights currently stale nodes.

```text
Usage: rgt status [OPTIONS]

Options:
      --stale-only    Show only stale nodes and invalidation sources
      --json          Output status summary in JSON format
  -h, --help          Print help information
```

### Example JSON Output (`rgt status --json`)
```json
{
  "total_nodes": 12,
  "active_nodes": 9,
  "stale_nodes": 3,
  "tracked_files": 4,
  "stale_details": [
    {
      "node_id": "node_drv_7a8b9c",
      "value_kind": "NUMBER",
      "value": 300.0,
      "stale_reason": "PARENT_NODE_STALE",
      "root_source_file": "docs/pricing.md"
    }
  ]
}
```

---

## 3. `rgt query`

Queries the provenance lineage ("Why is this value X?") for a given node ID.

```text
Usage: rgt query <NODE_ID> [OPTIONS]

Options:
      --json          Output derivation lineage as structured JSON
  -h, --help          Print help information
```

---

## 4. `rgt graph`

Exports or renders the dependency DAG.

```text
Usage: rgt graph [OPTIONS]

Options:
      --format <FORMAT>    Export format: dot, mermaid, text [default: text]
  -h, --help               Print help information
```

---

## 5. `rgt hook`

Internal command invoked by AI agent execution hooks.

```text
Usage: rgt hook <EVENT_TYPE>

Arguments:
  <EVENT_TYPE>    Hook phase: pre or post
```
