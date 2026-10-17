# Implementation Plan: Export Provenance as Turtle

**Branch**: `develop` (planning only; implementation requires a `feat/...` branch) | **Date**: 2026-09-24 | **Spec**: [spec.md](spec.md)

**Input**: Clarified feature specification at `specs/042-export-provenance-turtle/spec.md`.

## Summary

Add `rgt graph --format ttl` as a deterministic PROV-O-aligned view of the current project graph. Keep existing node IDs unchanged, but scope exported identities by a persisted random project token so independently initialized projects never conflate matching local IDs. Export only source paths safe for sharing by default. Retain one agent-value capture association for event-capable integrations and serialize it as a capture activity, without claiming authorship. Reconstruct a derivation activity only when stored ordered edges match the child node's content-derived ID; otherwise keep supported direct lineage and warn. Use a focused Rust writer and existing SQLite store, with no runtime RDF engine.

This is a design artifact. The prior [assessment decision](../../.specify/assessments/prov-o-turtle-export/decision.md) still recommends holding implementation until a named consumer and exchange task are identified.

## Technical Context

**Language/Version**: Rust 2021 edition; current crate version 0.5.0.

**Primary Dependencies**: Existing `rusqlite`, `blake3`, `chrono`, `clap`, and `serde_json`. No new runtime RDF dependency. An independent Turtle parser may be added as a test-only dependency after license verification.

**Storage**: Existing per-project `.rgt/store.db`. Add one durable `capture_associations` table with a versioned schema migration. Use the existing `project_settings` table for a random export project token; no node ID rewrite.

**Testing**: Deterministic Rust unit, contract, and integration tests through `cargo test`; independent Turtle parser for contract validation; `cargo fmt --all -- --check` and `cargo clippy --all-targets` at implementation.

**Target Platform**: macOS, Linux, and Windows on the architectures in the constitution.

**Project Type**: Single-binary CLI with passive agent hooks and an existing SQLite provenance store.

**Performance Goals**: Enforce a one-second command-wide deadline from CLI entry through database access, snapshot, rendering, and final stdout write. Accept at most 10,000 values, 30,000 derivation edges, and 64 MiB of rendered UTF-8 Turtle; fail promptly with an actionable error above any limit. Referenced sources and capture pairs are already bounded by the value cap and graph rules. Benchmark the fixed 10,000-value graph and exact value, edge, and byte boundaries in the [contract](contracts/turtle-export-contract.md) on a standard, GitHub-hosted x86_64 `ubuntu-24.04` runner, including startup and output to a local file. Use bounded bulk snapshot reads and one render pass rather than per-node database queries.

**Constraints**: Existing text, Mermaid, and DOT output and value verification stay intact; hooks fail open; quota, store, render, and pre-write deadline errors emit zero Turtle bytes; only explicit full-path export can disclose an absolute stored source-document path through RGT-generated source-location fields. Recorded free text, including expressions, is preserved verbatim and may contain path-like strings. TTL uses a bounded store lock wait and deadline-aware final write; a failed or timed-out write can still leave bytes in a pipe or file, but must exit nonzero rather than signal a complete export. A synchronous blocked stdout write cannot be interrupted by a clock check on the same thread: run the final write on a dedicated worker, wait on a completion channel for only the remaining deadline, and on timeout emit stderr and terminate the process nonzero without joining the blocked writer. Test this mechanism with a real non-reading pipe on each target OS.

**Scale/Scope**: One selected project per invocation within the explicit Turtle row and byte limits above. Oversize projects remain recordable and queryable and retain existing graph formats, but Turtle export returns an error. Eight current agent integrations can supply capture events; the other five provide instructions only.

## Constitution Check

*GATE: Pass before Phase 0 research; rechecked after Phase 1 design below.*

| Principle | Gate result | Design evidence |
|-----------|-------------|-----------------|
| I, VII: single Rust binary and compatible dependencies | Pass | Writer stays in Rust; no runtime RDF engine or service. Any parser is test-only and license-checked. |
| II, III, IV: change detection, graph correctness, typed values | Pass | No change to detection or node ID generation. Export is an explicit full-graph operation, not an incremental invalidation path; numeric, date, and duration values retain distinct types. |
| Architecture: CLI latency | Pass with delivery requirement | The supported Turtle domain has explicit row and byte limits plus a one-second command-wide deadline, including bounded lock wait and output write. The fixed 10,000-value and exact-limit fixtures must succeed under one second on the pinned runner class; one-over-limit and real blocked-pipe deadline paths must fail under one second before release. |
| VI, VIII: CLI-first and thin hooks | Pass | `graph` is the public export interface. Existing hook delegates still call the binary; attribution logic remains inside it. |
| IX: deterministic verification | Pass | Plan includes independent parsing, identity/collision, ambiguity, escaping, migration, hook, and regression tests. |
| X: documentation parity | Pass with delivery requirement | Implementation must update README, localized graph-format references, AGENTS.md, and CLI help in the same changeset. |
| XI: errors, compatibility, and code quality | Pass | Explicit validation, versioned migration, actionable errors, stable old formats, and required fmt/clippy checks. |
| Workflow: branch and go/no-go | Pass for planning | Current `develop` checkout is used only for local artifacts. A feature branch and a renewed product decision are required before implementation. |

