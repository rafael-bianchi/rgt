# Contract: PDF Capture

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/unit/test_pdf_extract.rs` and `tests/contract/test_pdf_capture_contract.rs`.

## I. Envelope recognition (FR-001, FR-006)

- A `tool_response` with `type == "pdf"`, `file.filePath`, and `file.base64` (no `content`) is recognized and processed.
- A non-PDF payload is completely unaffected (existing text/Bash paths unchanged).

## II. Recording (FR-002, SC-001)

- The PDF's known numeric/date values are recorded as provenance nodes (asserted via the store).
- Node IDs derive from the envelope's `filePath`.

## III. Stdout injection (FR-003, FR-004, FR-007, SC-002)

- `--agent claude-code` emits parseable `hookSpecificOutput.additionalContext` containing the extracted text.
- Truncation over `MAX_CONTEXT_CHARS` appends an explicit marker naming the omitted-character count.
- A no-text-layer PDF emits the `rgt record --stdin` notice.
- Any other agent emits no stdout for a PDF event.

## IV. Fail-open (FR-005, SC-003)

- Malformed base64: no panic, no non-zero exit, no nodes recorded.
- Garbage/corrupt PDF bytes: no panic or hang (catch_unwind guard; fuzz-style test).
- Envelope-shape drift: falls through to the existing fail-open disk-read fallback, no error.

## V. Non-regression (Constitution IX, XI)

- Existing hook tests green; `cargo fmt --all -- --check`, `cargo clippy --all-targets`, full `cargo test` pass.
