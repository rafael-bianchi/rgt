# Tasks: Locale-Aware Extraction

**Input**: Design documents from `/specs/027-locale-aware-extraction/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/number-format-invariants.md, quickstart.md

**Tests**: Included — the spec's "Independent Test" per story and Constitution IX require test coverage.

**Organization**: Tasks grouped by user story (US1–US4) for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the baseline before any changes.

- [x] T001 Verify baseline: run `cargo build` and `cargo test --test test_hook_parsers --test test_record` and confirm they pass before changes; confirm no new dependencies are required (hand-rolled parsing per research.md §1).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and storage that every user story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T002 [P] Add `NumberFormat` enum (`Us`, `Eu`, `Auto` with `Default = Auto` and a `FromStr` accepting `us|eu|auto`) and the `ExtractionResult { values: Vec<ExtractedValue>, skipped_malformed: usize }` struct in `src/hooks/parser.rs`
- [x] T003 [P] Add `project_settings` table (`key TEXT PRIMARY KEY, value TEXT NOT NULL`) to `src/store/schema.rs` (idempotent `CREATE TABLE IF NOT EXISTS`)
- [x] T004 [P] Add `get_setting(conn, key) -> Option<String>` and `set_setting(conn, key, value)` helpers in `src/store/queries.rs`
- [x] T005 Change `extract_values_from_content` to `extract_values_from_content(content: &str, format: NumberFormat) -> ExtractionResult` and update the two call sites `src/cli/record.rs:69` and `src/hooks/mod.rs:88` to pass `NumberFormat::Auto` (placeholder, refined in later phases)
- [x] T006 Update existing tests in `tests/unit/test_record.rs` and `tests/contract/test_hooks_contract.rs` for the new signature and `auto` default (dates still emit `Date`, plain integers unchanged)

**Checkpoint**: Foundation ready — the enum, table, helpers, and new signature compile and existing tests pass.

---

## Phase 3: User Story 1 - Signs Are Part of the Number (Priority: P1) 🎯 MVP

**Goal**: `-42` → `-42`, `(500)` → `-500`, `+42` → `42` (locale-independent, always on).

**Independent Test**: `cargo test --test test_hook_parsers sign` — unit tests assert the four sign/paren inputs map to the correct signed values.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T007 [P] [US1] Add unit tests for sign/parentheses in `tests/unit/hook_parsers.rs` (`-42` → `-42`, `+42` → `42`, `(500)` → `-500`, `-(500)` → `500`)

### Implementation for User Story 1

- [x] T008 [US1] Implement sign and parentheses normalization in the number-candidate tokenizer/normalizer in `src/hooks/parser.rs` (leading `-`/`+` and enclosing `(...)` handled before separator normalization)

**Checkpoint**: Signs are captured; `cargo test --test test_hook_parsers` green.

---

## Phase 4: User Story 2 - Decimal and Thousands Separators (Priority: P1)

**Goal**: `1,234.56` (us) and `1.234,56` (eu) each yield one `1234.56` node; malformed values are skipped + counted, never split.

**Independent Test**: `cargo test --test test_hook_parsers separator` — asserts one-node extraction per format and malformed skip counts.

### Tests for User Story 2

- [x] T009 [P] [US2] Add unit tests for separator interpretation and adjacent-symbol handling in `tests/unit/hook_parsers.rs` (`1,234.56`/us → `1234.56`, `1.234,56`/eu → `1234.56`, `1,234`/us → `1234`, `1,234`/eu → `1.234`, `$1,234.56` → `1234.56`, `12.5%` → `12.5`, malformed `1.234.567,89`/us → `skipped_malformed == 1`)

### Implementation for User Story 2

- [x] T010 [US2] Implement separator normalization in `src/hooks/parser.rs` (strip thousands separator, require exactly-3-digit thousands groups, allow at most one decimal separator; count and skip malformed candidates) per research.md §4

**Checkpoint**: `us`/`eu` formats parse correctly and malformed candidates are counted (not emitted).

---

## Phase 5: User Story 3 - Format Persists for Passive Capture (Priority: P2)

**Goal**: `rgt init --number-format eu` persists; flagless `rgt record`/hook uses `eu`; explicit flag overrides.

**Independent Test**: `cargo test --test test_cli_record_derive_contract` — asserts init persistence, flagless resolution, and flag override.

### Tests for User Story 3

- [x] T011 [P] [US3] Add contract tests in `tests/contract/test_cli_record_derive_contract.rs` for `rgt init --number-format eu` persistence, flagless `record` using `eu`, and `rgt record --number-format us` overriding a persisted `eu`

### Implementation for User Story 3

- [x] T012 [US3] Add `--number-format <us|eu|auto>` flag to `Commands::Init` and `Commands::Record` in `src/main.rs` with a clap `value_parser` restricted to the three literals (invalid → exit code 2) and thread it into the handlers
- [x] T013 [US3] Update `execute_init` in `src/cli/init.rs` to persist the flag via `set_setting(conn, "number_format", ...)`; update `execute_record` in `src/cli/record.rs` to resolve the format (flag > setting > auto) and print a stdout warning when `skipped_malformed > 0`
- [x] T014 [US3] In `handle_passive_hook_event` (`src/hooks/mod.rs`), read the persisted `number_format` (via `get_setting`) and pass the resolved format to `extract_values_from_content` (silent skip — no warning, never blocks)

**Checkpoint**: Format persists and reaches both the record and passive hook paths with correct resolution precedence.

---

## Phase 6: User Story 4 - `auto` Infers the Locale (Priority: P2)

**Goal**: `auto` (the default) infers once per file: rightmost separator is the decimal when both `.`/`,` appear; single separator falls back to `us`.

**Independent Test**: `cargo test --test test_hook_parsers auto` — asserts per-file inference results.

### Tests for User Story 4

- [x] T015 [P] [US4] Add unit tests for `auto` inference in `tests/unit/hook_parsers.rs` (`1,234.56` → `1234.56`, `1.234,56` → `1234.56`, `1,234` → `1234`, `1.234` → `1.234`, and a two-line file using one format consistently)

### Implementation for User Story 4

- [x] T016 [US4] Implement `auto` pre-pass inference (once per file) and resolution to a concrete format in `src/hooks/parser.rs`, applying the resolved format to every line (research.md §5)

**Checkpoint**: `auto` (default) matches all contract cases; `cargo test --test test_hook_parsers` green.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation.

- [x] T017 [P] Document `--number-format` in `README.md` and `AGENTS.md` (Constitution X)
- [x] T018 Run quickstart.md scenarios, then `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; fix any regressions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS all user stories.
- **User Stories (Phase 3–6)**: Depend on Foundational.
  - US1 and US2 are both P1 (sequential within `parser.rs` — US2 builds on US1's tokenizer).
  - US3 (persistence/CLI) is independent of US1/US2 implementation and can proceed in parallel once Foundational is done.
  - US4 (auto) depends on US2's concrete-format normalizer.
- **Polish (Phase 7)**: Depends on all desired stories.

### User Story Dependencies

- **US1 (P1)**: after Foundational; no other-story deps.
- **US2 (P1)**: after US1 (extends the tokenizer); independently testable.
- **US3 (P2)**: after Foundational; independent of US1/US2 (only needs `ExtractionResult` from T002/T005).
- **US4 (P2)**: after US2 (reuses the concrete-format normalization).

### Within Each User Story

- Tests written first and observed failing, then implementation.
- Models/structs before services, core before integration.

### Parallel Opportunities

- T002/T003/T004 (different files) in parallel.
- T007/T009/T015 (all in `tests/unit/hook_parsers.rs` but different test functions — write sequentially to avoid edit conflicts, or parallel with care).
- US3 (T011–T014) can run in parallel with US1+US2 (T007–T010) once Foundational is done.

---

## Parallel Example: User Story 3 (independent of US1/US2)

```bash
# Persistence/CLI path (parallel with the parser work in US1/US2):
Task: "Contract tests for init/record persistence in tests/contract/test_cli_record_derive_contract.rs"
Task: "Add --number-format flag to Commands::Init/Record in src/main.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002–T006) — CRITICAL
3. Phase 3: US1 signs (T007–T008)
4. **STOP and VALIDATE**: `cargo test --test test_hook_parsers` — signs captured
5. Then US2 separators (T009–T010), US3 persistence (T011–T014), US4 auto (T015–T016)

### Incremental Delivery

1. Foundational → compile + existing tests green
2. US1 → signs correct (MVP)
3. US2 → separators correct per `us`/`eu`
4. US3 → format persists to the passive path
5. US4 → `auto` default matches contract cases
6. Polish → docs + fmt/clippy/full test pass

---

## Notes

- `parser.rs` is touched by T002, T005, T008, T010, T016 — keep these sequential in that order.
- `tests/unit/hook_parsers.rs` gains new test functions across US1/US2/US4 — prefer sequential writes to avoid edit conflicts.
- Default format is `auto`; this changes existing extraction on `1,234.56`-style inputs (intended — spec Assumptions).
- Commit after each task or logical group; stop at checkpoints to validate independently.
