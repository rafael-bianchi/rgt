# Feature Specification: Capture Bash-Tool Reads

**Feature Branch**: `feat/026-capture-bash-tool-reads`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "Claude Code and Cursor Bash tool calls that read files (`cat`, `head`, `tail`, `grep`, `less`, `more`, `python`, ...) are invisible to RGT's hook today, because the hook only inspects the native `Read` tool's fields. The hook must also capture Bash reads: extract the file path from `tool_input.command` and the content from `tool_response.stdout`, and it must see through RTK's auto-rewrite of `cat file` into `rtk read file`. Findings §8."

Source context: `docs/findings-hooks-and-value-extraction.md` §8 (plan: `docs/spec-plan-from-findings.md`, spec #8, P1).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bash Reads Are Captured for Claude Code and Cursor (Priority: P1)

A Claude Code or Cursor agent runs `cat report.csv` (or `head`/`tail`/`grep`/`less`/`more`/`python`) via the Bash tool. Today the hook receives a Bash payload (`tool_input.command`, `tool_response.stdout`) that doesn't match the `Read`-tool shape it expects, so the read is a complete silent blind spot. The hook must extract the file path from the command and capture the read's output.

**Why this priority**: §8 confirmed empirically — every Bash read by the two flagship integrations is invisible, a major capture gap.

**Independent Test**: Unit tests feed the §8.1 captured payload shape (`tool_name:"Bash"`, `tool_input.command`, `tool_response.stdout`) to `normalize_agent_event` for `None`/`claude-code`/`cursor` and assert a non-empty path + content.

**Acceptance Scenarios**:

1. **Given** a Bash event `cat /path/report.csv` with stdout containing values, **When** the hook normalizes it for claude-code/cursor, **Then** it captures the path `/path/report.csv` and the stdout as content.
2. **Given** a Bash event for `head -n 5 file.txt`, **When** normalized, **Then** the path is `file.txt` (flags skipped) and stdout captured.
3. **Given** a native `Read`-tool event, **When** normalized, **Then** behavior is unchanged (default dialect still used).

---

### User Story 2 - RTK's `rtk read` Rewrite Is Recognized (Priority: P1)

When RTK is installed alongside RGT (the documented complementary pairing), its PreToolUse hook rewrites `cat file` into `rtk read file` *before* RGT sees the event. Today RGT's command parser only checks the first token against the read-style list, so `rtk` (not in the list) yields no path. The parser must see through the `rtk` wrapper and extract the path from the second token.

**Why this priority**: §8.3 confirmed the rewrite happens live; without this, even the dialect fix wouldn't capture reads on RTK-paired setups.

**Independent Test**: Unit test `extract_path_from_command("rtk read /path/file.txt")` returns the path; `extract_path_from_command("cat /path/file.txt")` still works.

**Acceptance Scenarios**:

1. **Given** `rtk read /path/file.txt`, **When** the path is extracted, **Then** it returns `/path/file.txt`.
2. **Given** `rtk cat /path/file.txt` (not a read subcommand), **When** extracted, **Then** it returns no path (RTK's read subcommand is `read`, not `cat`).

---

### User Story 3 - Captured Output Is Used, With a Disk Fallback (Priority: P2)

When a Bash read has `tool_response.stdout`, that output is used as the captured content (matching what the agent actually saw). When a Bash command yields no stdout content (or a non-read command yields a path only), the existing disk-read fallback still applies. RGT remains strictly passive — it never rewrites, filters, or blocks the agent's command.

**Why this priority**: §8.4 item 3 — using stdout avoids an unnecessary disk re-read and captures the exact bytes the agent saw; the fallback preserves the current behavior for path-only cases.

**Independent Test**: Unit tests assert stdout is captured as content when non-empty; a path-only Bash event still yields a path (with `content: None`) so the disk fallback can act.

**Acceptance Scenarios**:

1. **Given** a Bash read with non-empty stdout, **When** normalized, **Then** `content` is the stdout.
2. **Given** a Bash read with empty/absent stdout, **When** normalized, **Then** `content` is `None` and `path` is set (disk fallback).
3. **Given** any Bash event, **When** the hook runs, **Then** the command is never altered (RGT stays passive).

---

### Edge Cases

- `grep` stdout is the matching lines, not the whole file — is that captured as content, or should grep be path-only?
- `head`/`tail` stdout is a slice — recording it as content records partial data; is that acceptable (it's what the agent saw)?
- A command with a quoted path containing spaces (`cat "my file.csv"`)?
- `rtk read --some-flag file.txt` (wrapper with flags before the path)?
- A non-read Bash command (e.g. `ls`, `git status`) — must remain a no-op.
- `python script.py` vs `python -c "..."` — which token is the path?
- An empty `tool_input.command` or a Bash event with neither `command` nor `stdout`.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: For `None`, `claude-code`, and `cursor`, `normalize_agent_event` MUST try the default (`Read`-shaped) dialect first and, when that yields no path, fall back to command-based extraction so Bash tool calls are captured.
- **FR-002**: `extract_path_from_command` MUST recognize `rtk` as a wrapper — when the first token is `rtk`, the second token is checked against the read-style list — and MUST extract the path from behind the wrapper. It MUST NOT treat `rtk` with a non-read subcommand as a read.
- **FR-003**: Command-based extraction MUST capture `tool_response.stdout` as the content when present and non-empty, and MUST still return a path (with `content: None`) when stdout is absent so the existing disk-read fallback applies.
- **FR-004**: RGT's hook MUST remain strictly passive: it records data only and never rewrites, filters, or blocks the agent's Bash command (constitution VIII).

### Key Entities *(include if feature involves data)*

- **BashCommand**: The `tool_input.command` string; its first (or, under `rtk`, second) token selects the read-style path extraction.
- **ReadStyle**: The recognized read verbs (`cat`, `read`, `less`, `head`, `tail`, `more`, `python`, `python3`, and now `grep`).
- **NormalizedCapture**: The `{path, content}` model; `content` is now populated from `tool_response.stdout` for Bash reads.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The §8.1 Bash payload shape normalizes to a non-empty path and content for `None`/`claude-code`/`cursor` (unit-tested).
- **SC-002**: `extract_path_from_command("rtk read /p/f")` returns `/p/f`; `rtk cat /p/f` returns nothing (unit-tested).
- **SC-003**: A Bash read with non-empty stdout captures it as content; with empty stdout, content is `None` and path is set (unit-tested).
- **SC-004**: A native `Read` event and a non-read Bash command are unchanged / no-op respectively (unit-tested).
- **SC-005**: No hook path alters the agent's command (asserted via the existing "record data only" tests).

## Assumptions

- The Bash read verbs are `cat`, `read`, `less`, `head`, `tail`, `more`, `python`, `python3`, plus `grep`. `head`/`tail`/`grep` produce partial content; RGT records the captured stdout as-is (it is what the agent saw), which is the documented behavior — not a correctness bug.
- `rtk`'s read subcommand is `read` (not `cat`), per RTK's own README; the wrapper check is `rtk read`, and other `rtk <subcommand>` invocations are not read captures.
- No new dependencies; the existing `serde_json` parsing and the current `NormalizedCapture` model are extended in place.
- The hook runtime (`handle_passive_hook_event`) already falls back to reading the file from disk when `content` is `None`; this feature only widens what the parser recognizes.
- Out of scope: per-agent payload shapes for the other 11 agents (already handled by their existing dialects), and any change to RTK itself.
