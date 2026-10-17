# Tasks: Automatic PDF Provenance Capture via `additionalContext`

**Input**: Design documents from `/specs/033-pdf-capture-via-additionalcontext/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/pdf-capture-contract.md, quickstart.md

**Tests**: Included — Constitution IX requires envelope, emission, and malformed-input coverage.

**Organization**: Single user story; the work is one additive hook-path extension.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependencies and baseline.

- [x] T001 Add `base64 = "0.23"` and `pdf-extract = "0.12"` to `[dependencies]` in `Cargo.toml`; register new test targets `test_pdf_extract` (path `tests/unit/test_pdf_extract.rs`) and `test_pdf_capture_contract` (path `tests/contract/test_pdf_capture_contract.rs`); confirm `cargo build` succeeds and existing tests are green

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The isolated extract module + envelope detection every other change builds on.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [x] T002 Create `src/extract/pdf.rs`: `pub fn extract_pdf_text(decoded: &[u8]) -> Option<String>` — calls `pdf-extract::extract_text_from_mem(decoded)` inside `std::panic::catch_unwind`, returning `Some(text)` (even if empty) or `None` on panic; add `pub mod extract;` to `src/lib.rs`
- [x] T003 Extend `NormalizedCapture` in `src/hooks/parser.rs` with `pub pdf_base64: Option<String>` (Default stays derivable); extend `default_dialect` to detect `tool_response.type == "pdf"` with `file.base64` when `content` is absent, setting `pdf_base64` (and leaving `content` None so the disk-read fallback is not taken)

**Checkpoint**: `cargo build` compiles; existing tests still pass (non-PDF paths unaffected).

---

## Phase 3: User Story 1 - PDF Values Are Captured With Zero Agent Action (Priority: P1) 🎯 MVP

**Goal**: PDF envelopes record values and (for claude-code) inject extracted text.

**Independent Test**: `cargo test --test test_pdf_extract --test test_pdf_capture_contract`.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T004 [US1] Unit tests in `tests/unit/test_pdf_extract.rs`: (a) a claude-code PDF envelope produces a parseable `hookSpecificOutput.additionalContext` containing the extracted text; (b) a non-claude-code agent produces no stdout; (c) truncation over `MAX_CONTEXT_CHARS` appends the explicit marker; (d) `extract_pdf_text` returns `None` (no panic) on garbage/truncated bytes (fuzz-style loop) and empty string for a no-text-layer input

### Implementation for User Story 1

- [x] T005 [US1] Add `MAX_CONTEXT_CHARS` (8000) and a `render_additional_context(text: &str) -> String` helper (truncation + marker, or the no-text-layer notice) — either in `src/extract/pdf.rs` or a small `src/hooks/mod.rs` helper (keep it a pure function for testability)
- [x] T006 [US1] Wire the PDF path into `handle_passive_hook_event` in `src/hooks/mod.rs`: when `capture.pdf_base64` is present, base64-decode (`BASE64_STANDARD`), `extract_pdf_text`, run `extract_values_from_content` + the existing `insert_tracked_node` transaction (source doc upsert uses the envelope's filePath); then, only for `--agent claude-code`, print the `hookSpecificOutput.additionalContext` JSON to stdout
- [x] T007 [US1] Contract test in `tests/contract/test_pdf_capture_contract.rs`: build a minimal real PDF (with a known value like `Total: 1234.56`), base64 it, feed the exact captured envelope via the hook path, assert the known value appears in the store; a malformed-base64 envelope asserts fail-open (exit 0, no nodes)

**Checkpoint**: `cargo test --test test_pdf_extract --test test_pdf_capture_contract` green.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Docs and final validation.

- [x] T008 Update the README's non-text-source note (and the installed `NON_TEXT_SOURCE_INSTRUCTION` glue text) to state that Claude Code PDF reads are now captured automatically via `additionalContext`, while Excel/images and other agents still use the `rgt record --stdin` fallback (Constitution X)
- [x] T009 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; measure/report the binary size delta from the two new dependencies

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T002/T003).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story.

### Within User Story 1

- Tests first (T004/T007), then implementation (T005/T006).

### Parallel Opportunities

- T004 (unit) and T007 (contract) touch different files; both depend on T005/T006.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002–T003)
3. Phase 3: US1 (T004–T007)
4. **STOP and VALIDATE**: pdf tests green

### Incremental Delivery

1. Foundational → extract module + envelope detection
2. US1 → recording + injection wired end-to-end
3. Polish → docs + fmt/clippy/full test + binary-size note

---

## Notes

- Recording half is agent-agnostic; injection is claude-code-only.
- The `additionalContext` cap is a centralized constant (8000), not a flag.
- The hook's fail-open contract must hold on every error path (never crash, never block, never non-zero exit from hook logic).
- Use the in-memory `extract_text_from_mem` (confirmed on docs.rs) — no temp-file round-trip.
