# Contract: Non-Text Source Capture Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the contract tests for `rgt record` and `rgt init`.

## `rgt record` error contract (FR-001)

- Non-UTF-8 (binary) input — file path or `--stdin` — returns an error whose message names "unsupported file format for `rgt record`" and the supported path (plain text/CSV, or the agent's own read tool). It MUST NOT be the raw "stream did not contain valid UTF-8" message.
- Plain-text/CSV input behavior is unchanged.

## Instruction contract (FR-002/FR-003/FR-004/FR-005)

- After `rgt init`, each in-scope instructions artifact contains the `NON_TEXT_SOURCE_INSTRUCTION` text exactly once:
  - Codex `AGENTS.md`, Windsurf `.windsurfrules`, Cline `.clinerules`, Antigravity `.agents/rules/antigravity-rgt-rules.md`, Kilo `.kilocode/rules/rgt-rules.md`, Copilot CLI `AGENTS.md`, Mistral Vibe `~/.vibe/prompts/rgt.md`.
- `rgt init --agent claude-code` writes `CLAUDE.md` (project) containing the instruction; `-g` writes `~/.claude/CLAUDE.md`.
- The instruction text MUST mention PDF/Excel/images AND explicitly state plain-text/CSV/JSON/Markdown are already auto-captured (no action).
- Re-running `rgt init` (and `--force`) never duplicates the instruction; user content outside RGT's block is preserved byte-for-byte.

## Out-of-scope contract (FR-006)

- No instructions file is written for `cursor`, `copilot` (VS Code Chat), `gemini`, `opencode`, `pi`, or `hermes` as part of this feature.

## Acceptance linkage

- US1/FR-001/SC-001 → record error contract.
- US2/FR-002/FR-004/SC-002/SC-004 → instruction coverage + scope.
- US3/FR-003/SC-003 → Claude Code `CLAUDE.md`.
- US4/FR-006/SC-005 → out-of-scope contract.
