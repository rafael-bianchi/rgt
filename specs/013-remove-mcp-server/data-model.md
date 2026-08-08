# Data Model: Remove MCP Server

**Feature**: `013-remove-mcp-server` | **Date**: 2026-08-08

## Overview

No database schema changes. No runtime data model changes. This feature removes the MCP server integration surface and replaces it with a direct CLI query path. The query logic (BFS traversal, lineage building) is moved, not rewritten.

## Entities

### Query Module (NEW: `src/query/mod.rs`)

Extracted from `src/mcp/handlers.rs`. Provides reusable query functions accessible by CLI subcommands.

**Functions**:

| Function | Source | Description |
|---|---|---|
| `query_provenance(node_id, db)` | `handle_query_provenance` | BFS traversal of derivation chain with cycle guard |
| `list_stale_nodes(db)` | `handle_list_stale_values` | Lists all nodes marked stale |

**Removed (no replacement)**:

| Function | Reason |
|---|---|
| `handle_record_value` | MCP-only; hooks handle value recording via passive hooks |
| `handle_record_derivation` | MCP-only; `rgt verify` CLI (spec 011) handles verification |

### CLI Surface Change

| Before | After |
|---|---|
| `rgt mcp` | Removed |
| `rgt query <id>` → MCP handler | `rgt query <id>` → `src/query/` module |
| MCP tools: 4 | CLI subcommands: 8 (unchanged count, mcp replaced by verify in spec 011) |

### Module Structure Change

```
BEFORE                          AFTER
src/                            src/
├── mcp/                        ├── query/          (NEW, extracted)
│   ├── mod.rs                  │   └── mod.rs
│   └── handlers.rs             ├── cli/
├── cli/                        │   └── query.rs    (MODIFIED)
│   └── query.rs                ├── lib.rs          (MODIFIED)
├── lib.rs                      └── main.rs         (MODIFIED)
└── main.rs
```

### Constitution Change

| Version | Principle VI | MCP Standards policy |
|---|---|---|
| v1.3.0 | "Active: MCP server" | Present |
| v1.4.0 | "Active: CLI subcommands" | Removed |
