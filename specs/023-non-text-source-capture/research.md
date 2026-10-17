# Research: Non-Text Source Capture

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Consolidates the findings-doc research (§4/§6/§3.1) — the platform reality is already established, so no new external research is required.

## 1. Why instructions, not hook code (the decisive constraint)

**Decision**: The fix is **agent instructions** + a clear `rgt record` error — not native PDF/Excel parsing in the hook path.

**Rationale** (findings §4, verified against official docs):
- Claude Code (and agents built the same way) do **not** run local PDF/image text extraction: PDF support is server-side vision, base64 → model → synthesized text, never written down locally. The `PostToolUse` hook receives a structured envelope (`{type:"pdf", file:{filePath, base64, originalSize}}`) with **no `content` string** — confirmed by a live-captured payload, not inference.
- Therefore no hook, for any agent, ever sees extracted text for a binary/multimodal file. The only place non-text content becomes text is inside the model's own response.
- Hence the robust, uniform-across-all-agents fix is: tell the agent to explicitly report values from non-text sources (write them to a plain-text file or pipe via `rgt record --stdin`). This depends only on the universally-supported "write plain text" capability.

**Alternatives considered**:
- Hook-side decode of `tool_response` embedded bytes + embedded `pdf-extract`/`calamine` (§4 "optional fragile fallback") — rejected as primary: it must be built/tested per-agent against each agent's undocumented envelope shape, adds heavy binary-format crates, and still fails for scanned/image-only PDFs (no text layer). Noted as future work, not this feature.
- Native `rgt record` binary parsing (§3.1 "more complete alternative", `calamine`/`pdf-extract`) — rejected for this feature: out of scope; the "at minimum" fix (clear error) is adopted.

## 2. `rgt record` non-UTF-8 detection

**Decision**: In `execute_record`, read the input as bytes and `String::from_utf8`; on failure return an explicit "unsupported file format for `rgt record`" error naming the supported path. Apply to both the file path and `--stdin` (read stdin as bytes too).

**Rationale**: The current `fs::read_to_string`/`io::read_to_string` surface a raw "stream did not contain valid UTF-8" that gives no recovery path. Detecting non-UTF-8 and returning an actionable message satisfies §3.1's "at minimum" fix with no new dependencies.

**Alternatives considered**: format sniffing by magic bytes (PDF `%PDF`, xlsx ZIP `PK`) — rejected: non-UTF-8 detection is sufficient and simpler; a UTF-8 file that's actually a binary format is out of scope per the spec.

## 3. Instruction placement

**Decision**: Add `glue::NON_TEXT_SOURCE_INSTRUCTION` (the findings §6.1 text, adapted) and splice it into each rules block at write time, before the end marker; use it as the body of a new Claude Code `CLAUDE.md` block.

**Rationale**: RGT already writes a paired-marker block to each of the 7 rules artifacts (Codex `AGENTS.md`, Windsurf `.windsurfrules`, Cline `.clinerules`, Antigravity, Kilo, Copilot CLI `AGENTS.md`, Vibe `rgt.md`); appending the instruction inside the block (before `RGT_END_MARKER`) keeps it idempotent and content-preserving via the existing `ensure_text_block`/`--force` machinery (019). Claude Code's instructions file is the well-established `CLAUDE.md` memory file (project root, or `~/.claude/CLAUDE.md` global) — a data-only addition mirroring the rules-tier pattern.

**Alternatives considered**: separate per-agent instruction files for the unverified agents (Cursor `.cursor/rules/*.mdc`, Copilot `.github/copilot-instructions.md`, Gemini `GEMINI.md`) — rejected for this feature (FR-006): their exact mechanism is unverified per §6.2/§6.3 and must not be guessed.
