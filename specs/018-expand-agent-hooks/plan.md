# Implementation Plan: Expand Agent Hook Coverage to Match RTK

**Branch**: `018-expand-agent-hooks` | **Date**: 2026-08-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/018-expand-agent-hooks/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Extend RGT's `--agent` coverage from 4 to 13 AI coding agents (full parity with RTK). Per the clarified scope, this adds **provenance-capture** hooks only — no command rewriting. Two integration tiers:

1. **Event-capture agents** (Copilot, Gemini, OpenCode, Pi, Hermes, Mistral Vibe — new; Claude Code, Cursor — existing): configure the agent to invoke `rgt hook pre/post`. New agents pass `--agent <name>`; Claude Code and Cursor retain their current `--agent`-less invocation so existing configs stay byte-identical (FR-004/SC-003). Rust normalizes each agent's native event JSON to the internal `(path, content)` capture model. Plugin agents (OpenCode, Pi, Hermes) capture `post`-tool only — provenance records the resulting file state after the tool call.
2. **Rules-file agents** (Codex, Windsurf, Cline/Roo Code, Antigravity, Kilo): write project-local instruction files telling the agent how to use RGT.

Glue artifacts (TypeScript plugins, Python plugin, rules files) are **embedded as constants in the Rust binary** and written to agent paths at `rgt init` time, preserving the single-binary distribution (constitution I) — mirroring RTK's thin-delegate architecture (constitution VIII).

## Technical Context

**Language/Version**: Rust (stable, as in `Cargo.toml`) for the binary; embedded thin glue artifacts in TypeScript (OpenCode, Pi), Python (Hermes), and rules-file text (Cline/Roo Code, Antigravity, Kilo). No new Rust dependencies.

**Primary Dependencies**: Existing `serde_json` (agent JSON normalization), `serde` (plugin payload parsing). No new crates. Glue artifacts use only the agent's own plugin APIs (zx/`pi.exec`/Hermes `plugins.enabled`) and invoke the `rgt` CLI as a subprocess.

**Storage**: N/A — no schema or store changes; the existing `.rgt/store.db` and provenance graph are unchanged.

**Testing**: `cargo test` (new unit tests for per-agent JSON normalizers; contract tests for installer output/config files), plus `cargo fmt --all -- --check` and `cargo clippy --all-targets` per constitution IX/XI. Plugin smoke checks live in quickstart (manual validation).

**Target Platform**: macOS, Linux, Windows (agent config paths resolved per host platform; all paths handled via `dirs` + `std::path`).

**Project Type**: CLI tool (Rust single binary) with agent-side hook glue artifacts.

**Performance Goals**: Hook invocation stays sub-second (existing `rgt hook` path unchanged); `rgt init` completes with all 13 agents configured in one run (SC-002).

**Constraints**:
- FR-004: existing 4 integrations (Claude Code, Cursor, Codex, Windsurf) MUST produce byte-identical configs — `rgt hook` without `--agent` keeps the current default format.
- FR-005: every hook fails open (missing binary, malformed JSON, absent runtime → exit 0, no output, no command alteration).
- FR-006: idempotent writes; no clobbering of user-authored config without `--force`.
- FR-007: all logic in Rust; glue is thin delegates (embedded TS/Python/rules, zero business logic).
- No command rewriting (clarified 2026-08-15); RTK is the reference only for agent set + mechanisms.

**Scale/Scope**: 9 new agent integrations (installer entries, normalizers, glue artifacts), `rgt hook` CLI flag extension, tests, docs. ~7 files touched in `src/` (5 modified, 2 new: `glue/`, `paths.rs`), 4 new test files (1 existing modified), embedded glue constants.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS (amended 2026-08-15) | RGT binary remains pure Rust with zero runtime deps. Thin TS/Python glue artifacts are agent-side, written by the binary at init time (not part of RGT's runtime) — covered by the Principle I Agent-Side Glue Exemption (constitution amendment 1.5.0, user-confirmed Option A 2026-08-15). |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | Unaffected. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Plain `rgt init` (no `--agent`) auto-detects and configures the full 13-agent set (SC-002). |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Passive hooks per agent call `rgt hook pre/post`; active CLI unchanged. |
| VII. Permissive OSS License | ✅ PASS | No new dependencies; glue artifacts are MIT/Apache-2.0 (repo license). |
| VIII. CLI-First Hook Architecture | ✅ PASS | Glue = thin delegates (parse event JSON, invoke `rgt hook`); all provenance logic in Rust. |
| IX. Test-Driven Verification | ✅ PASS | New unit tests (normalizers), contract tests (installer), fail-open paths. |
| X. Documentation Matches Behavior | ✅ PASS | README/AGENTS.md/`--help` agent lists updated in same changeset. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | serde_json parsing, clippy/fmt enforced in CI. |

**Gate Result**: PASS — 11/11 principles satisfied; Principle I's Agent-Side Glue Exemption ratified as constitution amendment 1.5.0 (2026-08-15).

## Project Structure

### Documentation (this feature)

```text
specs/018-expand-agent-hooks/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (RTK mechanism research)
├── data-model.md        # Phase 1 output (agent registry + capture model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (hook stdin contracts per agent)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── main.rs              # MODIFIED: Hook { event, agent } flag; Init agent help text
├── hooks/
│   ├── mod.rs           # MODIFIED: handle_passive_hook_event(event, agent); content fallback (read file if content missing)
│   ├── parser.rs        # MODIFIED: per-agent normalizers (copilot, gemini, vibe, opencode, pi, hermes)
│   ├── installer.rs     # MODIFIED: 13-agent registry, config writers, plugin/rules emitters
│   └── glue/            # NEW: embedded thin glue templates (TS plugins, Python plugin, rules files) as const strings
│       └── mod.rs

tests/
├── unit/hook_parsers.rs        # NEW: per-agent event-JSON normalization + fail-open tests
└── contract/hook_installer.rs  # NEW: per-agent config/plugin file content + idempotency tests
```

**Structure Decision**: Single-project layout (existing Rust CLI). Glue artifacts embedded as `const` strings in `src/hooks/glue/mod.rs` and written to agent paths by the installer — no separate distribution channel, single-binary install preserved.

## Complexity Tracking

> Constitution amendments that required justification (resolved via constitution amendment 1.5.0, 2026-08-15):

| Exemption | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Principle I (zero runtime engines): shipping thin TypeScript (OpenCode, Pi) and Python (Hermes) plugin glue | Those three agents expose NO shell-hook API; only plugin systems (TS/Python) can invoke RGT per tool call. Full-parity coverage (user-confirmed Option A) requires them | Restricting these agents to rules-file instructions (Option B) would drop real per-tool-call capture and fail the full-parity goal; excluding them (Option C) reduces the supported count below RTK's |
| The glue is embedded in the binary, not shipped as a separate package | Keeps `cargo install`/install.sh single-artifact UX | Shipping a separate `hooks/` directory would require a second distribution channel and break the one-command install |
