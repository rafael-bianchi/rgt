# Implementation Plan: Fix Query JSON Flag Behavior

**Branch**: `feat/040-fix-query-json-flag-behavior` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/040-fix-query-json-flag-behavior/spec.md`

## Summary

`rgt query <id>` prints JSON in both the default and `--json` branches of `execute_query` (`src/cli/query.rs:9-18`) — the only difference is a cosmetic `{"lineage": ...}` wrapper on the default branch. The README and `--help` both document `--json` as opt-in (default = human-readable text). Implement a real plain-text lineage renderer for the default path, and make `--json` emit clean JSON (no wrapper). The lineage data (`query_provenance` in `src/query/mod.rs`) is unchanged.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021).

**Primary Dependencies**: None new. Uses `serde_json` (existing) for the `--json` path and `TrackedNode`/`ValueKind`/`ValueData` display impls (existing).

**Storage**: N/A — reads existing lineage via `query_provenance`.

**Testing**: New contract test target `tests/contract/test_query_contract.rs` (registered in `Cargo.toml`) asserting text-vs-JSON behavior, parseability, and error cases.

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: N/A — presentation-only change; output size is proportional to lineage size.

**Constraints**: Constitution X (docs already match the target behavior; do not drift). Constitution XI (no new dependencies, exit-code semantics preserved). No change to `src/query/mod.rs`.

**Scale/Scope**: `src/cli/query.rs` (rewrite the presentation), `src/main.rs` (none — flag already wired), `Cargo.toml` (add test target), new `tests/contract/test_query_contract.rs`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | No new dependencies. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Lineage data construction untouched. |
| IV. Rich value types | PASS | Uses existing `ValueKind`/`ValueData` display. |
| V. Distribution / RTK UX | PASS | Unaffected. |
| VI. Dual passive/active surface | PASS | `query` is an active command; output shape refined. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | New contract test target covering both modes + errors. |
| X. Documentation matches behavior | PASS | README/`--help` already describe the corrected behavior; verified no drift. |
| XI. Idiomatic Rust, exit codes | PASS | "Node not found" still exits non-zero; no panics. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/040-fix-query-json-flag-behavior/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── query-output-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
└── cli/
    └── query.rs               # the only production file changed

tests/
└── contract/
    └── test_query_contract.rs # NEW: text vs JSON vs error contract
```

**Structure Decision**: Single-project CLI. One production file rewritten (`src/cli/query.rs`): keep `execute_query` as the thin entry, add a pure `render_lineage_text(result: &serde_json::Value) -> String` helper (testable without IO) and emit the JSON path's `serde_json::to_string_pretty(result)` directly (no wrapper). A new `tests/contract/test_query_contract.rs` covers the behavior end-to-end.

## Complexity Tracking

> None — no constitution violations requiring justification.
