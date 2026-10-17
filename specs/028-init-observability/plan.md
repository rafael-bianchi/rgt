# Implementation Plan: Init Observability

**Branch**: `feat/028-init-observability` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/028-init-observability/spec.md`

## Summary

Make a broken or not-yet-active RGT install visible. Add a `rgt doctor` diagnostic subcommand that distinguishes errors (not initialized, unreadable config → non-zero exit) from warnings ("hooks present but no captures yet" → PATH hint, zero exit). Prevent the silent-PATH failure by writing hook commands that reference the CLI by absolute path (resolved at `rgt init` time via `std::env::current_exe()`) for the eight subprocess-spawning hook agents (Claude Code, Cursor, Copilot Chat, Gemini, Vibe, OpenCode, Pi, Hermes). Scope the Claude Code hook matcher from `""` (matches everything) to `Read|Edit|Write|Bash` (preserving 026's Bash-read capture). Make `rgt init` print a restart reminder when it newly configures a hook. Pin the Copilot CLI dialect assumption with a unit test.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021)

**Primary Dependencies**: `clap`, `rusqlite`, `serde_json`, `dirs`, `chrono` — all already in `Cargo.toml`. No new dependencies.

**Storage**: SQLite at `.rgt/store.db` (existing); `rgt doctor` reads node counts from it, never writes.

**Testing**: `cargo test` — unit `tests/unit/hook_parsers.rs`, `tests/unit/hook_editors.rs`; contract `tests/contract/hook_installer.rs`, `tests/contract/test_hooks_contract.rs`; new `tests/contract/test_doctor_contract.rs`.

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: `rgt doctor` completes in well under a second (a handful of filesystem `exists` checks + one SQLite count query).

**Constraints**: Zero new runtime dependencies (Constitution I, XI); hooks remain thin delegates and fail open (VIII); `rgt doctor` is read-only.

**Scale/Scope**: 13 agents, 6 of which spawn a subprocess and get the absolute-path treatment.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Pure Rust; `std::env::current_exe()` needs no new crate. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | `doctor` reads counts only. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Additive `doctor` subcommand; flag/behavior additive. |
| VI. Dual passive/active surface | PASS | `doctor` is an active diagnostic command. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Absolute path is still a thin command string; glue keeps zero logic. |
| IX. Test-driven verification | PASS | New unit + contract tests; existing installer tests updated. |
| X. Documentation matches behavior | PASS | README/AGENTS updated with `rgt doctor`. |
| XI. Idiomatic Rust, exit codes | PASS | `doctor` uses 0 (healthy/warning) / 1 (error); no new deps. |

**Post-design re-check**: No violations — absolute-path glue is still a thin delegate, the new subcommand is additive, no dependencies added.

## Project Structure

### Documentation (this feature)

```text
specs/028-init-observability/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── doctor-invariants.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── doctor.rs        # NEW: execute_doctor (diagnostics + exit code)
│   ├── init.rs          # absolute path + restart reminder
│   ├── mod.rs
│   └── status.rs
├── hooks/
│   ├── glue/
│   │   └── mod.rs       # plugin constants -> path-injected format functions
│   ├── installer.rs     # absolute-path command strings + matcher scope
│   └── parser.rs        # copilot_dialect (unchanged; test-only)
└── main.rs              # Doctor subcommand wiring

tests/
├── unit/
│   ├── hook_parsers.rs  # Copilot CLI dialect assumption test (US5)
│   └── hook_editors.rs  # matcher / absolute-path artifact assertions
├── contract/
│   └── test_doctor_contract.rs  # doctor exit-code contract (US1)
└── integration/         # quickstart scenario
```

**Structure Decision**: Single-project CLI. One new module (`src/cli/doctor.rs`); edits to `installer.rs`, `glue/mod.rs`, `init.rs`, `main.rs`.

## Complexity Tracking

> None — no constitution violations requiring justification.
