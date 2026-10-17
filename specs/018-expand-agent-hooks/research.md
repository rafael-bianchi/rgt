# Research: Expand Agent Hook Coverage to Match RTK

**Phase 0 output** | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Scope

Identify, for every agent RTK supports, the hook mechanism, installation location, and event JSON shape, so RGT can replicate coverage with thin delegates calling the RGT CLI. Source of truth: `rtk-ai/rtk` `hooks/` directory (develop) and its READMEs.

## RTK Agent Mechanism Matrix

| Agent | RTK Mechanism | Install Location (RTK) | Event Shape | RGT Tier |
|-------|---------------|------------------------|-------------|----------|
| Claude Code | Shell hook (PreToolUse) | `~/.claude/settings.json` | `tool_name`, `tool_input{path}`, `tool_response{content}` | Existing (unchanged) |
| Cursor | Shell hook | `~/.cursor/hooks.json` | same as Claude Code | Existing (unchanged) |
| Codex CLI | Rules file / AGENTS.md | `AGENTS.md` (project) | n/a (prompt-level) | Existing (unchanged) |
| Windsurf | Rules file | `.windsurfrules` (project) | n/a (prompt-level) | Existing (unchanged) |
| Copilot | Rust binary hook | VS Code user settings (Copilot Chat hooks); Copilot CLI config | dual: snake_case `tool_name`/`tool_input` (VS Code) or camelCase `toolName`/`toolArgs` JSON-stringified (CLI) | Event-capture |
| Gemini CLI | Rust binary hook | `~/.gemini/hooks.toml` | `tool_name: "run_shell_command"`, `tool_input.command` | Event-capture |
| Mistral Vibe | Rust binary hook | `~/.vibe/hooks.toml` (`pre_tool`, `match="bash"`, `strict=false`) + `~/.vibe/prompts/rtk.md` | `tool_name`, `tool_input.command`, `hook_event_name`, `session_id` | Event-capture |
| OpenCode | TS plugin (zx) | `~/.config/opencode/plugins/rtk.ts` | `tool.execute.before/after` → `args.command`, `args.session` | Event-capture (plugin) |
| Pi | TS extension (`pi.exec`) | `.pi/extensions/rtk.ts` (project) or `~/.pi/agent/extensions/rtk.ts` (global) | `tool_call` event → `event.input.command` | Event-capture (plugin) |
| Hermes | Python plugin | `~/.hermes/plugins/rtk-rewrite/` + `plugins.enabled` in Hermes config | `pre_tool_call` → `args["command"]` | Event-capture (plugin) |
| Cline / Roo Code | Rules file | `.clinerules` (project) | n/a (prompt-level) | Rules file |
| Antigravity | Rules file | `.agents/rules/antigravity-rtk-rules.md` (project) | n/a (prompt-level) | Rules file |
| Kilo | Rules file | `.kilocode/rules/rtk-rules.md` (project) | n/a (prompt-level) | Rules file |

## Key Decisions

### Decision: Event-capture agents reuse `rgt hook pre/post` with a `--agent` flag

- **Decision**: `rgt hook pre/post` gains an optional `--agent <name>` argument. When set, the Rust binary parses that agent's native stdin JSON and normalizes it to the internal `(path, content)` capture model. Without `--agent`, the current default (Claude Code format) is used — zero regression for existing configs (FR-004).
- **Rationale**: Keeps all parsing and provenance logic in Rust (constitution VIII, testable via `cargo test` — constitution IX). Agent configs invoke `rgt hook post --agent copilot` directly (no per-agent shell script needed, cross-platform on Windows).
- **Alternatives considered**: (a) per-agent shell/`jq` wrapper scripts — rejected: `jq` isn't guaranteed on Windows and adds a deployable artifact; (b) separate `rgt hook copilot` subcommands — rejected: duplicates the capture pipeline 9 times.

### Decision: Capture model uses `path` + `content`, with disk-read fallback

- **Decision**: The normalizer extracts a file `path` from the agent event (`path`/`file_path` fields, or the first path argument of a read-style `command`), and `content` from `tool_response.content` when present; when content is absent but a valid path exists, `handle_passive_hook_event` reads the file from disk as content fallback.
- **Rationale**: Command-based agents (Copilot CLI, Gemini, Vibe, plugins) don't embed file content in the event; reading the resolved path keeps provenance capture working across all event shapes.
- **Alternatives considered**: Recording nothing for command events — rejected: would make coverage nominal for 6 of 13 agents.

### Decision: Plugin/rules glue embedded in the binary

- **Decision**: TS plugins (OpenCode, Pi), the Python plugin (Hermes), and rules-file templates are embedded `const` strings in `src/hooks/glue/mod.rs` and written to agent paths by the installer at `rgt init` time.
- **Rationale**: Single-binary distribution preserved (constitution I); mirrors RTK's thin-delegate contract (glue only invokes the RGT CLI — constitution VIII).
- **Alternatives considered**: Shipping a `hooks/` directory — rejected (second distribution channel); prompting users to copy files manually — rejected (breaks one-command UX).

### Decision: Existing four integrations byte-identical

- **Decision**: Claude Code, Cursor, Codex, Windsurf installer code and `rgt hook` default format are untouched.
- **Rationale**: FR-004 / SC-003 (byte-identical baseline). New code is additive.
- **Alternatives considered**: Refactoring the installer into a generic registry — rejected for this feature (risks regressions; defer to a follow-up cleanup).

## Unknowns Resolved by Research

- Copilot install location and dual format → VS Code user settings hooks + CLI camelCase (`toolArgs` JSON-stringified).
- Vibe install → `~/.vibe/hooks.toml` `pre_tool` entry + system prompt fallback.
- OpenCode/Pi/Hermes plugin locations → confirmed from RTK READMEs.
- Cline/Antigravity/Kilo → prompt-level rules files only (no programmatic hook API).
