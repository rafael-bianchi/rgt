# Quickstart & Feature Validation Guide: Provenance Tracking

**Feature Branch**: `001-provenance-tracking` | **Date**: 2026-07-31

## Overview

This guide provides end-to-end validation scenarios to verify that numeric and date provenance tracking, sub-millisecond two-tier change detection, and invalidation cascades function correctly in RGT.

---

## Prerequisites

- Built RGT binary (`target/release/rgt`) placed in `PATH` or executed directly.
- Terminal with standard shell (macOS zsh/bash, Linux bash, Windows PowerShell/cmd).

---

## Scenario 1: One-Command Project Initialization

### Objective
Verify that `rgt init -g` initializes `.rgt/store.db` and auto-configures agent hooks.

### Execution
```bash
# Initialize RGT tracking in current repository
rgt init -g
```

### Expected Outcome
- `.rgt/store.db` SQLite database is created with schema tables (`source_documents`, `tracked_nodes`, `derivation_edges`).
- Output confirms hook registration for supported AI coding tools (Claude Code, Cursor, Codex CLI, Windsurf).

---

## Scenario 2: Record Source Values and Derived Calculation

### Objective
Simulate an AI agent reading a date from a file and computing a derived date-diff `Duration`.

### Execution
Create a test file `docs/timeline.md`:
```markdown
# Project Timeline
Start Date: 2026-01-01
End Date: 2026-01-31
```

Record values via MCP or CLI test tool:
```bash
# Record root start date
rgt hook post <<'EOF'
{
  "event": "PostToolUse",
  "tool_name": "ReadLocalFile",
  "tool_input": {"path": "docs/timeline.md"},
  "tool_response": {"content": "Start Date: 2026-01-01\nEnd Date: 2026-01-31"}
}
EOF
```

Check initial status:
```bash
rgt status --json
```

### Expected Outcome
- `total_nodes` shows recorded root nodes (`2026-01-01` and `2026-01-31`).
- `stale_nodes` count is `0`.

---

## Scenario 3: Source File Edit & Automatic Invalidation Cascade

### Objective
Verify that editing `docs/timeline.md` causes Tier 1 (`mtime` + size) change detection to trigger a transactional invalidation cascade in <1ms.

### Execution
```bash
# Modify source file
echo "Start Date: 2026-02-01" > docs/timeline.md

# Run status check
rgt status --json
```

### Expected Outcome
- `stale_nodes` count increases to reflect all root and derived nodes anchored to `docs/timeline.md`.
- `stale_reason` reports `"SOURCE_FILE_MODIFIED"`.
- Status execution completes in sub-millisecond latency per checked file.

---

## Scenario 4: Query Derivation Lineage ("Why is this value X?")

### Objective
Query the full provenance chain of a derived node.

### Execution
```bash
# Query derivation tree for a node ID
rgt query <NODE_ID> --json
```

### Expected Outcome
- Output displays structured lineage tree containing original file path (`docs/timeline.md`), line numbers, parent node values, and operation types.
