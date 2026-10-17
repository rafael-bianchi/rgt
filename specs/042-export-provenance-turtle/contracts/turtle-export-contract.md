# Contract: Turtle Graph Export

**Feature**: [Export Provenance as Turtle](../spec.md)

**Design**: [data-model.md](../data-model.md)

## CLI surface

```text
rgt graph --format ttl [--include-absolute-paths]
rgt graph -f ttl [--include-absolute-paths]
```

`ttl` is added to the existing graph-format selector. The optional flag is valid only with `ttl` and must be an explicit choice. Existing `text`, `mermaid`, and `dot` output remains unchanged, including the current text fallback for an unknown format. No separate merge command is added.

| Outcome | stdout | stderr | Exit |
|---------|--------|--------|------|
| Complete export | Valid UTF-8 Turtle, even for an empty project | Empty | 0 |
| Complete export with uncertain derivation | Valid Turtle with supported direct links and no uncertain activity | One actionable warning per omitted child activity | 0 |
| Any Turtle row or byte limit exceeded | Empty | Actionable error naming the exceeded limit | 1 |
| Store, validation, or render failure before output | Empty | Actionable error | 1 |
| One-second deadline reached before output | Empty | Actionable timeout error | 1 |
| Invalid flag combination | Empty | Usage error | 2 |
| Deadline or OS write failure after rendering begins writing | May contain an incomplete write | Actionable error; never claims a complete export | Nonzero |

The exporter completes its snapshot read, validation, and rendering before writing stdout. Warnings and errors never appear in Turtle content. An empty project produces a valid prefix-only document with no provenance triples. The one-second deadline starts at CLI entry and includes store access, snapshot, rendering, and final stdout writing; SQLite lock waiting and the final write must honor the remaining budget.

## Supported Turtle export limits

| Resource | Maximum |
|----------|---------|
| Recorded root and derived values combined | 10,000 |
| Derivation edges | 30,000 |
| Rendered UTF-8 Turtle, including prefixes | 64 MiB (67,108,864 bytes) |

The exporter checks the value and edge limits with bounded reads inside the export snapshot before loading the full graph and stops buffered rendering as soon as it would exceed the byte limit, always before stdout. Referenced sources are at most one per root; capture associations are at most one per value and each of the eight eligible agents, so neither needs a separate export quota. A project over any limit has no partial Turtle export. Recording, querying, and the existing graph formats have no new limit from this contract. A within-limit export that cannot finish within the command-wide deadline fails with the timeout semantics above.

## Release benchmark protocol

Use a standard, GitHub-hosted x86_64 `ubuntu-24.04` runner for the latency gate. Build with `cargo build --release --locked`, prepare the SQLite fixtures before timing, and run the release binary as a fresh process for each invocation. Redirect stdout to a new local file and measure from process spawn through successful exit and completed file output. Perform one untimed warm-up followed by ten measured runs per fixture; every measured run must finish in less than one second. Record the runner image version, CPU model, available memory, Rust version, fixture counts, rendered byte count, and all ten elapsed times. A failed or over-limit invocation must exit nonzero with the contract's stdout/stderr behavior in less than one second in every measured run.

The representative fixture has exactly 10,000 values: 4,000 number roots, 2,000 date roots, 2,000 numeric derivations, and 2,000 duration derivations. Each root has a distinct, short in-project source path and line 1, giving 6,000 referenced sources. Each derivation has two distinct ordered parents, giving 8,000 edges; number derivations use a fixed short arithmetic expression and duration derivations use date difference. Each value has one durable capture association, alternating `claude-code` and `cursor`, giving 10,000 associations. The generator uses fixed values, parent order, paths, operations, and expressions; it records the exact rendered byte count.

| Boundary | At the limit, must succeed | One over, must fail | Fixture construction |
|----------|----------------------------|---------------------|----------------------|
| Values | 10,000 | 10,001 | Short numeric roots sharing one source; zero edges and captures. |
| Edges | 30,000 | 30,001 | 3,000 roots and 7,000 derived values. Give 2,000 derived values five distinct parents and 5,000 four distinct parents; add a fifth parent to one of the latter for the over-limit case. Recompute the affected child ID. |
| Rendered UTF-8 bytes | 67,108,864 | 67,108,865 | Start with the representative graph, then distribute deterministic ASCII-space padding across numeric derivation expressions. Use the production renderer with an injected test-only cap to calculate and confirm the exact serialized byte count; keep all row counts unchanged. |

Every boundary fixture must stay at or below the other two limits. The source and capture counts are recorded for audit, even though they have no separate quota. A separate valid fixture producing at least 2 MiB of Turtle exercises a non-reading stdout pipe: the parent retains the pipe's read handle but never drains it, and the child must exit nonzero from the one-second output deadline. Run this real-pipe probe on Linux, macOS, and Windows; use an outer harness watchdog to kill a stuck child and fail the probe, without counting that watchdog as a passing timeout. Keep wall-clock probes outside normal deterministic `cargo test` execution.

## Namespaces and identities

```turtle
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix rgt: <https://github.com/rafael-bianchi/rgt/vocab#> .
```

