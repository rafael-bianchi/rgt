# Contracts: Agent Hook Stdin Formats & Install Artifacts

**Phase 1 output** | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

This file defines the two interface contracts of this feature: (1) the stdin JSON dialect each agent's hook delivers to `rgt hook pre/post`, and (2) the install artifacts `rgt init --agent <name>` writes.

## 1. `rgt hook` CLI Contract

### Signature

```text
rgt hook <pre|post> [--agent <name>]
```

- Reads one JSON object from stdin.
- `--agent` selects the input dialect. Omitted → `default` (Claude Code compatible).
- Always exits 0. On parse/processing failure: no stdout, exit 0 (fail-open, FR-005).
- Never modifies, denies, or rewrites the agent's tool call (no command rewriting, clarified scope).

### Dialects (normalizer → internal `{path, content}`)

**default** (Claude Code / Cursor / Copilot VS Code Chat snake_case):

```json
{
  "tool_name": "Read",
  "tool_input": { "path": "/abs/path.json", "file_path": "/abs/path.json" },
  "tool_response": { "content": "{\"count\": 42}" }
}
```

**copilot** (dual): VS Code Chat (snake_case, above) or Copilot CLI:

```json
{ "toolName": "read_file", "toolArgs": "{\"file_path\": \"/abs/path.json\"}" }
```

`toolArgs` is JSON-stringified. Normalizer: if `toolArgs` present, parse it and look for `path`/`file_path`/`command`.

**gemini**:

```json
{ "tool_name": "run_shell_command", "tool_input": { "command": "cat /abs/path.json" } }
```

Normalizer: `path` = first path argument of a read-style command (`cat`, `read`, `less`, `head`, `tail`, `python <file>`); content fallback reads the file.

**vibe**:

```json
{
  "tool_name": "bash",
  "tool_input": { "command": "cat /abs/path.json" },
  "hook_event_name": "pre_tool",
  "session_id": "…"
}
```

Normalizer: as gemini; non-bash tool / empty command / malformed JSON → no-op.

**opencode / pi / hermes** (plugins): plugin glue receives the agent's native event, extracts the command/path, and calls `rgt hook post --agent <same-name>` piping the event or an extracted `{tool_input}` JSON. Normalizer: same as vibe/command fallback.

### Normalizer output

| Field | Rule |
|-------|------|
| path | `tool_input.path` → `file_path` → command-path extraction → `null` |
| content | `tool_response.content`, else disk-read fallback when path is valid |

## 2. Install Artifacts (`rgt init --agent <name>`)

| Agent | Tier | Artifact Written | Location (project / `--global`) |
|-------|------|------------------|--------------------------------|
| claude-code | hook | `settings.json` hooks entry (`PreToolUse`/`PostToolUse` → `rgt hook pre/post`) | `.claude/` / `~/.claude/` |
| cursor | hook | `hooks.json` (pre/post tool hooks) | `.cursor/` / `~/.cursor/` |
| codex | rules | `AGENTS.md` RGT section (append) | `AGENTS.md` (project) |
| windsurf | rules | `.windsurfrules` RGT section | `.windsurfrules` (project) |
| copilot | hook | VS Code Copilot Chat hooks config (`github.copilot.chat.hooks` → `rgt hook post --agent copilot`) | user settings JSON (per-platform) |
| gemini | hook | `hooks.toml` entry → `rgt hook post --agent gemini` | `~/.gemini/hooks.toml` |
| vibe | hook | `hooks.toml` `pre_tool` entry → `rgt hook post --agent vibe` + `prompts/rgt.md` | `~/.vibe/` |
| opencode | plugin | `rgt.ts` TS plugin (calls `rgt hook post --agent opencode`) | `~/.config/opencode/plugins/` / project `.opencode/plugin/` |
| pi | plugin | `rgt.ts` TS extension (calls `rgt hook post --agent pi`) | `.pi/extensions/` / `~/.pi/agent/extensions/` |
| hermes | plugin | `rgt/` Python plugin (calls `rgt hook post --agent hermes`) + `plugins.enabled` entry | `~/.hermes/plugins/` / project plugin dir |
| cline / roo-code | rules | `.clinerules` RGT section | `.clinerules` (project) |
| antigravity | rules | `antigravity-rgt-rules.md` | `.agents/rules/` (project) |
| kilocode | rules | `rgt-rules.md` | `.kilocode/rules/` (project) |

### Artifact write contract

- **Merge, never clobber** (FR-006): JSON configs merge the RGT hooks block into existing user JSON; rules files append a marked `## RGT Integration` section; plugin files are created if absent. `--force` overwrites.
- **Idempotent**: re-running produces identical content; RGT-marked sections are replaced in place, not duplicated.
- **Cross-platform**: paths resolved via `dirs`/`std::path` (Windows user settings under `%APPDATA%`).
