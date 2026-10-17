# Data Model: Capture Bash-Tool Reads

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for the Bash-capture changes. No new persisted entities — the `NormalizedCapture` model is populated from additional fields.

## Entities

### BashCommand

The `tool_input.command` string of a Bash tool call.

| Attribute | Type | Notes |
|---|---|---|
| `tokens` | `Vec<&str>` | Whitespace-split tokens. |

Rules:
- The effective verb is the first token, EXCEPT when the first token is `rtk`, in which case the second token is the verb (FR-002).
- The path is the first non-flag argument of a recognized read verb.

### ReadStyle

The recognized read verbs.

| Value | Notes |
|---|---|
| `cat`, `read`, `less`, `head`, `tail`, `more`, `python`, `python3` | Existing. |
| `grep` | **New** (FR-002) — path-only capture (stdout is partial). |
| `rtk read` | **New wrapper** — `rtk` + a read verb; `rtk cat` is not recognized. |

### NormalizedCapture

The internal `{path, content}` model.

| Attribute | Type | Notes |
|---|---|---|
| `path` | `Option<String>` | From `tool_input.path`/`file_path` (Read tool) or from `tool_input.command` (Bash). |
| `content` | `Option<String>` | From `tool_response.content` (Read) or `tool_response.stdout` (Bash, when non-empty). |

Rules:
- For `None`/`claude-code`/`cursor`, the Read-shaped dialect is tried first; a pathless result falls back to the Bash-shaped dialect (FR-001).
- `content == None` with `path == Some` triggers the disk-read fallback in the hook runtime (unchanged).

## Validation rules (from FR-001..FR-004)

- **Fallback** (FR-001): Bash calls yield a path for claude-code/cursor.
- **Wrapper** (FR-002): `rtk read` recognized; `rtk <non-read>` is a no-op.
- **stdout content** (FR-003): non-empty stdout → content; else path-only.
- **Passive** (FR-004): no command mutation.

## Out of scope

Other 11 agents' dialects (already handled); RTK itself; the hook runtime/disk fallback.
