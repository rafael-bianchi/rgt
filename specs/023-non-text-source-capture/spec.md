# Feature Specification: Non-Text Source Capture

**Feature Branch**: `feat/023-non-text-source-capture`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "When an agent reads a PDF, Excel (.xlsx), image, or other binary/non-text source, RGT must still capture its numbers and dates — or clearly tell the user it can't. Since the platform provides no local text extraction (PDF/image interpretation happens inside the model, confirmed in findings §4), the fix is (1) agent instructions that tell the agent to explicitly report values from non-text sources, and (2) a clear 'unsupported format' error from `rgt record` for binary files instead of a raw UTF-8 error. Findings §4 + §6 + §3.1."

Source context: `docs/findings-hooks-and-value-extraction.md` §4 (confirmed native-PDF gap), §6 (per-agent explicit-report instructions), §3.1 (no native binary parsing). Plan: `docs/spec-plan-from-findings.md`, spec #5, P1.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `rgt record` Reports Unsupported Formats Clearly (Priority: P1)

A developer runs `rgt record report.pdf` (or `.xlsx`) directly. Today it fails with a generic, misleading `failed to read file: stream did not contain valid UTF-8` — with no indication of *why* or what to do instead. It must return a clear, actionable message: non-text formats aren't natively parsed by `rgt record`, and the supported path is plain text/CSV or letting the agent's own read tool feed the hook.

**Why this priority**: This is the §3.1 finding — the only code-level fix in this feature, and the first thing a user hits when trying to record a PDF/Excel. A silent/generic error blocks the whole non-text workflow with no recovery path.

**Independent Test**: Unit/contract tests pass a non-UTF-8 byte sequence (e.g. a `.pdf` magic prefix) to the record path and assert the error names "unsupported file format" and the supported alternatives — not the raw UTF-8 message.

**Acceptance Scenarios**:

