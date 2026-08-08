# Quickstart: Remove MCP Server

**Feature**: `013-remove-mcp-server` | **Date**: 2026-08-08

## Prerequisites

- Rust toolchain, repository on feature branch
- Existing test nodes in `.rgt/store.db` (for query testing)

## Validation Scenarios

### Scenario 1: MCP Source Code Removed

```bash
grep -r "mcp\|MCP\|Model Context Protocol" src/ || echo "CLEAN"
```

**Expected**: `CLEAN` — zero MCP references in source code.

### Scenario 2: `rgt mcp` Fails Gracefully

```bash
cargo run -- mcp; echo "Exit: $?"
```

**Expected**: Error message like "unrecognized command" or clap default error. Exit code non-zero.

### Scenario 3: `rgt query` Works Without MCP

```bash
# Record a value node, get its ID, then query
cargo run -- query <node_id>
cargo run -- query <node_id> --json
```

**Expected**: Lineage output matches pre-removal output (identical BFS logic).

### Scenario 4: No MCP in Docs

```bash
grep -ri "mcp\|Model Context Protocol" README.md AGENTS.md CONTRIBUTING.md || echo "CLEAN"
```

**Expected**: `CLEAN` — zero MCP in documentation.

### Scenario 5: Constitution Updated

```bash
grep "MCP" .specify/memory/constitution.md || echo "CLEAN"
```

**Expected**: `CLEAN` — zero MCP in constitution. Principle VI mentions CLI subcommands, not MCP.

### Scenario 6: Full Test Suite

```bash
cargo test --all -- --test-threads=1
```

**Expected**: All tests pass (integration tests use extracted query module, not MCP handlers).