The RGT namespace is an identifier contract; HTTP dereferencing is not required for graph parsing. Object IRIs use `https://github.com/rafael-bianchi/rgt/iri/v1/` followed by these path segments:

| Object | IRI suffix |
|--------|------------|
| Project | `project/{project-token}` |
| Recorded value | `project/{project-token}/node/{encoded-local-node-id}` |
| Source document | `project/{project-token}/source/{source-row-id}` |
| Derivation activity | `project/{project-token}/activity/derive/{encoded-child-id}` |
| Ordered usage | `project/{project-token}/usage/derive/{encoded-child-id}/{zero-based-index}` |
| Capture activity | `project/{project-token}/activity/capture/{encoded-node-id}/{canonical-agent}` |
| Coding tool agent | `agent/{canonical-agent}` |

The project token is fixed lowercase hex. Local node ID components are percent-encoded from their UTF-8 bytes; they are never interpolated raw. An agent IRI denotes the canonical software tool, not a person, run, or individual session. Matching local IDs in independent projects produce different value IRIs. Physical copies of one store share its project token and therefore its identities.

## RGT terms

| Term | Subject | Meaning |
|------|---------|---------|
| `rgt:Project` | Project | Project identity for the exported store. |
| `rgt:SourceDocument` | Source entity | Stored source document. |
| `rgt:inProject` | Value, source, activity | Link to the project entity. |
| `rgt:localNodeId` | Value | Original RGT node ID as a string literal. |
| `rgt:valueKind` | Value | `NUMBER`, `DATE`, or `DURATION`. |
| `rgt:value` | Value | The recorded typed value. |
| `rgt:isStale` | Value | Current stale state as `xsd:boolean`. |
| `rgt:lineNumber` | Root value | One-based source line as `xsd:integer`, when known. |
| `rgt:relativePath` | Source | Path relative to the selected project, only when safely established. |
| `rgt:absolutePath` | Source | Available absolute path, only with the explicit full-path flag. |
| `rgt:operationType` | Derivation activity | Stored operation label. |
| `rgt:expression` | Derivation activity | Stored expression string, when present. |
| `rgt:operandIndex` | Qualified usage | Zero-based position of a validated parent operand. |
| `rgt:agentName` | Agent | Canonical supported tool name. |

`rgt:value` uses `xsd:double` for an RGT number, `xsd:dateTime` for a date, and a valid `xsd:duration` lexical form for a duration stored in seconds. The exporter preserves the stored value without using RGT's human-readable duration display as an XML Schema duration. All string literals escape quotes, backslashes, line breaks, and control characters according to Turtle syntax and preserve Unicode text.

## PROV-O relationships

| Stored evidence | Required triples |
|-----------------|------------------|
| Root with known source | Root value `prov:wasDerivedFrom` source entity. A missing source link produces no invented source. |
| Every valid parent-child edge | Child value `prov:wasDerivedFrom` parent value. |
| Validated derivation group | Derivation activity `a prov:Activity`; activity `prov:used` every parent; child `prov:wasGeneratedBy` activity. |
| Validated ordered operand | Activity `prov:qualifiedUsage` usage; usage `a prov:Usage`, `prov:entity` parent, and `rgt:operandIndex` position. |
| Known agent-value association | Capture activity `a prov:Activity`, `prov:used` value, and `prov:wasAssociatedWith` canonical agent; agent `a prov:Agent`. |

An ambiguous derivation emits direct supported links but no derivation activity, generation relation, qualified usages, or consolidated operation/expression claim. Stored edge labels are not asserted as one verified calculation. Its warning goes to stderr. A capture relation does not mean the coding tool authored the source or computed a derivation.

## Privacy, stability, and failure rules

- Default output never puts an absolute project or source-document path in project or source identifiers or RGT-generated source-location fields. It emits `rgt:relativePath` only for verified in-project sources; outside or unclassifiable sources have a path-free source IRI with no path literal. Recorded free text, including `rgt:expression`, is preserved verbatim and may contain path-like strings.
- The full-path flag adds `rgt:absolutePath` where an absolute source path is available; it does not alter identities or existing relative paths.
- An unchanged store and the same flags produce byte-stable Turtle after the project token is established. Node, source, activity, usage, and agent facts are emitted in a defined sorted order.
- The row, byte, and deadline limits are part of the Turtle CLI contract. They bound the supported export domain and keep an oversized or stalled export from becoming an unbounded CLI operation.
- A malformed project token, invalid typed value, broken required reference, or failed snapshot read is a hard error rather than a fabricated default. Omitted derivation activities are the only supported partial-evidence warning case.
- The output is a projection of the selected project only. Readers may combine multiple exports by normal RDF union, but this feature emits no cross-project equivalence claim.

## Validation contract

An independent Turtle parser must accept empty, ordinary, escaped-text, multi-parent, and combined-project exports. Contract tests compare graph facts, not just output substrings. They assert correct PROV-O direction, value types, source-path disclosure, stable identities, distinct independent projects, old IDs, duplicate agent capture behavior, omitted uncertain activities, quotas, controlled deadline failures, and stdout/stderr/exit semantics.