1. **Given** a `.pdf` file (binary), **When** `rgt record <file>.pdf` runs, **Then** it prints an error naming the unsupported format and the supported path (plain text/CSV, or the agent's own read tool), exiting non-zero.
2. **Given** a `.xlsx` file, **When** `rgt record` runs, **Then** the same clear unsupported-format error is produced (not a UTF-8 decode error).
3. **Given** a plain-text/CSV file, **When** `rgt record` runs, **Then** behavior is unchanged (values extracted as before).

---

### User Story 2 - Agents Are Told to Report Non-Text Values (Priority: P1)

A developer uses any agent RGT already gives instructions to (Codex, Windsurf, Cline/Roo Code, Antigravity, Kilo, Copilot CLI, Mistral Vibe). Those instructions files currently only cover `rgt record`/`derive` for data files. After this feature, they also tell the agent: for PDF/Excel/image sources, explicitly extract the numbers and dates and report them via `rgt record --stdin` (or an intermediate text file), because the passive hook cannot see inside binary files.

**Why this priority**: This is the primary, robust fix from §6 — the only place non-text content ever becomes text is inside the model, so the instruction must be delivered to the agent. It's a documentation change with zero code risk for these agents.

**Independent Test**: Contract tests assert the new "Recording Values from Non-Text Sources" instruction text appears exactly once (idempotently) in each of the 7 agents' rules/instructions files after `rgt init`.

**Acceptance Scenarios**:

1. **Given** `rgt init` for Codex, Windsurf, Cline, Antigravity, Kilo, Copilot CLI, or Mistral Vibe, **When** the instructions file is written, **Then** it contains the non-text-source reporting instruction, scoped to non-text formats only.
2. **Given** the instruction explicitly excludes plain-text/CSV/JSON/Markdown, **When** the instructions file is written, **Then** the text states those formats are already auto-captured (no extra agent action).
3. **Given** a re-run of `rgt init` (and `--force`), **When** the instructions file is refreshed, **Then** the instruction appears exactly once (no duplication, per 019's installer guarantees).

---

### User Story 3 - Claude Code Gets the Same Instruction (Priority: P2)

A Claude Code user currently gets hooks only — no instructions file. After this feature, `rgt init` writes a `CLAUDE.md` (project-scoped, or global with `--global`) containing the non-text-source reporting instruction, so Claude Code agents get the same guidance as the rules-tier agents.

**Why this priority**: Claude Code is a flagship integration (§6.2 "New"), and today it has no instructions artifact at all — so the non-text gap is entirely unaddressed for it.

**Independent Test**: Contract test asserts `rgt init --agent claude-code` writes a `CLAUDE.md` containing the instruction (idempotently, preserving any user content below/above RGT's block, per 019).

**Acceptance Scenarios**:

1. **Given** `rgt init --agent claude-code`, **When** it runs, **Then** a `CLAUDE.md` is written with the non-text-source instruction.
2. **Given** an existing `CLAUDE.md` with user content, **When** `rgt init --force` runs, **Then** the RGT instruction block is added/replaced without destroying user content.
3. **Given** `--global`, **When** `rgt init -g --agent claude-code` runs, **Then** the global `CLAUDE.md` is written instead of the project one.

---

### User Story 4 - Remaining Agents Verified Before Adding Instructions (Priority: P3)

Cursor, Copilot Chat (VS Code), and Gemini CLI also have hook-only integrations. Their instructions-file mechanisms (Cursor `.cursor/rules/*.mdc`, Copilot `.github/copilot-instructions.md`, Gemini `GEMINI.md`) are **not verified** — the findings doc requires live verification before implementing, not guessing. OpenCode/Pi/Hermes are plugin-only with no known instructions mechanism.

**Why this priority**: These are lower-confidence additions that must not ship based on a guessed file location/schema; the findings doc (§6.3) explicitly gates them on live-capture verification.

**Independent Test**: Out of scope for this feature — each is a follow-up that first verifies the agent's real instructions mechanism (same live-capture method used in §4/§8).

**Acceptance Scenarios**:

1. **Given** an unverified agent (Cursor, Copilot Chat, Gemini), **When** this feature ships, **Then** no instructions file is written for it (no guessed mechanism); the gap is documented as a follow-up.
2. **Given** OpenCode/Pi/Hermes, **When** this feature ships, **Then** they remain plugin-only with no instructions file (documented as unsupported for this path).

---

### Edge Cases

- What happens when `rgt record` is given a file that is valid UTF-8 but is actually a binary format that happens to decode (e.g. a text-based `.xlsx`)? (Out of scope — detection is non-UTF-8 only, per §3.1's "at minimum" fix.)
- What happens when the `CLAUDE.md` file already exists with an RGT block from a previous version without the new instruction (the §2.4 legacy-block concern)?
- What happens when the non-text instruction is added to a rules file that a user has heavily customized below RGT's block?
- What happens with `--stdin` (`rgt record --stdin` piped non-text bytes) — does the same unsupported-format detection apply?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt record` MUST detect non-UTF-8 (binary) content and return an explicit "unsupported file format for `rgt record`" error naming the supported path (plain text/CSV, or the agent's own read/extraction tool feeding the hook) — instead of the raw UTF-8 decode error.
- **FR-002**: A "Recording Values from Non-Text Sources" instruction MUST be appended to every rules/instructions artifact RGT already writes: Codex (`AGENTS.md`), Windsurf (`.windsurfrules`), Cline/Roo Code (`.clinerules`), Antigravity (`.agents/rules/antigravity-rgt-rules.md`), Kilo (`.kilocode/rules/rgt-rules.md`), Copilot CLI (`AGENTS.md`), and Mistral Vibe (`~/.vibe/prompts/rgt.md`).
- **FR-003**: `rgt init --agent claude-code` MUST write a `CLAUDE.md` (project-scoped by default, global with `--global`) containing the same instruction, using the existing RGT-marker idempotent-write helpers.
- **FR-004**: The instruction MUST be scoped narrowly to PDF, Excel (`.xlsx`), images, and other non-plain-text formats, and MUST explicitly state that plain-text/CSV/JSON/Markdown are already captured automatically (no action needed).
- **FR-005**: All instruction writes MUST be idempotent and preserve user content (no duplication; `--force` replaces only RGT's delimited block), consistent with the 019 installer guarantees.
- **FR-006**: No instructions file MUST be written for unverified agents (Cursor, Copilot Chat, Gemini) or plugin-only agents (OpenCode, Pi, Hermes); those gaps are documented as follow-ups requiring live verification, not guessed mechanisms.

### Key Entities *(include if feature involves data)*

- **Non-Text Source**: A binary/non-plain-text input (PDF, Excel `.xlsx`, images) whose values RGT's hook cannot see. Handled by explicit agent reporting, not native parsing.
- **Recording Instruction**: A short, scoped instruction block added to agent instructions files, telling the agent to report values from non-text sources via `rgt record --stdin` or an intermediate text file.
- **Instructions Artifact**: A file an agent reads for guidance — existing rules files, the new `CLAUDE.md`, or (unverified) future files for Cursor/Copilot/Gemini.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `rgt record` on a binary file returns a clear unsupported-format error (naming the supported path), not a raw UTF-8 message — asserted by a contract test with a non-UTF-8 fixture.
- **SC-002**: After `rgt init`, each of the 7 in-scope agents' instructions files contains the non-text-source instruction exactly once (idempotent), asserted by contract tests.
- **SC-003**: `rgt init --agent claude-code` writes a `CLAUDE.md` containing the instruction, preserving user content (byte-for-byte outside RGT's block), asserted by contract tests.
- **SC-004**: The instruction text explicitly scopes out plain-text/CSV/JSON/Markdown (asserted by string match in the contract tests).
- **SC-005**: 0 guessed instructions for unverified agents — no instructions file is written for Cursor/Copilot Chat/Gemini/OpenCode/Pi/Hermes in this feature.

## Assumptions

- The instruction text is the one drafted in the findings doc §6.1 (exact wording adapted slightly for placement), scoped to non-text sources and explicitly excluding plain-text formats.
- Scope is the findings §4/§6/§3.1 fixes: (1) the `rgt record` non-UTF-8 error (code), and (2) the per-agent instructions for the 7 high-confidence agents + Claude Code (`CLAUDE.md`). Cursor/Copilot Chat/Gemini instructions files and OpenCode/Pi/Hermes are **out of scope** pending live verification (FR-006).
- Native binary parsing (calamine for Excel, pdf-extract for PDFs — §3.1's "more complete" alternative) is out of scope for this feature; the error message + instruction path is the fix.
- Claude Code `CLAUDE.md` placement defaults to project-scoped, with `--global` writing the global `CLAUDE.md` (mirroring `write_claude_code`'s existing project/global split). This is a design default; if the user prefers global-only, it's a one-line change.
- The 019 installer's paired-marker + backup + per-agent-outcome guarantees apply to all new writes here (FR-005).
- Non-UTF-8 detection in `rgt record` covers the `--stdin` path too (FR-001), since `--stdin` can also receive binary bytes.
