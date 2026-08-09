# Data Model: Fix Agent Hook Formats

**Feature**: `010-fix-agent-hook-formats` | **Date**: 2026-08-02

## Overview

This feature changes the file format and location of agent hook configurations. No database schema changes. The entities below describe the configuration files before and after the fix.

## Entities

### Claude Code Hook Config

**File**: `.claude/settings.json` (was: `.claude/hooks.json`)

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

**Merge behavior**: If `settings.json` already exists, parse the JSON, add/update the `hooks` key. Other top-level keys (`permissions`, `enableAllProjectMcpServers`, etc.) are preserved.

**Cleanup**: If `.claude/hooks.json` exists from a previous broken installation, it is removed when `--force` is used.

### Cursor Hook Config

**File**: `.cursor/hooks.json` (unchanged location, changed content)

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

**Changes from old format**:
- Added `"version": 1`
- Event names: `PreToolUse` → `preToolUse`, `PostToolUse` → `postToolUse`
- Schema: flat array of `{event, command}` → nested object with `{command, matcher}` per entry

### Windsurf Rules File

**File**: `.windsurfrules` (was: `.codeium/windsurf/hooks.json`)

**Content** (example):
```
# RGT Integration
When executing shell commands, use `rgt` prefix for optimized output.
Example: `rgt git status`, `rgt cargo test`, `rgt ls`.
Run `rgt --help` for available commands.
```

**Purpose**: Project-level instructions the Windsurf agent reads as guidance. No JSON schema — plain text.

### Codex CLI Instructions

**File**: `AGENTS.md` entry (was: `.codex/hooks.json`)

**Content**: A section in the project's `AGENTS.md` file:
```markdown
## RGT Integration

Use `rgt` prefix for shell commands to get optimized, token-efficient output.
- `rgt git status` instead of `git status`
- `rgt cargo test` instead of `cargo test`
- `rgt ls` instead of `ls`
```

**Purpose**: Codex CLI reads `AGENTS.md` as instructions. Adding an RGT section makes Codex aware of the `rgt` command without needing a hook system.

## State Transitions

```
BEFORE                          AFTER
──────                          ─────
.claude/hooks.json (broken)  →  .claude/settings.json (correct)
.cursor/hooks.json (wrong)   →  .cursor/hooks.json (corrected)
.codex/hooks.json (dead)     →  AGENTS.md entry (instructions)
.codeium/windsurf/hooks.json →  .windsurfrules (rules-file)
 (dead)
```

## Downstream Consumers

| Consumer | Impact |
|---|---|
| Claude Code agent | Reads `.claude/settings.json` — hook fires on PreToolUse/PostToolUse |
| Cursor agent | Reads `.cursor/hooks.json` — hook fires on preToolUse/postToolUse |
| Windsurf agent | Reads `.windsurfrules` — uses rules as guidance, no hook events |
| Codex CLI agent | Reads `AGENTS.md` — uses instructions for shell command rewriting |
| `cargo test test_hooks_installer` | Asserts on corrected formats |
