# Implementation Plan: Hook Editor Path Quoting Fix

**Branch**: `feat/031-hook-editor-path-quoting-fix` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/031-hook-editor-path-quoting-fix/spec.md`

## Summary

`is_rgt_command` (`src/hooks/editor.rs:400-416`) tokenizes a hook `command` string with `split_whitespace()` — not quote-aware. `shell_quote` (`src/hooks/installer.rs:231-233`) writes commands with the resolved `rgt` path wrapped in literal double quotes. When that path contains a space (e.g. `/Applications/My App/rgt`), the predicate splits inside the quoted path and fails to recognize RGT's own entry, so a re-init can append a duplicate. Fix: make `is_rgt_command`'s tokenizer quote-aware with a small, dependency-free parser that respects a shell-quoted first token. No change to `shell_quote`.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021).

**Primary Dependencies**: None new. The quote-aware tokenizer is hand-rolled (a ~15-line state machine) per Constitution XI — no shell-parsing crate.

**Storage**: N/A.

**Testing**: Unit tests in `tests/unit/hook_editors.rs` for the predicate across spaced/unspaced/Windows/`--agent`/non-RGT forms; a contract test in `tests/contract/hook_installer.rs` for re-init idempotency with a spaced install path.

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: N/A — predicate runs only during `rgt init` config reads.

**Constraints**: Constitution XI (no new deps, no panics on malformed input). Constitution IX (new coverage for the spaced-path case that was previously reasoned-only). No change to `shell_quote`.

**Scale/Scope**: `src/hooks/editor.rs` (`is_rgt_command` + a small tokenizer helper); tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | No new dependency. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Unaffected. |
| VI. Dual passive/active surface | PASS | Fixes a config-merge path used by passive hooks. |
| VII. Open-source governance | PASS | No new third-party code. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | New predicate + re-init idempotency tests, including a spaced path. |
| X. Documentation matches behavior | PASS | No user-facing behavior change. |
| XI. Idiomatic Rust, exit codes | PASS | Hand-rolled tokenizer, no panics. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/031-hook-editor-path-quoting-fix/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── command-recognition-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
└── hooks/
    └── editor.rs               # is_rgt_command made quote-aware

tests/
├── unit/
│   └── hook_editors.rs         # predicate unit tests (new forms)
└── contract/
    └── hook_installer.rs       # re-init idempotency with spaced path
```

**Structure Decision**: Single-project CLI. One production file edited (`src/hooks/editor.rs`); tests extend two existing files.

## Complexity Tracking

> None — no constitution violations requiring justification.
