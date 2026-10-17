# Passive Hooks Interface Contract: `rgt hook`

**Feature Branch**: `001-provenance-tracking` | **Date**: 2026-07-31

## Overview

Passive hooks capture background file reading and execution events emitted by AI coding tools (Claude Code, Cursor, Codex CLI, Windsurf). Hook stdin payloads are parsed asynchronously, updating `.rgt/store.db` without blocking host tool execution.

---

## 1. `PostToolUse` Payload Format

Triggered after an AI tool completes a file read or search operation.

### Input JSON Payload (stdin)
```json
{
  "event": "PostToolUse",
  "tool_name": "ReadLocalFile",
  "tool_input": {
    "path": "docs/project_budget.md"
  },
  "tool_response": {
    "content": "Total budget is 150000 USD. Start date: 2026-01-01."
  }
}
```

### Hook Processing Logic
1. Hook receiver parses `path` from `tool_input`.
2. Evaluates Tier 1 metadata (`mtime` + size) and Tier 2 BLAKE3 hash for `docs/project_budget.md`.
3. Scans `tool_response.content` for numeric values (`150000`) and ISO dates (`2026-01-01`).
4. Records/updates `source_documents` and `tracked_nodes` in `.rgt/store.db`.
5. If file content changed since last read, triggers transactional invalidation cascade.

---

## 2. Non-Blocking Fail-Open Guarantee

- Hook process MUST enforce a hard execution timeout of 500 milliseconds.
- If database locks or file reading errors occur, `rgt hook` exits with status `0` after logging a silent warning to `.rgt/hook.log`, ensuring host AI coding sessions are never interrupted.
