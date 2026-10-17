# Data Model: Automatic PDF Provenance Capture via `additionalContext`

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

## 1. PdfEnvelope (transient, input)

The Claude Code `tool_response` shape for a native PDF read:

| Field | Type | Meaning |
|-------|------|---------|
| `type` | string (`"pdf"`) | marks a binary PDF envelope |
| `file.filePath` | string | the PDF path (used for node IDs + source-document upsert) |
| `file.base64` | string | the raw PDF bytes, base64-encoded |
| `file.originalSize` | integer | original byte size (informational) |

## 2. NormalizedCapture (extended)

Gains `pdf_base64: Option<String>`. When present, the hook takes the PDF path instead of the disk-read fallback.

## 3. ExtractedPdfText (transient)

`pdf-extract::extract_text_from_mem(&decoded)` output. Empty string ⇒ scanned/image-only PDF (no text layer) — handled as a "no text layer" notice, not an error.

## 4. AdditionalContext (transient, output, claude-code only)

`hookSpecificOutput.additionalContext` — extracted text truncated to `MAX_CONTEXT_CHARS` (8000) with an explicit marker, or the no-text-layer notice. The graph always holds the full recorded values regardless of truncation.
