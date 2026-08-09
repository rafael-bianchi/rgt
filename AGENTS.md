# RGT Agents Instructions

## RGT Integration

RGT tracks numeric and date provenance for AI coding agents.

### Recording Values
After reading a data file containing numbers or dates, record its values:
- `rgt record <file>` — extracts and tracks numeric/date values from file content
- Run `rgt status` to check what's tracked

### Recording Derivations
After computing a derived value from tracked root nodes:
- `rgt derive --parents <id1>,<id2> --operation EXPRESSION --expression "a - b" --result <val>`
- The derivation is verified before recording; wrong results are rejected (exit 1)

### Inspecting the Graph
- `rgt status` — see all tracked nodes and staleness state
- `rgt query <node_id>` — trace provenance lineage for a value
- `rgt graph` — export the dependency graph as text or mermaid

Run `rgt --help` for all available commands.
