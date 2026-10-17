# Tasks: Value Extraction Correctness

**Input**: Design documents from `/specs/022-value-extraction-correctness/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- Existing modules: `src/hooks/parser.rs`, `src/types/node.rs`, `src/cli/record.rs`, `src/hooks/mod.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the dependency baseline (no new crates)

- [x] T001 Confirm `regex` is already in `[dependencies]` in `Cargo.toml` (no new dependencies are required — see `research.md`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `occurrence` discriminator on `ExtractedValue` and the `generate_root_id` signature change that every user story depends on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `src/types/node.rs` `#[cfg(test)]`: (a) `generate_root_id(f, line, 0, v)` is deterministic and equals the pre-change `file+line+value` hash; (b) `generate_root_id(f, line, 1, v) != generate_root_id(f, line, 0, v)` for the same `v` (collision fixed)

### Implementation for Foundational Primitives

- [x] T003 Add `occurrence: u32` to `ExtractedValue` in `src/hooks/parser.rs` (all constructors set it, defaulting to 0 for now); change `TrackedNode::generate_root_id` in `src/types/node.rs` to take an `occurrence: u32` parameter and hash it only when `> 0`; update the two call sites `src/cli/record.rs` (`generate_root_id(file, ext.line_number, ext.occurrence, &ext.value)`) and `src/hooks/mod.rs` (`generate_root_id(path, ext.line_number, ext.occurrence, &ext.value)`)

**Checkpoint**: `generate_root_id` occurrence tests pass; the code compiles with occurrence threaded through both call sites (still all-zero)

---

## Phase 3: User Story 1 - A Date Emits One Date Node, Never Spurious Numbers (Priority: P1) 🎯 MVP

**Goal**: A recognized ISO-8601 date emits exactly one `Date` and zero `Number` nodes (FR-001); the date fixture asserts the exact total count (FR-005)

**Independent Test**: `extract_values_from_content("start: 2026-01-15")` returns exactly 1 node (a `Date`); the three-date fixture returns exactly 3

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T004 [P] [US1] Update `tests/unit/test_record.rs::test_extract_dates_from_content` to assert `extracted.len() == 3` (all `Date`); add a test asserting `extract_values_from_content("start: 2026-01-15\n")` returns exactly 1 node and it is a `Date`; add a test for an ISO datetime `2026-01-15T12:30:00Z` returning exactly 1 `Date`

### Implementation for User Story 1

- [x] T005 [US1] In `extract_values_from_content` (`src/hooks/parser.rs`), collect each line's ISO-date match spans (byte `[start, end)`) first, then skip any number match whose byte range lies within a date span before emitting it as a `Number`

**Checkpoint**: US1 tests pass; no spurious number nodes from recognized dates

---

## Phase 4: User Story 2 - Identical Values on the Same Line Are Tracked Separately (Priority: P1)

**Goal**: Two identical values on one line produce two distinct root nodes via the occurrence discriminator (FR-002)

**Independent Test**: `extract_values_from_content("revenue,120000,discount,120000")` returns two values with occurrences 0 and 1 (and, at the node level, two distinct IDs)

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T006 [P] [US2] Unit test in `tests/unit/test_record.rs`: `extract_values_from_content("revenue,120000,discount,120000\n")` returns two values, both `Number(120000)`, with occurrences 0 and 1; a control line `a,100,b,200` still returns two values with occurrences 0 and 1; additionally assert that `generate_root_id(file, line, 0, &v) != generate_root_id(file, line, 1, &v)` for the two same-line identical values (the full SC-003 chain: occurrences → distinct node IDs)

### Implementation for User Story 2

- [x] T007 [US2] In `extract_values_from_content` (`src/hooks/parser.rs`), assign a per-line occurrence counter (0-based, incrementing in emission order — dates first, then non-masked numbers) to each emitted `ExtractedValue`

**Checkpoint**: US2 tests pass; same-line identical values carry distinct occurrences (and therefore distinct node IDs)

---

## Phase 5: User Story 3 - Extraction Is Deterministic and Re-Recording Dedupes (Priority: P2)

**Goal**: Identical content yields identical occurrences and node IDs; single-value lines keep pre-change IDs (FR-003/FR-004)

**Independent Test**: Repeated extraction of the same content yields identical `(value, line_number, occurrence)` triples; the occurrence-0 ID equals the pre-change formula

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T008 [P] [US3] Unit test in `tests/unit/test_record.rs`: run `extract_values_from_content` twice on a mixed content sample (dates + numbers, including a same-line repeat) and assert the two result vectors are equal (determinism); assert the occurrence-0 root ID for a single-value line matches the pre-change `file+line+value` hash (backward compatibility, FR-003)

**Checkpoint**: US3 tests pass; determinism and backward compatibility verified (no further implementation — these are properties asserted by the tests)

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation

- [x] T009 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T010 Run the manual scenarios in `specs/022-value-extraction-correctness/quickstart.md` (no spurious date numbers; same-line identical values tracked separately)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: None
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational (occurrence field exists; masking is independent)
- **US2 (Phase 4)**: Depends on Foundational (assigns non-zero occurrences)
- **US3 (Phase 5)**: Depends on US1 + US2 (asserts the combined behavior)
- **Polish (Phase 6)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: after Foundational — date masking
- **US2 (P1)**: after Foundational — occurrence assignment (shares `parser.rs` with US1, so run sequentially)
- **US3 (P2)**: after US1 + US2 — determinism/backward-compat assertions

### Within Each User Story

- Tests written first, must FAIL before implementation
- Foundational signature change before story work; masking (US1) before occurrence (US2) within `extract_values_from_content`
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 (node.rs test) ∥ T004/T006/T008 (record.rs tests) — different files
- T009 (fmt/clippy) after implementation

---

## Parallel Example: Foundational + Story Tests

```bash
# Foundational test (node.rs):
Task: "generate_root_id occurrence tests (src/types/node.rs)"

# Story tests (tests/unit/test_record.rs), after the foundational impl:
Task: "exact-count date tests (US1)"
Task: "same-line collision tests (US2)"
Task: "determinism + backward-compat tests (US3)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (no-op) + Phase 2 (Foundational: occurrence field + ID param)
2. Phase 3: User Story 1 (date masking + exact-count test)
3. **STOP and VALIDATE**: no spurious number nodes from dates (the §3.3 bug is fixed)
4. Then US2/US3 incrementally

### Incremental Delivery

1. Foundational → occurrence plumbing in place
2. US1 → date→number cross-contamination fixed (MVP)
3. US2 → same-line identical values tracked separately
4. US3 → determinism + backward compatibility verified
5. Polish → fmt/clippy/full-suite + manual validation

---

## Notes

- [P] tasks = different files, no dependencies; US1/US2 both edit `src/hooks/parser.rs`, so their implementation tasks run sequentially
- [Story] label maps task to user story for traceability
- Occurrence-0 IDs are backward compatible (FR-003) — no schema/ID migration needed for existing data
- No `main.rs`/CLI-flag changes
- Commit after each task or logical group; stop at each checkpoint
