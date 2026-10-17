# Implementation Plan: Agent Instructions Feature Parity

**Branch**: `feat/039-agent-instructions-feature-parity` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/039-agent-instructions-feature-parity/spec.md`

## Summary

The installed agent-instructions text for 6+ agent integrations lags the feature set: `CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, `VIBE_PROMPT`, `COPILOT_CLI_RULES` (`src/hooks/glue/mod.rs`), `codex_section()` and the Windsurf inline base (`src/hooks/installer.rs`) only teach `record`/`derive`/`status`/`query`/`graph`. Bring each to the canonical `AGENTS.md` feature set (add `rgt doctor`, `rgt gc [--vacuum]`, `--number-format`) in each artifact's existing style, preserving the `with_instruction` wrapper. Also fix `CONTRIBUTING.md`'s stale file/test listings. Text-only change; no installer logic.

## Technical Context

**Language/Version**: Rust (string constants) + Markdown (`CONTRIBUTING.md`).

**Primary Dependencies**: None.

**Storage**: N/A.

**Testing**: A contract test in `tests/contract/hook_installer.rs` (or a new unit test) asserting the written artifacts mention `doctor`/`gc`/`--number-format`; existing tests must keep passing (positive assertions on `rgt record <file>` etc.).

**Target Platform**: All (text embedded in the binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: N/A.

**Constraints**: Constitution VIII (glue stays thin — text only, no logic). Constitution IX (test the new content). Constitution X (installed docs must match behavior). Constitution XI (no new deps).

**Scale/Scope**: `src/hooks/glue/mod.rs` (5 constants), `src/hooks/installer.rs` (`codex_section()` + windsuf base), `CONTRIBUTING.md`, tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Text constants only. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Unaffected. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | Unaffected. |
| VIII. Passive hooks, thin delegates | PASS | Glue stays text-only; no logic added. |
| IX. Test-driven verification | PASS | New contract assertions; existing tests must stay green. |
| X. Documentation matches behavior | PASS | The entire point: installed instructions re-synced. |
| XI. Idiomatic Rust, exit codes | PASS | No logic change. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/039-agent-instructions-feature-parity/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── glue-parity-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── hooks/
│   ├── glue/mod.rs      # 5 constants updated
│   └── installer.rs     # codex_section() + windsurf base updated
CONTRIBUTING.md          # file/tree + test examples updated

tests/
└── contract/
    └── hook_installer.rs   # new parity assertions
```

**Structure Decision**: Single-project CLI. Text edits to two source files + CONTRIBUTING.md; tests extend the existing installer contract file.

## Complexity Tracking

> None — no constitution violations requiring justification.
