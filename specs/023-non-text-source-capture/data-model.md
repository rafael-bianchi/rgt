# Data Model: Non-Text Source Capture

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for the non-text-source gap. No new persisted entities — the change is a clear-error path plus embedded instruction text.

## Entities

### NonTextSource

A binary/non-plain-text input RGT's passive hook cannot see into.

| Attribute | Type | Notes |
|---|---|---|
| `kind` | `enum` | `Pdf`, `Excel`, `Image`, `OtherBinary` — user-facing labels in the error/instruction. |
| `recordable_natively` | `bool` | Always `false` for `rgt record` (only plain text/CSV are natively parsed). |

Rules:
- `rgt record` on a non-UTF-8 (binary) input returns a clear "unsupported file format" error (FR-001).
- Values from a `NonTextSource` are captured only via explicit agent reporting (FR-002/FR-003), never by the passive hook.

### RecordingInstruction

The embedded instruction block telling agents to report non-text-source values.

| Attribute | Type | Notes |
|---|---|---|
| `text` | `const &str` | `glue::NON_TEXT_SOURCE_INSTRUCTION` — the findings §6.1 text, scoped to PDF/Excel/images. |
| `excludes` | `&str` | Explicitly states plain-text/CSV/JSON/Markdown are auto-captured (FR-004). |

Placement:
- Appended inside the RGT block of each of the 7 in-scope rules artifacts (Codex, Windsurf, Cline, Antigravity, Kilo, Copilot CLI, Vibe).
- Body of a new Claude Code `CLAUDE.md` block (project `CLAUDE.md`, or `~/.claude/CLAUDE.md` with `--global`).

### InstructionsArtifact

A file an agent reads for guidance, written by `rgt init`.

| Attribute | Type | Notes |
|---|---|---|
| `path` | `PathBuf` | Per-agent (existing rules files; new `CLAUDE.md`). |
| `block` | `String` | The RGT block (open marker + content + instruction + end marker). |

Rules:
- Idempotent (no duplication); `--force` replaces only RGT's delimited block; user content preserved byte-for-byte (FR-005, per 019).
- No artifact is written for unverified/plugin-only agents (FR-006).

## Validation rules (from FR-001..FR-006)

- **Clear error**: non-UTF-8 → explicit unsupported-format message (FR-001).
- **Instruction coverage**: 7 rules artifacts + `CLAUDE.md` (FR-002/FR-003).
- **Narrow scope**: PDF/Excel/images only; plain-text excluded (FR-004).
- **Idempotency/preservation** (FR-005).
- **No guessed mechanisms** (FR-006).

## Out of scope

Native binary parsing (`calamine`/`pdf-extract`), hook-side `tool_response` decode, and instructions for Cursor/Copilot Chat/Gemini/OpenCode/Pi/Hermes (all follow-ups requiring verification).
