# Contract: Doctor & Hook-Artifact Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Executable invariants verified by `tests/unit/` and `tests/contract/test_doctor_contract.rs`.

## I. `rgt doctor` exit-code contract (FR-001, FR-002)

| Scenario | Exit code | Output |
|----------|-----------|--------|
| Store not initialized | 1 | "not initialized … run `rgt init`" |
| Store initialized, hooks present, values recorded | 0 | healthy report |
| Store initialized, hooks present, zero values | 0 | warning naming PATH resolution |
| Store initialized, no hooks | 0 | warning "hooks not installed" |

## II. Absolute-path hook commands (FR-003)

- For `claude-code`, `cursor`, `copilot` (Chat), `gemini`, `vibe`, `opencode`, `pi`, `hermes`, the written command references the CLI by absolute path (from `std::env::current_exe()`), never the bare `rgt` name.
- The five instructions-only agents (`codex`, `windsurf`, `cline`, `antigravity`, `kilocode`) are NOT path-injected. (`copilot` CLI rules and the `vibe` prompt are instruction text *in addition to* their subprocess hooks and remain instruction text.)

## III. Claude Code matcher (FR-004)

- `settings.json` hook entries use `"matcher": "Read|Edit|Write|Bash"`, not `""`.
- `Bash` remains matched (026 Bash-read capture preserved).

## IV. Restart reminder (FR-005)

- `rgt init` prints a restart instruction iff ≥1 hook was newly configured.
- A no-op re-run (all `Skipped`) does not print the reminder.

## V. Copilot CLI dialect assumption (FR-006)

- A unit test pins `copilot_dialect` for the assumed shapes; the assumption is documented in the spec.
