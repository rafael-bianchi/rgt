# Tasks: Human-Readable Duration Display

**Input**: Design documents from `/specs/008-human-readable-durations/`

**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Tests are explicitly required per spec acceptance criteria — `test_date_diff.rs` must be updated with assertions on the new format.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Primary change file: `src/types/value.rs`
- Test file: `tests/unit/test_date_diff.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: No setup needed — the project structure, dependencies, and test harness are already in place. Proceed directly to user story implementation.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: None needed — the codebase already has all required infrastructure. The `ValueData` enum, `chrono::Duration` dependency, and test harness are in place.

**⚠️ No tasks in this phase** — proceed directly to user story implementation.

**Checkpoint**: Foundation ready for user story work.

---

## Phase 3: User Story 1 - Agent Reads a Derived Duration Value (Priority: P1) 🎯 MVP

**Goal**: `ValueData::Duration` displays as human-readable `Xd Xh Xm` / `Xh Xm Xs` / `Xm Xs` / `Xs` format instead of raw seconds. Trailing zero units are omitted; middle zero units are preserved.

**Independent Test**: Build, then record two dates 14 days apart, derive a Duration node, and query it — output shows `14d` not `1209600s`.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T003 [US1] Add format assertions to existing `test_date_difference_calculation` test in `tests/unit/test_date_diff.rs` — assert that `ValueData::Duration(dur).to_string_repr()` returns `10d` for 864,000s (10 days)
- [x] T004 [P] [US1] Add test for exact-day duration (`14d`) without trailing units in `tests/unit/test_date_diff.rs`
- [x] T005 [P] [US1] Add test for hour-range duration (`2h 15m 30s`) in `tests/unit/test_date_diff.rs`
- [x] T006 [P] [US1] Add test for minute-range duration (`5m 30s`) in `tests/unit/test_date_diff.rs`
- [x] T007 [P] [US1] Add test for sub-minute duration (`45s`) in `tests/unit/test_date_diff.rs`
- [x] T008 [P] [US1] Add test for zero duration (`0s`) in `tests/unit/test_date_diff.rs`
- [x] T009 [P] [US1] Add test for large duration beyond 365 days (e.g., 34,560,000s = `400d`) in `tests/unit/test_date_diff.rs`

### Implementation for User Story 1

- [x] T010 [US1] Replace `format!("{}s", dur.num_seconds())` with human-readable formatter in `ValueData::to_string_repr()` Duration variant in `src/types/value.rs` — implement unit decomposition: days (>=86,400s), hours (>=3,600s), minutes (>=60s), seconds otherwise; omit trailing zero units; preserve middle zeros
- [x] T011 [US1] Run `cargo test test_date_diff -- --test-threads=1` to verify all US1 tests pass

**Checkpoint**: All Duration values display in human-readable format. Tests pass for US1.

---

## Phase 4: User Story 2 - Negative Duration Display (Priority: P2)

**Goal**: Negative durations display with a `-` prefix (e.g., `-5d` or `-30s`).

**Independent Test**: Record two Date nodes with the later date first, derive the duration, verify output starts with `-`.

### Tests for User Story 2

- [x] T012 [US2] Add test for negative multi-day duration (`-5d`) in `tests/unit/test_date_diff.rs`
- [x] T013 [P] [US2] Add test for negative sub-minute duration (`-30s`) in `tests/unit/test_date_diff.rs`

### Implementation for User Story 2

- [x] T014 [US2] Add negative sign handling to `ValueData::to_string_repr()` Duration variant in `src/types/value.rs` — detect `num_seconds() < 0`, prepend `-`, format absolute value
- [x] T015 [US2] Run `cargo test test_date_diff -- --test-threads=1` to verify all US2 tests pass

**Checkpoint**: Negative durations display correctly with `-` prefix. US1 tests still pass.

---

## Phase 5: User Story 3 - Doc Comment for Year/Month Precision Limitation (Priority: P3)

**Goal**: `date_diff()` has a doc comment warning developers that `chrono::Duration` cannot represent years/months precisely, directing them to use calendar-aware methods on original Date nodes for year/month display.

**Independent Test**: Inspect `src/types/value.rs` — the `date_diff()` function has a doc comment with the required content.

### Implementation for User Story 3

- [x] T016 [US3] Add doc comment to `date_diff()` in `src/types/value.rs` stating: `chrono::Duration` has no `num_years()` or `num_months()` methods; callers needing year/month display should use calendar-aware methods like `chrono::NaiveDate::years_since()` on the original Date nodes rather than the derived Duration node

**Checkpoint**: Doc comment is present and accurate.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Verification and integration validation

- [x] T017 Run `cargo fmt --all -- --check` to verify formatting
- [x] T018 Run `cargo clippy --all-targets` to verify no clippy warnings
- [x] T019 Run `cargo test --all -- --test-threads=1` to verify all tests pass
- [x] T020 [P] Run quickstart.md validation scenarios — verify `rgt query <node_id>` shows `14d` (not `1209600s`), `rgt status --json` includes `"value": "14d"`, `rgt graph` outputs show human-readable format
- [x] T021 [P] Run `cargo doc --no-deps` and verify `date_diff()` doc comment renders correctly

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — project already initialized
- **Foundational (Phase 2)**: Empty — no blocking prerequisites
- **User Story 1 (Phase 3)**: Can start immediately after Setup
- **User Story 2 (Phase 4)**: Depends on US1 implementation (T010) — extends the same function
- **User Story 3 (Phase 5)**: Independent of US1/US2 — different function, same file
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1)**: No dependencies on other stories
- **US2 (P2)**: Depends on US1 T010 (the human-readable formatter must exist before adding sign handling to it). Tests (T012, T013) can be written in parallel with US1 tests.
- **US3 (P3)**: Independent — doc comment on `date_diff()`, separate from `to_string_repr()`. Can be done in parallel with US1.

### Within Each User Story

- Tests written first, verified to FAIL before implementation
- Implementation follows tests
- Verification after implementation

### Parallel Opportunities

- **Phase 1**: No tasks — proceed directly to Phase 3
- **US1 Tests**: T004, T005, T006, T007, T008, T009 are on the same file but test different cases — written together as one batch update
- **US2 Tests vs US1 Implementation**: T012 and T013 can be written alongside US1 test updates
- **US3 (T016)**: Can be done in parallel with US1 implementation (different function in same file)
- **Phase 6**: T020 and T021 can run in parallel

---

## Parallel Example: US1

```bash
# After T003 creates the format assertion skeleton:
# T004-T009 add test cases to the same file — batch as one edit

