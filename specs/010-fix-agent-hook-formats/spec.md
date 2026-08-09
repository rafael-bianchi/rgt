# Feature Specification: Fix Agent Hook Formats (RTK-Informed)

**Feature Branch**: `fix/010-agent-hook-formats`

**Created**: 2026-08-02

**Status**: Draft

**Input**: "Fix broken hook configs using RTK's approach as reference. RTK supports 9 agents in 3 tiers: full hook (Claude Code, Cursor, Copilot, Gemini), plugin (OpenCode, Hermes, Pi), rules-file (Cline, Windsurf, Codex)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Claude Code Hook Works (Priority: P1)

A developer runs `rgt init -g --agent claude-code`. The installer writes a valid hook configuration to `.claude/settings.json` using the same schema RTK uses (per the official Claude Code docs). The hook intercepts `PreToolUse` and `PostToolUse` events. The old `.claude/hooks.json` is not created.

**Why this priority**: Claude Code is the most popular AI coding tool. The current configuration writes to a file Claude Code never reads with a schema it doesn't recognize. RTK's working implementation confirms the correct approach.

**Independent Test**: Run `rgt init -g --agent claude-code`. Verify `.claude/settings.json` exists with `"hooks"` key containing `PostToolUse` and `PreToolUse` arrays. Verify `.claude/hooks.json` does NOT exist.

**Acceptance Scenarios**:

1. **Given** a clean home directory, **When** `rgt init -g --agent claude-code` runs, **Then** `.claude/settings.json` is created with the hooks block using the official schema: `{"hooks": {"PostToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook post"}]}], "PreToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook pre"}]}]}}`.
2. **Given** `.claude/settings.json` already exists with other settings (e.g., `permissions`, `enableAllProjectMcpServers`), **When** `rgt init -g --agent claude-code` runs, **Then** the RGT hook entries are merged into the existing JSON — other keys are preserved, only `hooks` is extended.
3. **Given** the old `.claude/hooks.json` exists from a previous broken installation, **When** `rgt init -g --agent claude-code --force` runs, **Then** the old file is removed and `.claude/settings.json` is written.

---

### User Story 2 - Cursor Hook Works (Priority: P1)

A developer runs `rgt init -g --agent cursor`. The installer writes a valid hook configuration to `.cursor/hooks.json` using the same format RTK uses (per current `cursor.com/docs/hooks`). The hook uses `preToolUse` and `postToolUse` with `"version": 1`.

**Why this priority**: Cursor is the second most popular agent. The current configuration uses PascalCase event names and a wrong schema. RTK's working Cursor integration confirms the correct approach.

**Independent Test**: Run `rgt init -g --agent cursor`. Verify `.cursor/hooks.json` contains `"version": 1`, camelCase event names (`preToolUse`, `postToolUse`), and `"matcher": "Shell"`.

**Acceptance Scenarios**:

1. **Given** a clean home directory, **When** `rgt init -g --agent cursor` runs, **Then** `.cursor/hooks.json` is created with `{"version": 1, "hooks": {"preToolUse": [{"command": "rgt hook pre", "matcher": "Shell"}], "postToolUse": [{"command": "rgt hook post", "matcher": "Shell"}]}}`.
2. **Given** `.cursor/hooks.json` already exists, **When** `rgt init -g --agent cursor --force` runs, **Then** the RGT hook entries replace the existing `preToolUse` and `postToolUse` entries.

---

### User Story 3 - Windsurf Integration Fixed (Priority: P3)

**RTK discovery**: RTK treats Windsurf as a **rules-file integration** (`.windsurfrules`), not a hook-based integration. The current RGT installer tries to write hooks JSON to `.codeium/windsurf/hooks.json`, which is wrong — Windsurf doesn't have a hooks API.

The Windsurf integration is changed from hook-based to rules-file-based, following RTK's pattern. A `.windsurfrules` file is placed in the project or the hook config is replaced with a prompt-level instruction.

**Why this priority**: Windsurf is a rules-file agent. The current hook JSON is dead code — Windsurf never reads it. Fixing this turns a broken integration into a functioning one, or removes it if the product is no longer available.

**Independent Test**: Run `rgt init -g --agent windsurf`. Verify no `.codeium/windsurf/hooks.json` is written. Verify a `.windsurfrules` rules file is created instead (or the agent is removed with a clear message if Windsurf is unavailable).

**Acceptance Scenarios**:

1. **Given** RTK's Windsurf approach is validated, **When** `rgt init -g --agent windsurf` runs, **Then** a `.windsurfrules` file is created with RGT usage instructions (not hook JSON).
2. **Given** Windsurf is confirmed unavailable/rebranded, **When** `rgt init -g --agent windsurf` runs, **Then** the installer prints a message: "Windsurf integration uses rules-file guidance. See https://github.com/rafael-bianchi/rgt for updates."

---

### User Story 4 - Codex CLI Integration Fixed (Priority: P3)

