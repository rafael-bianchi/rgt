# Research: Automatic PDF Provenance Capture via `additionalContext`

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `docs/spec-033-pdf-capture-via-additionalcontext.md` (the deep spec, verified live in-session), `src/hooks/parser.rs`, `src/hooks/mod.rs`, and direct API verification of `pdf-extract`/`base64` on docs.rs (fetched this session).

## 1. Envelope recognition (FR-001, FR-006)

**Decision**: Extend `default_dialect` (`src/hooks/parser.rs:331-342`) so that when `tool_response.content` is absent, it checks for the PDF envelope shape: `tool_response.type == "pdf"` with a `tool_response.file.base64` string. When matched, surface the raw base64 (or the decoded intent) in the `NormalizedCapture` so the hook can process it. A new field on `NormalizedCapture` (e.g. `pdf_base64: Option<String>`) carries the base64; `content` stays `None` so the existing disk-read fallback path is NOT taken for PDFs.

**Rationale**: The deep spec §4 warns the envelope is an undocumented internal detail that could change; detecting it defensively (pattern-match, don't assume field presence) and falling through cleanly when it doesn't match preserves the fail-open contract (FR-005/FR-006). The exact captured shape (`tool_response.type == "pdf"`, `file.filePath`, `file.base64`, `file.originalSize`) is pinned by the fixture test.

**Alternatives considered**: A separate `PdfCapture` variant instead of a field — rejected: a field keeps the change additive and the existing `{path, content}` flow untouched.

## 2. Extraction: in-memory, no temp file (FR-002, Assumptions)

**Finding (new, supersedes the deep spec's §3.3 assumption)**: `pdf-extract` exposes `extract_text_from_mem(buffer: &[u8])` (confirmed on docs.rs this session) — an in-memory variant that avoids the temp-file round-trip the deep spec assumed was required. This is strictly better: no `NamedTempFile`, no disk write, no cleanup.

**Decision**: Base64-decode via `base64::prelude::BASE64_STANDARD.decode(...)`, then call `extract_text_from_mem(&decoded)`. Both new deps are pinned (`base64 = "0.23"`, `pdf-extract = "0.12"`), matching the project's pinning practice.

**Rationale**: Fewer moving parts, matches the project's minimal-dependency ethos; the deep spec explicitly said "if an in-memory variant exists, prefer it over the temp-file round-trip."

## 3. Fail-open and malformed-input safety (FR-005, SC-003)

**Finding**: The deep spec's Risks table flags that `pdf-extract`'s behavior on malformed/corrupt PDFs is undocumented (happy path only), and this project has precedent (evalexpr) for a third-party crate being driveable into a crash by adversarial input.

**Decision**: Wrap the extraction call in `std::panic::catch_unwind` in `src/extract/pdf.rs`, returning a `None`/`Err` on panic. Base64 decode failure returns cleanly. A timeout is not added (extraction of a hook-sized PDF is fast; the deep spec lists catch_unwind as mitigation (b) and timeout as (c) — catch_unwind is the minimal sufficient guard). The malformed-input fuzz test (SC-003) feeds garbage bytes and truncated base64 and asserts no panic.

**Rationale**: Constitution XI "zero unhandled panics"; the hook must never crash the agent's tool call. `catch_unwind` is the low-cost guard the deep spec recommends.

## 4. Recording half (FR-002, FR-007)

**Decision**: In `handle_passive_hook_event`, when `capture.pdf_base64` is present: decode → extract → run `extract_values_from_content(text, number_format)` → record via the existing `insert_tracked_node` transaction path (identical to the text path). The source document upsert uses the envelope's `filePath`. This is agent-agnostic: any agent whose dialect surfaces the envelope records values (FR-007).

**Rationale**: Reuses the existing, tested recording path; the "record first" ordering means values land in the graph even if stdout injection fails or the agent doesn't support it (deep spec §3.1 step 3).

## 5. Stdout injection (FR-003, FR-004, FR-007)

**Decision**: Only for `--agent claude-code`, after recording, print a `hookSpecificOutput.additionalContext` JSON object to stdout. Shape:

```json
{"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": "<truncated text or no-text notice>"}}
```

- Non-empty text: `additionalContext` is the extracted text truncated to `MAX_CONTEXT_CHARS` (centralized constant, 8000) with `\n[... truncated: N characters omitted; full text was still extracted and recorded to the provenance graph ...]\n` appended when truncated.
- Empty text: `additionalContext` = `RGT: no text layer found in this PDF; provenance capture requires the agent to extract values manually via rgt record --stdin`.
- Non-claude-code agents emit nothing to stdout (FR-007).

**Rationale**: The deep spec verified (twice, live) that `additionalContext` is injected into the model's context automatically. This is the first code path in the hook that writes to stdout — reviewed carefully against the "fail open with no stdout for every other case" contract (only the PDF+claude-code combination prints).

## 6. Test strategy (SC-001..SC-004)

**Decision**:
- `tests/contract/test_pdf_capture_contract.rs`: build a minimal real PDF (via `pdf-extract`'s own test fixtures or a hand-built PDF with known text), base64 it, feed the exact captured envelope through the hook path, assert the known values appear in the store. A malformed-base64 envelope asserts fail-open (no panic, exit 0, no nodes).
- `tests/unit/test_pdf_extract.rs`: `additionalContext` JSON emission for claude-code; no stdout for another agent; truncation marker on long text; `catch_unwind` guard on garbage bytes (fuzz-style loop over pseudo-random/truncated inputs).

**Rationale**: Constitution IX; the envelope fixture is the deep spec's recommended regression pin for Claude Code shape drift.
