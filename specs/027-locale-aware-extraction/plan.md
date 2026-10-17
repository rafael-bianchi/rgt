# Implementation Plan: Locale-Aware Extraction

**Branch**: `feat/027-locale-aware-extraction` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/027-locale-aware-extraction/spec.md`

## Summary

Extend RGT's numeric extraction so signs and locale separators are handled correctly. Today `extract_values_from_content` matches `\b(\d+(?:\.\d+)?)\b`, so `-42` is read as `42`, `(500)` as `500`, and `1,234.56` splits into two bogus nodes. This feature makes a leading `-`/`+` and accounting parentheses `(N)` part of the number (locale-independent, always on), and interprets decimal/thousands separators per a selectable `--number-format us|eu|auto` flag. The format persists per project in a new `project_settings` SQLite table so the passive hook path uses it. Default (no flag, no setting) is `auto`, which infers the format once per file (rightmost separator is the decimal when both `.` and `,` appear; a single separator falls back to `us`). Malformed-for-format numbers are skipped, never silently split; `rgt record` warns, hooks skip silently.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021)

**Primary Dependencies**: `rusqlite` (SQLite), `regex`, `chrono`, `clap`, `petgraph`, `blake3` — all already in `Cargo.toml`. No new dependencies.

**Storage**: SQLite at `.rgt/store.db` (existing). Add a `project_settings` key/value table (additive schema migration).

**Testing**: `cargo test` (unit `tests/unit/hook_parsers.rs`, `tests/unit/test_record.rs`; contract `tests/contract/test_hooks_contract.rs`, `tests/contract/test_cli_record_derive_contract.rs`).

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: Extraction stays linear in content size; no regression to the sub-second CLI budget and no impact on the <1ms no-change detection path.

**Constraints**: Zero new runtime dependencies (Constitution I, XI); passive hooks fail open and never rewrite commands (VIII); malformed numbers are skipped not emitted (FR-005).

**Scale/Scope**: Up to `VALUE_LIMIT` = 10,000 values per file (existing).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Pure Rust, hand-rolled parsing; no new crates. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected (extraction, not detection). |
| III. Graph correctness | PASS | Node-ID change is intended and documented; derivation graph untouched. |
| IV. Rich value types | PASS | `Number` values may now be negative; `f64` already supports this. |
| V. Distribution / RTK UX | PASS | New `--number-format` flag is additive. |
| VI. Dual passive/active surface | PASS | Both `record` and hook paths updated. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Hooks remain capture-only; no command rewriting. |
| IX. Test-driven verification | PASS | New unit + contract tests; existing tests updated for new default. |
| X. Documentation matches behavior | PASS | README/AGENTS updated with `--number-format` in the same changeset. |
| XI. Idiomatic Rust, no new deps | PASS | `cargo fmt`/`clippy` enforced; invalid `--number-format` exits code 2. |

**Post-design re-check**: No violations introduced — the `project_settings` table is an additive migration, the new flag is opt-in additive, and no new dependencies are added.

## Project Structure

### Documentation (this feature)

```text
specs/027-locale-aware-extraction/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── number-format-invariants.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── hooks/
│   ├── parser.rs        # NumberFormat enum, ExtractionResult, extract_values_from_content
│   └── mod.rs           # handle_passive_hook_event reads persisted format
├── cli/
│   ├── record.rs        # --number-format flag, warning on malformed
│   ├── init.rs          # --number-format flag, persists to project_settings
│   └── mod.rs
├── store/
│   ├── schema.rs        # project_settings table
│   └── queries.rs       # get_setting / set_setting helpers
└── main.rs              # CLI arg plumbing for Init + Record

tests/
├── unit/
│   ├── hook_parsers.rs  # new NumberFormat + sign + separator unit tests
│   └── test_record.rs   # updated for new signature/default
├── contract/
│   ├── test_hooks_contract.rs    # extraction contract tests
│   └── test_cli_record_derive_contract.rs  # init/record --number-format contract
└── integration/         # quickstart end-to-end scenario
```

**Structure Decision**: Single-project CLI (Option 1). No new top-level modules; the feature lives inside the existing `hooks::parser`, `cli`, and `store` modules.

## Complexity Tracking

> None — no constitution violations requiring justification.
