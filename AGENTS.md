# RGT Agents Instructions

## RGT Integration

RGT tracks numeric and date provenance for AI coding agents.

RGT configures provenance-capture hooks for all 13 RTK-supported agents: Claude Code (`claude-code`/`claude`), Cursor (`cursor`), Copilot (`copilot`), Gemini CLI (`gemini`), Mistral Vibe (`vibe`), OpenCode (`opencode`), Pi (`pi`), Hermes (`hermes`), Windsurf (`windsurf`), Codex CLI (`codex`), Cline/Roo Code (`cline`/`roo-code`), Antigravity (`antigravity`), and Kilo (`kilocode`/`kilo`). Hooks record data only and never rewrite or block tool commands.

### Recording Values
After reading a data file containing numbers or dates, record its values:
- `rgt record <file>`: extracts and tracks numeric/date values from file content
- `rgt record <file> --number-format <us|eu|auto>`: parse numbers with a specific locale (`.`/`,` separator convention); default is the project's persisted setting, then `auto`
- Run `rgt status` to check what's tracked

### Recording Derivations
After computing a derived value from tracked root nodes:
- `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression "a - b" --result <val>`
- The derivation is verified before recording; wrong results are rejected (exit 1)

### Inspecting the Graph
- `rgt status`: see all tracked nodes and staleness state
- `rgt query <node_id>`: trace provenance lineage for a value
- `rgt graph`: export the dependency graph as text or mermaid
- `rgt doctor`: diagnose hook installation and capture health (0 = healthy/warnings, 1 = error)

> **Copilot CLI note**: RGT's Copilot CLI event-shape handling is an assumption
> pending live verification against the real tool — the dual-dialect parser is
> pinned by tests, so a future divergence is a deliberate, tested change.

Run `rgt --help` for all available commands.
