# Data Model: Expand Agent Hook Coverage to Match RTK

**Phase 1 output** | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

## Entities

### Agent

The unit of `--agent` targeting and auto-detection.

| Attribute | Type | Constraints / Notes |
|-----------|------|---------------------|
| name | string | Canonical `--agent` value. Values: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode` (13 values; 13 distinct agents) |
| aliases | string[] | `claude` ↔ `claude-code`; `cline` ↔ `roo-code`; `kilo` ↔ `kilocode`. Aliases normalize to the canonical name before any lookup. Accepted `--agent` spellings = 13 canonical + 3 aliases (`claude`, `roo-code`, `kilo`). |
| tier | enum: `hook`, `plugin`, `rules` | `hook` = agent invokes `rgt hook --agent` directly via its native hook API (claude-code, cursor, copilot, gemini, vibe). `plugin` = thin TS/Python glue invoking `rgt hook --agent` (opencode, pi, hermes). `rules` = prompt-level instruction file, no programmatic hook (codex, windsurf, cline/roo-code, antigravity, kilocode). |
| input_format | enum: `default`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes` | The stdin JSON dialect the Rust normalizer parses for this agent (only for `hook`/`plugin` tiers). `default` = Claude Code compatible (existing). |
| config_path | string (resolved) | Where the hook config/plugin/rules file is written. Resolved per host platform and `--global` vs project scope at init time. |
| detection_trigger | string | Host path whose presence indicates the agent is installed (for auto-detection, US2). |

### Capture Event (normalized)

The internal model produced by the normalizer and consumed by `handle_passive_hook_event`.

| Attribute | Type | Source |
|-----------|------|--------|
| path | string \| null | `tool_input.path` / `file_path`, or first path argument of a read-style `command` (command agents) |
| content | string \| null | `tool_response.content`; if null and `path` valid → read file from disk (fallback) |

### Detection Result (auto-detection output)

The set of agents found installed on the host during `rgt init` auto-detection (US2), each mapped to its hook configuration target.

| Attribute | Type | Notes |
|-----------|------|-------|
| agent | string | canonical name |
| config_path | string | resolved hook config / plugin / rules file target (same as `Agent.config_path`)

### Hook Configuration (artifact)

| Attribute | Type | Notes |
|-----------|------|-------|
| agent | string | canonical name |
| scope | enum: `project`, `global` | project-local vs `~/.config`-style global install |
| artifact_type | enum: `settings_json`, `hooks_json`, `rules_file`, `ts_plugin`, `py_plugin`, `toml_entry` | How the agent consumes it |
| content | string | Exact file content written (embedded glue template or generated JSON) |
| target_path | string | Absolute path written by `rgt init` |

## Validation Rules

- Agent name resolution: normalize aliases → canonical; unknown names fail with exit 2 + valid-name list (FR-001).
- Auto-detection: a supported agent is "installed" iff its `detection_trigger` path exists (US2).
- Idempotency: if the target artifact already exists and contains an RGT marker, do not rewrite unless `--force` (FR-006). Rules files append an RGT section only when absent (existing Codex/Windsurf pattern).
- Fail-open: normalizers return `None`/empty on any parse error; `handle_passive_hook_event` returns `Ok(())` and emits no stdout (FR-005).

## State Transitions

No persistent state changes — the existing `.rgt/store.db` schema is untouched. The only lifecycle is per-`rgt init` artifact write (create / idempotent-skip / `--force` overwrite).
