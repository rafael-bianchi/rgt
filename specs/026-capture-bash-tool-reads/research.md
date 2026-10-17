# Research: Capture Bash-Tool Reads

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Consolidates findings §8 (already backed by a live-captured payload) and the current `src/hooks/parser.rs` behavior. No external research needed.

## 1. Dispatch fallback (FR-001)

**Decision**: In `normalize_agent_event`, for `None`/`claude-code`/`cursor`, call `default_dialect` first; if the resulting capture has `path == None`, fall back to `command_dialect`.

**Rationale**: A Bash call carries `tool_input.command` (no `path`/`file_path`) and `tool_response.stdout` (no `content`), so `default_dialect` returns a capture with both fields `None` and the hook exits early. The §8.1 captured payload confirms this shape. Falling back to `command_dialect` (which already parses `tool_input.command` and is wired to gemini/vibe/opencode/pi/hermes) reuses existing, tested logic. Native `Read` events still hit `default_dialect` first and are unchanged.

**Alternatives considered**: branching on `tool_name == "Bash"` before choosing a dialect — rejected: the fallback-when-pathless approach is simpler and also catches any other command-shaped event.

## 2. `rtk` wrapper + `grep` (FR-002)

**Decision**: In `extract_path_from_command`, if the first token is `rtk`, treat the *second* token as the verb and check it against the read-style list; add `grep` to the list.

**Rationale**: RTK's PreToolUse hook rewrites `cat file` → `rtk read file` before RGT sees it (live-confirmed §8.1/§8.3). The current code checks only the first token, so `rtk` (not a read verb) yields no path. RTK's read subcommand is `read` (per its README), so `rtk read <path>` is the wrapper to recognize; `rtk cat` is not a valid RTK read and must stay a no-op. `grep` is added to the read verbs per §8.1's "cat/head/tail/grep" set (grep's path is captured so the disk fallback can record the file; its stdout is partial, addressed below).

**Alternatives considered**: a general "unwrap one wrapper level" (peel any leading non-flag token) — rejected: too permissive; only `rtk` is a known wrapper.

## 3. stdout as content (FR-003)

**Decision**: `command_dialect` reads `tool_response.stdout` and, when non-empty, sets `content`; otherwise `content` stays `None` (path-only), and the existing disk-read fallback in `handle_passive_hook_event` records the file.

**Rationale**: For full-content readers (`cat`, `python`), stdout equals the file content; capturing it avoids a disk re-read and records exactly what the agent saw. For `head`/`tail`/`grep`, stdout is a slice — it is still captured as content (documented: it is what the agent saw). When stdout is absent, `content: None` preserves the current path-only → disk-fallback flow.

**Alternatives considered**: content only for full-readers (path-only for head/tail/grep) — rejected: complicates the model for marginal correctness gain; the spec explicitly documents partial content as acceptable.
