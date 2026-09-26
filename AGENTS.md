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

For temporal values, prefer typed operations so the graph preserves Duration semantics:
- `rgt derive --parents <date1>,<date2> --operation DATE_DIFF [--unit days]`: computes the ordered elapsed gap automatically; parent order is `date2 - date1`
- `rgt derive --parents <duration1>,<duration2> --operation DURATION_SUM|DURATION_AVG [--unit minutes]`: computes a typed Duration from 2–26 distinct Duration parents
- Optional `--result <value> --result-unit <seconds|minutes|hours|days|weeks>` checks a claim before writing; claims default to seconds. Stored values remain exact whole seconds, and claims or averages that require fractional seconds are rejected without rounding.
- The fixed elapsed-time units are seconds, minutes, hours, days, and weeks. Calendar months and years are unsupported because their lengths vary. Claim and display units are transient: neither is persisted as node metadata; the graph stores exact seconds.
- Legacy `EXPRESSION` with Date or Duration parents remains numeric: Dates are Unix timestamp seconds, Durations are elapsed seconds, and the result is a Number. RGT emits a warning; use `DATE_DIFF` or typed Duration operations when a Duration result is intended.

### Inspecting the Graph
- `rgt status`: see all tracked nodes and staleness state
- `rgt query <node_id>`: trace provenance lineage for a value
- `rgt query <duration_node_id> --unit days [--json]`: redisplay a Duration without changing its stored value; the requested lineage entry keeps `value` and adds exact `duration_seconds` (decimal string) plus `selected_unit_display`. Read `duration_seconds` for calculations, not the rounded display decimal.
- `rgt graph`: export the dependency graph as text, mermaid, DOT, or Turtle
- `rgt graph --format ttl [--include-absolute-paths]`: export project-scoped PROV-O Turtle; absolute source paths are opt-in and sensitive
- `rgt gc [--vacuum]`: remove obsolete (stale, dependent-free) nodes
- `rgt doctor`: diagnose hook installation and capture health (0 = healthy/warnings, 1 = error)

Turtle export is bounded to 10,000 values, 30,000 derivation edges, 64 MiB,
and a one-second command deadline. Its RDF union keeps separate projects'
same-spelled local node IDs distinct. Capture attribution identifies which of
the eight event-capable hook/plugin agents recorded a value; it does not mean
the agent authored the source or calculated a derivation.

> **Copilot CLI note**: RGT's Copilot CLI event-shape handling is an assumption
> pending live verification against the real tool — the dual-dialect parser is
> pinned by tests, so a future divergence is a deliberate, tested change.

Run `rgt --help` for all available commands.
