# Feature Specification: Automatic PDF Provenance Capture via `additionalContext`

**Feature Branch**: `feat/033-pdf-capture-via-additionalcontext`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Close the §4 native-PDF gap fully automatically — `rgt hook post` decodes the PDF's base64 envelope, runs local `pdf-extract` text extraction, records provenance nodes with zero agent action required, and (for Claude Code) injects the real extracted text back via `PostToolUse`'s `additionalContext` field."

Source context: `docs/spec-033-pdf-capture-via-additionalcontext.md` (the dedicated deep spec — read in full). Plan: `docs/spec-plan-followups-from-verification.md`, spec #4, P1.

## Clarifications

### Session 2026-08-31

- Q: The deep spec's §3.4 proposes a token-cost mitigation cap (`--max-context-chars`, default ~8000) on the injected `additionalContext`. Where should this live? → A: Default 8000 characters, applied only to the stdout `additionalContext` injection (the *recording* half is never truncated). The cap value is a constant for now (no new CLI flag/setting in this spec — that adds surface area to an already-large change; the constant is centralized for a future setting). Rationale: the graph already holds the full text-derived values; the injection is only to inform the model.
- Q: The deep spec's §3.3 notes `pdf-extract::extract_text` takes a file path, so decoded bytes must go to a temp file first. Is the temp-file round-trip acceptable, or must we find an in-memory variant? → A: Accept the temp-file round-trip via `tempfile::NamedTempFile` (already a dependency). The decoded bytes are written once and immediately read back by `pdf-extract`; this matches the updater's existing download-then-verify pattern and needs no new dependency. Confirm at implementation time whether an in-memory variant exists, but do not block on it.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - PDF Values Are Captured With Zero Agent Action (Priority: P1)

A developer (via Claude Code) reads a PDF invoice containing numeric and date values. Today the passive hook receives a `tool_response` shaped `{type: "pdf", file: {filePath, base64, originalSize}}` with no `content` string; the disk-read fallback fails on the binary, and **zero values are recorded, silently**. After this fix, the hook decodes the base64, extracts the PDF's text layer locally, records the values into the provenance graph automatically, and additionally injects the real extracted text into the model's context via `PostToolUse`'s `additionalContext` so Claude Code can answer precisely instead of relying solely on its vision pass.

**Why this priority**: P1 per the plan — closes a confirmed total gap with a mechanism strictly better than spec 023's instruction-based fallback (no agent cooperation required), and the core mechanism was empirically verified live twice in-session (including proving `additionalContext` reaches the model).

**Independent Test**: A contract test feeds the exact captured `{type:"pdf", file:{...}}` envelope (with a real base64 PDF containing known values) into the hook path and asserts `rgt status` shows the PDF's values as tracked nodes, plus a unit test asserts the `additionalContext` JSON is emitted for `--agent claude-code`.

**Acceptance Scenarios**:

1. **Given** a Claude Code hook event with `tool_response` shaped `{type:"pdf", file:{filePath, base64, originalSize}}` and no `content`, **When** `rgt hook post --agent claude-code` processes it, **Then** the PDF's numeric/date values are recorded as provenance nodes (visible in `rgt status`) with no agent action.
2. **Given** the same event, **When** the hook runs, **Then** stdout carries a `hookSpecificOutput.additionalContext` JSON object containing the extracted PDF text (truncated to the cap with an explicit marker when truncated).
3. **Given** a PDF with no extractable text layer (scanned/image-only), **When** the hook runs, **Then** an `additionalContext` is still emitted noting "no text layer found; use `rgt record --stdin`" rather than failing silently, and no nodes are recorded from empty text.
4. **Given** a malformed base64 string or a corrupt PDF, **When** the hook runs, **Then** it fails open (no crash, exit 0, no stdout pollution beyond an optional notice), consistent with the hook's existing fail-open contract.

---

### Edge Cases