**RTK discovery**: RTK treats Codex CLI as a **rules-file/instructions integration** (`AGENTS.md`), not a hook-based integration. The current RGT installer writes hook JSON to `.codex/hooks.json`, which Codex never reads.

The Codex CLI integration is changed from hook-based to instructions-based. An `AGENTS.md` snippet or `.codex/config.json` guidance is provided.

**Why this priority**: Codex CLI doesn't support hooks. The current hook JSON is dead code. RGT already has an `AGENTS.md` file that could include Codex guidance.

**Independent Test**: Run `rgt init -g --agent codex`. Verify no `.codex/hooks.json` with hook commands is written. Verify either an `AGENTS.md` entry or a `.codex/` config file is created with instructions for using `rgt`.

**Acceptance Scenarios**:

1. **Given** RTK's Codex approach is validated, **When** `rgt init -g --agent codex` runs, **Then** an `AGENTS.md` or `.codex/config.json` entry is created with `rgt` usage instructions (not hook JSON).

---

### Edge Cases

- What happens when `.claude/settings.json` already contains a `hooks` key from other plugins? The installer must merge RGT entries into the existing `hooks` object — adding `PreToolUse`/`PostToolUse` arrays without removing other event entries.
- What happens when `rgt init -g` runs with no `--agent` flag? All 4 agent configs must be written. Rules-file agents (Windsurf, Codex) must not block or error — they just write files differently.
- What about the `--force` flag? It must replace old configurations with corrected versions.
- Integration tests currently assert on the old, broken formats. They must be updated to assert on the corrected formats.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The installer MUST write Claude Code hooks to `.claude/settings.json` (not `.claude/hooks.json`), using the schema `{"hooks": {"PostToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook post"}]}], "PreToolUse": [{"matcher": "", "hooks": [{"type": "command", "command": "rgt hook pre"}]}]}}`.
- **FR-002**: The installer MUST write Cursor hooks to `.cursor/hooks.json` using camelCase event names (`preToolUse`, `postToolUse`), `"version": 1`, `"matcher": "Shell"`, and per-entry `"command"` in an array.
- **FR-003**: The Windsurf integration MUST be changed from hook JSON to a rules-file approach (`.windsurfrules`), following RTK's confirmed working pattern.
- **FR-004**: The Codex CLI integration MUST be changed from hook JSON to an instructions-based approach (`AGENTS.md` or `.codex/config.json` entry), following RTK's confirmed working pattern.
- **FR-005**: When `.claude/settings.json` already exists, the installer MUST merge RGT hook entries without overwriting other top-level keys.
- **FR-006**: Integration tests MUST be updated to assert on the corrected hook formats for all agents.

### Key Entities

- **Hook Configuration (Full Hook Tier)**: A JSON settings file in an agent-specific directory defining commands the agent invokes at `PreToolUse`/`PostToolUse`. Used for Claude Code (`.claude/settings.json`) and Cursor (`.cursor/hooks.json`).
- **Rules File (Rules-File Tier)**: A project-level instruction file the agent reads as guidance. Used for Windsurf (`.windsurfrules`) and Codex CLI (`AGENTS.md` or `.codex/config.json`). Contains text instructions rather than JSON hook definitions.
- **Agent Tier**: RTK's classification for integration complexity — Full Hook (intercept + rewrite commands), Plugin (agent's plugin system), Rules File (prompt-level instructions).

### Files Requiring Changes

| File | Change |
|---|---|
| `src/hooks/installer.rs` | Primary: fix all 4 agent configs per RTK-verified formats |
| `tests/integration/test_hooks_installer.rs` | Update hook format assertions |
| `src/cli/init.rs` | Update `--agent` help text if agents change tiers |

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `rgt init -g --agent claude-code` creates `.claude/settings.json` with the correct schema and does NOT create `.claude/hooks.json`.
- **SC-002**: `rgt init -g --agent cursor` creates `.cursor/hooks.json` with `"version": 1`, camelCase event names, and `"matcher": "Shell"`.
- **SC-003**: `rgt init -g --agent windsurf` does NOT create `.codeium/windsurf/hooks.json` — it creates a rules file or prints a clear status message instead.
- **SC-004**: `rgt init -g --agent codex` does NOT create `.codex/hooks.json` with hook commands — it creates instructions-based guidance instead.
- **SC-005**: Integration tests pass with assertions on all four corrected formats.
- **SC-006**: Existing `.claude/settings.json` files with non-hook settings are preserved after RGT hook installation.

## Assumptions

- The RTK `hooks/` directory approach is the authoritative reference for working agent integrations. Their Claude Code, Cursor, Windsurf, and Codex integrations are confirmed working in production.
- Windsurf is still available as a product (despite docs 404). If it's been fully replaced by Devin Desktop, the integration should be removed rather than converted to rules-file.
- Codex CLI reads `AGENTS.md` from the project root, which is where RGT already places its `AGENTS.md` file. The Codex integration may simply need a section in the existing `AGENTS.md`.
- The `rgt hook pre` and `rgt hook post` commands work correctly — only the configuration format is wrong. The hook binary itself does not need changes.
