# Implementation Plan: Capture Bash-Tool Reads

**Branch**: `026-capture-bash-tool-reads` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/026-capture-bash-tool-reads/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Close the Bash-tool capture gap (findings §8) for Claude Code and Cursor:

1. **Dispatch fallback (FR-001)**: `normalize_agent_event` for `None`/`claude-code`/`cursor` tries `default_dialect` first; when that yields no path, it falls back to `command_dialect`, so Bash tool calls (whose payload is `tool_input.command`/`tool_response.stdout`) are captured.
2. **`rtk` wrapper (FR-002)**: `extract_path_from_command` recognizes `rtk` as a wrapper — when the first token is `rtk`, the second token is checked against the read-style list — and adds `grep` to the read verbs.
3. **stdout as content (FR-003)**: `command_dialect` captures `tool_response.stdout` as `content` when non-empty; otherwise it returns path-only so the existing disk-read fallback applies.
4. **Passive (FR-004)**: no command rewriting — RGT only widens what it observes.

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: `serde_json` (already present). **No new dependencies.**

**Storage**: N/A — no schema/store changes; the hook runtime (`handle_passive_hook_event`) and its disk-read fallback are unchanged.

**Testing**: `cargo test` — unit tests in `tests/unit/hook_parsers.rs` for the §8.1 payload shape, the `rtk read`/`rtk cat` wrapper cases, stdout-as-content vs path-only, and the native-`Read`/non-read no-op behavior. `cargo fmt`/`clippy`.

**Target Platform**: macOS, Linux, Windows (pure string parsing).

**Project Type**: CLI tool (Rust single binary).

**Performance Goals**: unchanged — string parsing only.

**Constraints**:
- FR-001: fallback only when `default_dialect` yields no path (native `Read` events unchanged).
- FR-002: `rtk <verb>` where `<verb>` is read-style → extract path; `rtk <non-read>` → no path.
- FR-003: stdout used as content when non-empty; path-only otherwise.
- FR-004: no command mutation.

**Scale/Scope**: edits confined to `src/hooks/parser.rs` (`normalize_agent_event`, `default_dialect` caller, `command_dialect`, `extract_path_from_command`) and `tests/unit/hook_parsers.rs`. No other files.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new dependencies. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | Unaffected. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Passive capture widened. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Parser-only; hooks remain thin, passive, non-rewriting (FR-004). |
| IX. Test-Driven Verification | ✅ PASS | Unit tests for the payload/wrapper/stdout cases. |
| X. Documentation Matches Behavior | ✅ PASS | README agent-matrix note (Bash reads now captured). |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/026-capture-bash-tool-reads/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (dialect/command-parsing design)
├── data-model.md        # Phase 1 output (Bash-capture domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (parser invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
└── hooks/
    └── parser.rs        # MODIFIED: normalize dispatch fallback; command_dialect stdout;
                         #           extract_path_from_command rtk wrapper + grep

tests/
└── unit/hook_parsers.rs # MODIFIED: payload/wrapper/stdout/negative unit tests
```

**Structure Decision**: Single-project layout; one source file + its existing test file.

## Complexity Tracking

None — no constitution violations; no new dependencies.
