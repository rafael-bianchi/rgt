# Implementation Plan: Automatic PDF Provenance Capture via `additionalContext`

**Branch**: `feat/033-pdf-capture-via-additionalcontext` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/033-pdf-capture-via-additionalcontext/spec.md`

## Summary

Extend the passive hook to recognize Claude Code's PDF envelope (`tool_response.type == "pdf"` with `file.base64`), base64-decode it, run local `pdf-extract` text extraction, record extracted values into the provenance graph (agent-agnostic), and — for `--agent claude-code` — inject the extracted text back via `PostToolUse`'s `additionalContext` (truncated to a centralized cap, with an explicit marker when truncated; a scanned-PDF no-text notice is emitted instead of the current silent failure). All errors fail open.

## Technical Context

**Language/Version**: Rust (existing project; edition 2021).

**Primary Dependencies**: `base64` (0.23.x, new), `pdf-extract` (0.12.x, new), `tempfile` (existing). Both new crates confirmed on crates.io and licensed compatibly (MIT/Apache-2.0 ecosystem).

**Storage**: SQLite via the existing `insert_tracked_node` path.

**Testing**: Contract test with the exact captured envelope + a real base64 PDF; unit tests for the `additionalContext` emission and truncation; a fuzz/malformed-input test for the extraction call path.

**Target Platform**: macOS / Linux / Windows (single static binary).

**Project Type**: CLI (single Rust binary).

**Performance Goals**: PDF extraction is on the hook path but only for PDF envelope events (rare vs. text reads); no budget change to the normal text path.

**Constraints**: Constitution I/XI (new deps are justified and lightweight); the hook's fail-open contract must hold (never crash, never block, never non-zero exit from hook logic); recording half agent-agnostic, injection Claude-Code-only.

**Scale/Scope**: `src/hooks/parser.rs` (envelope detection in `default_dialect`), `src/hooks/mod.rs` (new decode→extract→record→emit flow), new `src/extract/pdf.rs`, `Cargo.toml`, tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | New deps are pure-Rust crates (`base64`, `pdf-extract`); no runtime engine. |
| II. Two-tier detection + BLAKE3 | PASS | PDF recording hashes the decoded/raw bytes consistently (existing pattern). |
| III. Graph correctness | PASS | Values recorded via the existing insert path. |
| IV. Rich value types | PASS | Extracted text goes through existing `extract_values_from_content`. |
| V. Distribution / RTK UX | PASS | Binary grows modestly; acceptable per positioning (deep spec's §3.3). |
| VI. Dual passive/active surface | PASS | This is the passive hook path, upgraded. |
| VII. Open-source governance | PASS | New crates are permissively licensed. |
| VIII. Passive hooks, thin delegates | PASS | Logic lives in the Rust binary; the hook remains a thin invocation. |
| IX. Test-driven verification | PASS | Envelope fixture test, malformed-input fuzz, emission unit tests. |
| X. Documentation matches behavior | PASS | README non-text-source note updated to reflect automatic PDF capture for Claude Code. |
| XI. Idiomatic Rust, exit codes | PASS | Fail-open; `catch_unwind` guard on the unverified extraction call. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/033-pdf-capture-via-additionalcontext/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── pdf-capture-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
src/
├── extract/
│   └── pdf.rs           # NEW: decode + pdf-extract + error handling (testable)
├── hooks/
│   ├── parser.rs        # default_dialect: detect pdf envelope
│   └── mod.rs           # handle_passive_hook_event: decode→extract→record→emit
├── lib.rs               # add `pub mod extract`
└── main.rs              # none (hook wiring unchanged)

Cargo.toml               # add base64, pdf-extract

tests/
├── unit/
│   └── test_pdf_extract.rs   # NEW: envelope detection, additionalContext emission, truncation, malformed-input fuzz
└── contract/
    └── test_pdf_capture_contract.rs  # NEW: end-to-end envelope → store nodes
```

**Structure Decision**: Single-project CLI. A new `src/extract/pdf.rs` module isolates decode+extract+error handling from the hook orchestration, matching the deep spec's §3.5 and keeping it unit-testable without the hook I/O.

## Complexity Tracking

> None — no constitution violations requiring justification. The `catch_unwind` guard is a defensive measure on an unverified third-party call (Constitution XI "zero unhandled panics"), not a violation.