# US3 can run independently in parallel with US1:
Task: "Add doc comment to date_diff() in src/types/value.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (no tasks — project already initialized)
2. Complete Phase 3: User Story 1 (T003-T011)
3. **STOP and VALIDATE**: Run `cargo test` and verify `rgt query` output shows `14d` format
4. Feature is usable as MVP — agents see human-readable durations

### Incremental Delivery

1. Setup → verify current state
2. Add US1 → human-readable format (MVP!)
3. Add US2 → negative sign prefix
4. Add US3 → doc comment
5. Polish → fmt, clippy, full test suite, quickstart validation
6. Each story adds value without breaking previous stories

### Single Developer Strategy

Since all changes are in 2 files and US1/US2 share the same function:
- Implement US1 (T003-T011) — this is the core change
- Immediately add US2 sign handling (T012-T015) — natural extension of the same code
- Add US3 doc comment (T016) — separate function, can be done at any point
- Polish (T017-T021) — verify everything

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- US1 and US2 share the same `to_string_repr()` function — implement together for efficiency
- US3 is on `date_diff()` — separate function, genuinely independent
- The `plan.md` decision to modify `to_string_repr()` directly means the format change propagates automatically to all 11 call sites — no per-channel changes needed
- Commit after each phase or logical group
- All changes are in 2 files: `src/types/value.rs` and `tests/unit/test_date_diff.rs`

---

## Phase 7: Convergence

**Purpose**: Close remaining gaps identified by `/speckit.converge`

- [x] T022 [P] [US1] Add test for middle-zero preservation (`1h 0m 5s`) in `tests/unit/test_date_diff.rs` per spec.md:64 Edge Cases (partial)
