# Implementation Plan: Safe Duration Calculations and Unit Display

**Branch**: `develop` (planning only; implementation must use a `feat/` branch) | **Date**: 2026-09-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/043-date-diff-units/spec.md`

## Summary

Make `DATE_DIFF` compute a canonical whole-second Duration from its Date parents without requiring an LLM-supplied result. Add typed `DURATION_SUM` and `DURATION_AVG` so mixed-scale Duration parents produce Duration nodes rather than untyped Numbers. Keep optional exact claims separate from selected display units, expose selected-unit display on derive and query, and preserve historical graph identities. Use checked arithmetic and fail before writing on any invalid or unrepresentable calculation. Keep the existing SQLite schema unless research identifies an unavoidable migration.

## Technical Context

**Language/Version**: Rust 2021 edition; project toolchain as resolved by Cargo.

**Primary Dependencies**: Existing `chrono`, `clap`, `rusqlite`, `blake3`, `serde_json`, and `evalexpr`; no new runtime dependency planned.

**Storage**: Project-local SQLite `.rgt/store.db`; `tracked_nodes.duration_secs INTEGER` and `derivation_edges` with one row per parent-child pair.

**Testing**: Deterministic Rust unit, CLI contract, and graph integration tests. Local `cargo fmt`, `cargo clippy`, and selected tests during implementation; no CI/CD or GitHub workflows are part of this planning pass.

**Target Platform**: macOS, Linux, and Windows on the project's supported architectures.

**Project Type**: Single-binary CLI and passive agent hooks; this feature is in the active CLI and graph/export paths.

**Performance Goals**: Measure valid and invalid derive, verify, and query subprocess commands against a fixed 10,000-node fixture, using 26 distinct Duration parents for aggregates. After one warm-up per command, the 95th percentile over 20 runs must remain below one second, including startup and output. Record the local OS, CPU, release profile, commands, and measurements; keep wall-clock timing outside deterministic correctness assertions. Ordinary graph query latency continues to target <10 ms for repositories below 100,000 nodes.

**Constraints**: Whole-second Duration storage; at most 26 parent variables/IDs under the existing CLI limit; no invented unit metadata on stored nodes; no silent rounding, value overwrite, or full-graph scan; historical IDs and no-flag responses remain usable.

**Scale/Scope**: Three typed duration operations (`DATE_DIFF`, `DURATION_SUM`, `DURATION_AVG`); five fixed units (seconds through weeks); selected-unit output on derive and query; optional exact claims on derive and mandatory claims on verify. Graph export retains existing project limits.

## Constitution Check

*GATE: Checked before Phase 0 research; re-checked after Phase 1 design below.*

| Principle / rule | Pre-design gate | Reason |
|---|---|---|
| I. Rust-only single binary | PASS | Shared unit, arithmetic, identity, and display logic stay in the Rust binary with existing dependencies. |
| II. Speed-first detection | PASS | File detection is untouched. |
| III. Graph correctness and minimal invalidation | PASS | New derivations use checked, transactional writes and indexed parent/child lookup; no whole-graph rescan. |
| IV. First-class Date and Duration | PASS | Every new temporal result is a Duration node with lineage. |
| V-VIII. CLI-first integration | PASS | Commands own verification and formatting; hooks gain no business logic. |
| IX. Test-driven verification | PASS | The design requires deterministic unit, CLI contract, migration, and export tests before implementation. |
| VII/X. Public and matching documentation | PASS for planning | Implementation must update README and AGENTS.md and include the canonical feature documents under `specs/043-date-diff-units/` in the same changeset, explicitly staging them because `specs/` is ignored. |
| XI. Quality, errors, portability | PASS | Checked arithmetic, explicit errors, no unhandled panics, and no new runtime engine. |
| Latency and CLI compatibility | PASS | Parent work is bounded, indexed; existing seconds claims and no-flag output remain valid. |
| Branch, commit, and CI governance | PASS for planning | This phase creates only local design files. Implementation must branch from `develop`; any later PR/merge must satisfy the constitution's gates and the user's no-workflow constraint must be reconciled then. |

**Gate result**: No unjustified constitutional violation or unresolved feature clarification blocks research/design.

## Phase 0: Research Decisions

See [research.md](research.md) for evidence, decisions, and alternatives covering exact decimal claims, whole-second arithmetic, canonical graph identity, historical compatibility, query presentation, strict temporal reads, and Turtle export.

## Phase 1: Design and Contracts

- [data-model.md](data-model.md) defines typed claims, durations, aggregates, identity, and state transitions.
- [contracts/duration-cli.md](contracts/duration-cli.md) defines derive, verify, query, errors, numeric syntax, and compatibility.
- [contracts/duration-graph.md](contracts/duration-graph.md) defines node/edge identity, historical reuse, and Turtle behavior.
- [quickstart.md](quickstart.md) lists runnable end-to-end validation scenarios for implementation.

**Post-design constitution check**: PASS for planning. The data model and contracts retain integer-second Duration storage and type, require no new runtime dependency or schema migration, use checked bounded arithmetic and indexed identity lookup, preserve no-flag CLI and historical-ID behavior, and extend Turtle classification rather than dropping supported lineage. The quickstart identifies deterministic local tests and required documentation updates. A future implementation changeset must include the ignored canonical feature documents as well as README and AGENTS.md updates. No branch, commit, PR, CI/CD, or workflow action is part of this planning pass; those governance gates apply at implementation/review time.

## Project Structure

### Documentation (this feature)

```text
specs/043-date-diff-units/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/requirements.md
└── contracts/
    ├── duration-cli.md
    └── duration-graph.md
```

### Source Code (repository root)

```text
src/
├── main.rs                 # CLI flags and operation routing
├── cli/derive.rs           # checked derivation transaction and output
├── cli/query.rs            # selected-unit query output
├── query/mod.rs            # requested-node display fields
├── verify/
│   ├── mod.rs              # operation dispatch and verification CLI
│   ├── date_diff.rs        # full-precision sign/whole-second checks
│   ├── duration.rs         # typed sum/average and claim verification (new)
│   └── expression.rs       # legacy temporal coercion warning
├── types/
│   ├── duration_unit.rs    # exact unit conversion and display (new)
│   ├── node.rs             # canonical Duration-derived ID
│   └── value.rs            # Duration value formatting, if needed
├── store/queries.rs        # strict temporal reads and collision-safe reuse
└── export/turtle.rs        # recognize canonical Duration-derived IDs

tests/
├── unit/                   # exact arithmetic, units, ID and display cases
├── contract/               # CLI syntax, exit codes, unchanged responses
└── integration/            # lineage, historical IDs, query and Turtle
```

**Structure Decision**: Extend the existing single Rust CLI. Keep shared exact duration math separate from command I/O; reuse current SQLite tables and graph traversal. Specific module names may adjust during implementation if a narrower existing module fits better.

## Complexity Tracking

No constitutional exceptions are required for the planned design.
