# Implementation Plan: Non-Text Source Capture

**Branch**: `023-non-text-source-capture` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/023-non-text-source-capture/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Close the non-text-source capture gap (findings §4/§6/§3.1) via two workstreams:

1. **Clear error (code, FR-001)**: `rgt record` on a binary/non-UTF-8 file returns an explicit "unsupported file format for `rgt record`" error naming the supported path (plain text/CSV, or the agent's own read tool feeding the hook), instead of the raw `stream did not contain valid UTF-8`.
2. **Explicit-report instruction (instructions, FR-002/FR-003/FR-004)**: the "Recording Values from Non-Text Sources" instruction (findings §6.1, scoped to PDF/Excel/images) is added to the 7 rules/instructions artifacts RGT already writes (Codex, Windsurf, Cline, Antigravity, Kilo, Copilot CLI, Mistral Vibe), and a new `CLAUDE.md` is written for Claude Code. Unverified/plugin-only agents stay out of scope (FR-006).

The instruction is the primary, robust fix (§4/§6): non-text content only becomes text inside the model, so the agent must be told to report it — no local PDF/Excel parsing exists to hook into.

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: None new. The instruction is embedded as a constant; the `rgt record` change reads bytes and converts with `String::from_utf8`.

**Storage**: N/A — no schema/store changes.

**Testing**: `cargo test` — contract tests in `tests/contract/hook_installer.rs` assert each in-scope instructions artifact contains the instruction exactly once (idempotent), and a `tests/contract/test_cli_record_derive_contract.rs` test asserts the binary-file error message; `cargo fmt`/`clippy`.

**Target Platform**: macOS, Linux, Windows (pure string/byte handling).

**Project Type**: CLI tool (Rust single binary) with embedded agent-instructions artifacts.

**Performance Goals**: unchanged — the error detection adds one byte→UTF-8 conversion on the record path; instruction writes are the existing init-time path.

**Constraints**:
- FR-001: non-UTF-8 (binary) input → explicit "unsupported file format" error; covers both `--stdin` and file paths.
- FR-002: instruction appended to the 7 existing rules artifacts (Codex `AGENTS.md`, Windsurf `.windsurfrules`, Cline `.clinerules`, Antigravity, Kilo, Copilot CLI `AGENTS.md`, Vibe `rgt.md`).
- FR-003: `write_claude_code` additionally writes `CLAUDE.md` (project `CLAUDE.md` by default, `~/.claude/CLAUDE.md` with `--global`).
- FR-004: instruction scoped to PDF/Excel/images, explicitly excluding plain-text/CSV/JSON/Markdown.
- FR-005: idempotent, content-preserving writes (reuse 019's `ensure_text_block` + paired markers + backup).
- FR-006: no guessed instructions for Cursor/Copilot Chat/Gemini/OpenCode/Pi/Hermes.

**Scale/Scope**: a new `glue::NON_TEXT_SOURCE_INSTRUCTION` constant + append to 7 rules blocks (incl. `codex_section` and Windsurf's `rules_content`); `write_claude_code` gains a `CLAUDE.md` write; `execute_record` reads bytes and returns the clear error; contract tests. No new modules; `src/hooks/glue/mod.rs`, `src/hooks/installer.rs`, `src/cli/record.rs`, tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new deps; instruction embedded as a const. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | Unaffected. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Instructions are data (thin glue), not logic. |
| IX. Test-Driven Verification | ✅ PASS | Contract tests for instruction presence + error message. |
| X. Documentation Matches Behavior | ✅ PASS | README updated for the new record error + non-text instruction. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy enforced. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/023-non-text-source-capture/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (findings §4/§6/§3.1 consolidation)
├── data-model.md        # Phase 1 output (non-text capture domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (record error + instruction invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── cli/
│   └── record.rs        # MODIFIED: read bytes + from_utf8; explicit unsupported-format error
└── hooks/
    ├── glue/mod.rs      # MODIFIED: NON_TEXT_SOURCE_INSTRUCTION const; appended to rules blocks
    └── installer.rs     # MODIFIED: codex_section + write_windsurf append the instruction;
                         #           write_claude_code writes CLAUDE.md

tests/
├── contract/hook_installer.rs            # MODIFIED: instruction presence/idempotency per agent
└── contract/test_cli_record_derive_contract.rs  # MODIFIED: binary-file error message
```

**Structure Decision**: Single-project layout. The instruction is a single `glue` constant spliced into each agent's rules block at write time (and used as the `CLAUDE.md` body), reusing the 019 `ensure_text_block`/paired-marker machinery — no new modules.

## Complexity Tracking

None — no constitution violations; no new dependencies.
