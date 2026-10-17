# Contract: Bash-Tool Read Capture Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the unit tests in `tests/unit/hook_parsers.rs`.

## Normalization invariants

1. **Fallback (FR-001)**: `normalize_agent_event(None|claude-code|cursor, bash_payload)` returns a capture with a non-empty `path` (from `tool_input.command`) — even though the payload has no `tool_input.path`/`tool_response.content`.
2. **Read unchanged (FR-001)**: a native `Read`-shaped payload still normalizes via `default_dialect` (same path/content as before).
3. **Wrapper (FR-002)**: `extract_path_from_command("rtk read /p/f")` == `Some("/p/f")`; `extract_path_from_command("rtk cat /p/f")` == `None`; `extract_path_from_command("grep -r x /p")` == `Some("/p")`.
4. **stdout content (FR-003)**: a Bash payload with non-empty `tool_response.stdout` yields `content == Some(stdout)`; with empty/absent stdout, `content == None` and `path` set.
5. **No-op (FR-004)**: a non-read Bash command (`ls`, `git status`) yields `path == None`; no hook path mutates the command string.

## Acceptance linkage

- US1/FR-001/SC-001 → invariants 1, 2.
- US2/FR-002/SC-002 → invariant 3.
- US3/FR-003/FR-004/SC-003/SC-004/SC-005 → invariants 4, 5.