- **Non-claude-code agents**: the `additionalContext` emission is gated to `--agent claude-code` only (Cursor's equivalent mechanism is unverified); the *recording* half is agent-agnostic and runs for any agent whose dialect surfaces the PDF envelope shape.
- **Envelope shape drift**: the `{type:"pdf", file:{...}}` shape is an undocumented internal detail; if it doesn't match, fall through to the existing fail-open disk-read fallback (no error).
- **Empty extraction**: `pdf-extract` returns an empty string for scanned PDFs (not an error) — handled explicitly per acceptance scenario 3.
- **Corrupt/malicious PDF**: `pdf-extract`'s behavior on garbage input is undocumented — the call is wrapped defensively (see Risks) so a crash cannot take down the hook.
- **Path mismatch**: the envelope's `filePath` may differ from what an agent previously recorded; the node IDs derive from the envelope's `filePath` (consistent with the source document upsert).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt hook post` MUST recognize the PDF envelope shape (`tool_response.type == "pdf"` with `file.base64`) when `tool_response.content` is absent, decode the base64, and run local PDF text extraction.
- **FR-002**: When extraction yields non-empty text, the hook MUST record numeric/date values from it into the provenance graph via the existing `extract_values_from_content` + `insert_tracked_node` path, exactly as any other captured content (agent-agnostic).
- **FR-003**: For `--agent claude-code`, the hook MUST emit a `hookSpecificOutput.additionalContext` JSON object on stdout containing the extracted text, truncated to a centralized character cap (default 8000) with an explicit `[... truncated ...]` marker when truncated.
- **FR-004**: When extraction yields empty text (scanned PDF), the hook MUST emit an `additionalContext` noting the absence of a text layer and pointing to `rgt record --stdin`, instead of the current silent failure.
- **FR-005**: The PDF path MUST fail open on any error (malformed base64, extraction failure, corrupt input): no crash, no blocking, no non-zero exit from the hook's own logic — consistent with the existing hook contract.
- **FR-006**: Non-PDF captures MUST be completely unaffected (existing behavior preserved); the new path is additive.
- **FR-007**: The `additionalContext` emission MUST be gated to `--agent claude-code`; the recording half MUST NOT be gated (agent-agnostic).

### Key Entities *(include if feature involves data)*

- **PdfEnvelope**: the `{type:"pdf", file:{filePath, base64, originalSize}}` shape inside `tool_response`.
- **ExtractedPdfText**: the output of local text extraction (may be empty for scanned PDFs).
- **AdditionalContext**: the `hookSpecificOutput.additionalContext` stdout injection for Claude Code.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A contract test with the exact captured envelope shape records the PDF's known values as nodes (asserted via the store), and a malformed/shape-mismatched envelope degrades to the existing fail-open path without panicking.
- **SC-002**: A unit test asserts `--agent claude-code` emits a parseable `hookSpecificOutput.additionalContext` containing the extracted text; a non-claude-code agent emits no stdout.
- **SC-003**: A fuzz/malformed-input test confirms the `pdf-extract` call path does not panic or hang on garbage input (per the deep spec's Risks table requirement).
- **SC-004**: Existing hook tests remain green; `cargo fmt`/`clippy`/full `cargo test` pass.

## Assumptions

- `pdf-extract` (0.12.x on crates.io) and `base64` (0.23.x) are the correct crates (both vetted in the deep spec and growth research).
- The temp-file round-trip for decoded bytes is acceptable (no in-memory `pdf-extract` variant assumed).
- The `additionalContext` cap is a centralized constant (8000 chars) for now, not a user-facing flag.
- The `{type:"pdf", file:{...}}` envelope is treated as a documented-in-tests assumption; a fixture test pins it so drift is caught in CI.

## Out of scope

- Excel (`.xlsx`) / image capture via the same mechanism (explicitly not verified; the deep spec flags these as needing their own live-capture verification).
- Cursor `additionalContext` support (unverified; recording works for any agent, injection is Claude-Code-only).
- A user-facing `--max-context-chars` flag or persisted setting (constant only, per clarification).
- Replacing spec 023's instruction-based fallback (it stays as the safety net for agents/scenarios this path doesn't cover).