No constitution violation is accepted by this plan. The implementation gate remains separate from design readiness.

## Project Structure

### Documentation (this feature)

```text
specs/042-export-provenance-turtle/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── turtle-export-contract.md
└── tasks.md                  # Phase 2, created by speckit-tasks
```

### Source Code (repository root)

```text
src/
├── main.rs                   # graph option and hook agent dispatch
├── lib.rs
├── cli/
│   ├── graph.rs              # format dispatch and stdout/stderr behavior
│   └── derive.rs             # atomic future derivation writes
├── export/
│   ├── mod.rs
│   └── turtle.rs             # strict snapshot, mapping, escaping, render
├── hooks/
│   ├── mod.rs                # text/PDF capture associations
│   ├── parser.rs
│   └── installer.rs          # canonical agent registry
└── store/
    ├── db.rs
    ├── schema.rs             # v1 to v2 migration
    └── queries.rs            # token and association operations

tests/
├── unit/                     # identity, path, literal, derivation evidence
├── contract/                 # CLI/Turtle parser and hook capture
└── integration/              # old-store migration and manual timing/pipe probes

Cargo.toml                    # register test targets and any test-only parser
.github/workflows/turtle-performance.yml # on-demand three-OS timing/pipe gate
```

**Structure Decision**: Extend the existing single-crate CLI, store, and hook modules. A focused export module separates Turtle mapping from other graph formats. Test files follow the repository's unit, contract, and integration layout.

## Phase 0: Research Decisions

All technical unknowns have decisions in [research.md](research.md): project-scoped identity, conservative derivation activity reconstruction, durable agent associations and eight-agent capture scope, path disclosure, typed Turtle serialization, and output/error behavior. No unresolved technical question remains.

## Phase 1: Design

1. **Identity and migration**: Atomically insert a random project token into `project_settings` if absent, including for older stores; reread on concurrent opens. Add `capture_associations(node_id, agent_name)` with a primary key on the pair and a cascade foreign key to the node. Advance the schema version with an explicit migration. Never regenerate the token during normal open or export and never rewrite local node IDs.
2. **Capture path**: Canonicalize known agent names before parsing hook input. Insert each known agent-value association in the same transaction as its successfully recorded node for both text and PDF capture. Repeated pairs are idempotent. Manual records and rules-only integrations get no inferred agent.
3. **Derivation evidence**: Make future derived node and edge writes atomic. For export, sort a child's stored edges by insertion ID, require consistent operation and expression, existing distinct parents, and a recomputed derived ID matching the stored 128-bit or historical 48-bit form. Emit one derivation activity with its recorded operation and expression only for such a group. For any other stored group, emit supported `prov:wasDerivedFrom` links and an stderr warning without an invented activity or a consolidated operation/expression claim.
4. **Turtle export**: Check the value and edge row limits with bounded `limit + 1` reads inside the same SQLite snapshot used for bulk loading, decoding values strictly without the current query helpers' malformed-row fallbacks. Referenced sources are at most one per root and capture associations at most eight per value; validate those relationships while loading. From the first usable Turtle increment, build every project-scoped project/node/source/derivation/usage IRI from the persisted token and safe components and emit each value's unchanged `rgt:localNodeId`; never embed raw paths in identifiers. Use PROV-O for entities, activities, agents, lineage, and ordered qualified usage on validated derivations, plus documented RGT terms for values, source locations, status, operation, and expression. Render in a fixed order with correct RDF literal and IRI escaping, abort as soon as the buffered output would exceed 64 MiB, then write stdout only after query, validation, and rendering succeed. Track one deadline from CLI entry; bound SQLite lock waiting. Write the final buffer from a dedicated worker thread, wait on its completion channel for the remaining budget, and on timeout report the error on stderr and exit nonzero without joining a blocked writer.
5. **Path handling**: Show a project-relative source path only when containment can be established. Use a path-free source reference for outside or unclassifiable sources. Add an explicit TTL-only option to include available absolute source-document paths. Project and source identifiers and RGT-generated source-location fields never reveal absolute paths by default; recorded free text is not scrubbed.
6. **Interfaces and validation**: Apply [the contract](contracts/turtle-export-contract.md) and [data model](data-model.md); register the new contract and migration test targets in `Cargo.toml`; run the scenarios in [quickstart.md](quickstart.md). Keep legacy graph-format behavior and hook fail-open behavior. Use deterministic quota, lock-wait, and controlled-writer deadline tests in normal CI. Add ignored real-pipe and latency probes, invoke them on Linux, macOS, and Windows through an on-demand workflow, and record all ten release-build timings for the fixed 10,000-value and exact-limit fixtures on the pinned Linux runner. Do not release if a supported exact-limit case or blocked-pipe timeout misses the bound.

## Post-Design Constitution Recheck

**Pass as a bounded design; performance remains a delivery gate.** The Phase 1 model adds one versioned table and one existing-settings value, keeps all business logic in the Rust binary, uses no runtime RDF store, preserves passive hook failure behavior, and specifies explicit export limits, a command-wide deadline, deterministic error tests, near-limit benchmarks, and documentation updates. The one-second bound must be demonstrated before release; no exception to the constitution is accepted. The separate product assessment still controls whether implementation should start.
