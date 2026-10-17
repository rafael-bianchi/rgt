# Research: Fix Agent Hook Formats

**Feature**: `010-fix-agent-hook-formats` | **Date**: 2026-08-02

## Decision 1: Claude Code Hook Format

**Decision**: Write to `.claude/settings.json` (not `.claude/hooks.json`) using the official Claude Code schema. Merge into existing settings rather than overwriting.

**Rationale**:
- Claude Code official docs (`docs.anthropic.com/en/docs/claude-code/hooks`) specify the file is `.claude/settings.json` and the schema is `{"hooks": {"EventName": [{"matcher": "...", "hooks": [{"type": "command", "command": "..."}]}]}}`.
- RTK (`rtk-ai/rtk`) uses the exact same format in production — their `hooks/claude/` directory confirms `.claude/settings.json` patching works.
- Settings must be merged because `.claude/settings.json` may already contain `permissions`, `enableAllProjectMcpServers`, or other keys set by the user or other plugins.

**Alternatives reconsidered**: `hooks.json` with flat schema — confirmed non-functional per Claude Code docs.

## Decision 2: Cursor Hook Format

**Decision**: Write to `.cursor/hooks.json` with `"version": 1`, camelCase event names (`preToolUse`, `postToolUse`), `"matcher": "Shell"`, and per-entry `"command"` in an array.

**Rationale**:
- Cursor official docs (`cursor.com/docs/hooks`) specify `"version": 1` and camelCase event names (`preToolUse`, `postToolUse`). Each hook entry requires a `matcher` field.
- RTK's Cursor integration confirms the working format — their `hooks/cursor/README.md` references the same schema.
- The old format used `{"hooks": [{"event": "PostToolUse", "command": "..."}]}` — this is an older or incorrect schema that Cursor no longer recognizes.

**Alternatives reconsidered**: PascalCase array-of-objects format — confirmed non-functional per Cursor docs.

## Decision 3: Windsurf Integration — Rules-File Tier

**Decision**: Convert Windsurf from hook-based (`.codeium/windsurf/hooks.json`) to rules-file based (`.windsurfrules`), following RTK's confirmed approach.

**Rationale**:
- RTK's `hooks/windsurf/` directory confirms Windsurf is a **rules-file integration** (`.windsurfrules`), NOT a hook-based integration. The current RGT installer writes hook JSON that Windsurf never reads.
- Windsurf's documentation site (`docs.codeium.com/windsurf`, `docs.windsurf.com`) returns 404. The product may have been rebranded. Until confirmed otherwise, follow RTK's working pattern.
- Rules-file integrations are simpler — no JSON schema to maintain, no API drift risk.

**Alternatives considered**:
1. **Keep hooks JSON**: Non-functional — Windsurf doesn't have a hooks API.
2. **Remove Windsurf entirely**: Too aggressive — product may still be available under a new name. Rules-file is a safe fallback.

## Decision 4: Codex CLI Integration — Instructions Tier

**Decision**: Convert Codex CLI from hook-based (`.codex/hooks.json`) to instructions-based (`AGENTS.md` entry), following RTK's confirmed approach.

**Rationale**:
- RTK's `hooks/codex/` directory confirms Codex CLI is an **instructions-based integration** using `AGENTS.md`, NOT a hook-based integration. The current RGT installer writes `{"rgt_hook": "rgt hook post"}` to `.codex/hooks.json` — a format Codex never reads.
- RGT already has an `AGENTS.md` file in the project root. The Codex integration can reference or add to this file.
- Codex CLI GitHub README makes no mention of hooks — it reads instructions from `AGENTS.md`, `CONTEXT.md`, and `$CODEX_HOME` configurations.

**Alternatives considered**:
1. **Keep hooks JSON**: Non-functional — Codex CLI doesn't have a hooks API.
2. **Remove Codex entirely**: Codex CLI is actively maintained. Following RTK's instructions-based approach is a working integration.

## Decision 5: JSON Merging for Claude Code Settings

**Decision**: When `.claude/settings.json` already exists, parse the JSON, add/update the `hooks` key, and write back. If the `hooks` key exists, merge RGT's `PreToolUse`/`PostToolUse` entries into the existing hooks object without removing other event entries.

**Rationale**:
- Users may have other hooks configured (formatting, notifications, etc.). Overwriting `.claude/settings.json` would destroy their configuration.
- Merging only the `hooks.PreToolUse` and `hooks.PostToolUse` keys preserves user settings while adding RGT hooks.
- If the file doesn't exist or is malformed, create a fresh file.

**Alternatives considered**:
1. **Always overwrite**: Destroys user settings. Rejected.
2. **Skip if exists**: Never installs hooks if user has any settings. Rejected.

## Decision 6: Integration Tier Model

**Decision**: Adopt RTK's three-tier model for agent integrations, but only implement the two tiers relevant to RGT's current agent set.

**Tiers** (from RTK):
| Tier | Mechanism | Agents |
|---|---|---|
| Full Hook | Shell script in `~/.claude/settings.json` or `~/.cursor/hooks.json` | Claude Code, Cursor |
| Rules-File | Instruction file in project root | Windsurf, Codex CLI |
| Plugin | Agent-specific plugin system | (none yet) |

**Rationale**:
- Aligns with RTK's proven approach — they support 9 agents across these tiers.
- Clarifies maintenance expectations: Full Hook tier requires tracking agent API changes, Rules-File tier is low-maintenance.
- The tier classification should be reflected in the `--agent` help text and output messages.
