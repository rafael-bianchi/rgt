# Agent Hook Format Contracts

**Feature**: `010-fix-agent-hook-formats` | **Date**: 2026-08-02

## Overview

RGT installs agent hook configurations via `rgt init -g [--agent <name>]`. Each agent uses a different file, schema, and integration tier. This contract defines the correct format for each supported agent, validated against official documentation and RTK's working production code.

## Integration Tiers

| Tier | Mechanism | Maintenance | Agents |
|---|---|---|---|
| Full Hook | JSON config in agent-specific directory, agent invokes shell command on tool events | Must track agent API changes | Claude Code, Cursor |
| Rules-File | Plain text instruction file in project root, agent reads as guidance | Low — no API surface | Windsurf, Codex CLI |

## Full Hook Contracts

### Claude Code

**File**: `~/.claude/settings.json` (global) or `.claude/settings.json` (project)

**Schema**:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "rgt hook post"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "rgt hook pre"
          }
        ]
      }
    ]
  }
}
```

**Merge rule**: If the file exists, parse as JSON, ensure `hooks` key is an object, set `hooks.PostToolUse` and `hooks.PreToolUse` to the RGT entries. Other keys (`permissions`, `enableAllProjectMcpServers`, etc.) must be preserved.

**Cleanup**: On `--force`, remove `.claude/hooks.json` if it exists (old broken format).

**Reference**: `docs.anthropic.com/en/docs/claude-code/hooks`, `rtk-ai/rtk hooks/claude/`

### Cursor

**File**: `~/.cursor/hooks.json` (global) or `.cursor/hooks.json` (project)

**Schema**:
```json
{
  "version": 1,
  "hooks": {
    "preToolUse": [
      {
        "command": "rgt hook pre",
        "matcher": "Shell"
      }
    ],
    "postToolUse": [
      {
        "command": "rgt hook post",
        "matcher": "Shell"
      }
    ]
  }
}
```

**Merge rule**: On `--force`, replace the file. Without `--force`, skip if the file already exists.

**Reference**: `cursor.com/docs/hooks`, `rtk-ai/rtk hooks/cursor/`

## Rules-File Contracts

### Windsurf

**File**: `.windsurfrules` (project root)

**Content**: Plain text instructions, for example:
```
# RGT Integration
When executing shell commands such as git, cargo, npm, ls, grep,
use the `rgt` prefix for optimized, token-efficient output.
Learn more: https://github.com/rafael-bianchi/rgt
```

**Merge rule**: On `--force`, create or replace. Without `--force`, skip if the file already exists.

**Reference**: `rtk-ai/rtk hooks/windsurf/`

### Codex CLI

**File**: `AGENTS.md` section (project root) or `.codex/config.json` (global)

**Content**: A section appended to the project's `AGENTS.md`:
```markdown
## RGT Integration

For shell commands, use `rgt` prefix to get optimized output:
- `rgt git status`, `rgt cargo test`, `rgt ls`, `rgt grep`
- Run `rgt --help` for supported commands
```

**Merge rule**: If `AGENTS.md` exists, append the RGT section. If not, create it with the RGT section. On `--force`, overwrite the RGT section if it already exists.

**Reference**: `rtk-ai/rtk hooks/codex/`

## CLI Contract

### `rgt init -g [--agent <name>]`

| `--agent` value | Tier | Behavior |
|---|---|---|
| `claude-code` | Full Hook | Writes/merges `.claude/settings.json` |
| `cursor` | Full Hook | Writes `.cursor/hooks.json` |
| `windsurf` | Rules-File | Writes `.windsurfrules` |
| `codex` | Rules-File | Writes `AGENTS.md` section |
| (omitted) | All | Configures all 4 agents |

**Exit codes**: `0` on success or when files already exist (without `--force`). Non-zero on filesystem errors only.

**Output**: Prints one line per configured agent with the tier classification:
```
✓ Auto-configured AI coding agent hooks:
  Claude Code (full hook) -> ~/.claude/settings.json
  Cursor (full hook) -> ~/.cursor/hooks.json
  Windsurf (rules-file) -> .windsurfrules
  Codex CLI (rules-file) -> AGENTS.md
```
