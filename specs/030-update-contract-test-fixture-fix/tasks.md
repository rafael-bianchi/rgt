# Tasks: Update Contract Test Fixture Fix

**Input**: Design documents from `/specs/030-update-contract-test-fixture-fix/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/fixture-invariants.md, quickstart.md

**Tests**: Included — this feature IS a test-fixture repair (Constitution IX).

**Organization**: Single user story; the work is one tightly-scoped change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the red baseline before any change.

- [x] T001 Verify baseline: run `cargo test --test test_update_contract` and confirm exactly the two known failures (`genuine_newer_tag_proceeds_past_gating`, `failed_verification_never_reaches_replace`) with the other 11 passing; confirm `semver` is already a direct dependency in `Cargo.toml` (used by `src/updater/version.rs`).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The version-derivation helper every change depends on.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [x] T002 Add `fn newer_tag_for(current: &str) -> String` and `fn newer_stub_tag() -> String` helpers to the `mod tests` block of `tests/contract/test_update_contract.rs`: parse `current` via `semver::Version`, return `v{major}.{minor + 1}.0`; `newer_stub_tag()` calls `newer_tag_for(env!("CARGO_PKG_VERSION"))`. `expect` with a clear message on unparseable input (fail loudly, never silently wrong).

**Checkpoint**: Helpers compile; existing (still-failing) tests unchanged.

---

## Phase 3: User Story 1 - The Test Suite Is Green Again (Priority: P1) 🎯 MVP

**Goal**: Both "proceeds past gating" tests pass at version 0.5.0 and on any future version bump.

**Independent Test**: `cargo test --test test_update_contract` — all 13 tests green.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T003 [US1] Add unit tests for `newer_tag_for` (asserting strict-newer via `semver` comparison for `0.5.0`, `1.2.3`, `0.0.1`, `0.5.0-rc.1`) and for `newer_stub_tag()` (asserts it is strictly newer than `CARGO_PKG_VERSION`) in `tests/contract/test_update_contract.rs`

### Implementation for User Story 1

- [x] T004 [US1] Switch `genuine_newer_tag_proceeds_past_gating` to `stub_release(&newer_stub_tag(), true)` in `tests/contract/test_update_contract.rs`
- [x] T005 [US1] Switch `failed_verification_never_reaches_replace` to use `newer_stub_tag()` for both its checksum-mismatch and attestation-failure stubs in `tests/contract/test_update_contract.rs`

**Checkpoint**: `cargo test --test test_update_contract` reports all 13 passing; `git diff --stat -- src/` is empty.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [x] T006 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; confirm no regressions and no production-code changes (`git status` shows only `tests/` + this spec directory)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T002).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story — the whole feature.

### Within User Story 1

- Test tasks written first (T003), then fixture switches (T004/T005).

### Parallel Opportunities

- None meaningful — all changes land in the same test file, so they run sequentially.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002)
3. Phase 3: US1 (T003–T005)
4. **STOP and VALIDATE**: `cargo test --test test_update_contract` green, `git diff --stat -- src/` empty

### Incremental Delivery

1. Foundational → helpers in place
2. US1 → the two failing tests pass via derived stub tags
3. Polish → fmt/clippy/full test

---

## Notes

- This is a P0 unblock: the suite must go green before any other change on this branch is trusted.
- The helper derives from `CARGO_PKG_VERSION` so the fix does not re-break on the next version bump (spec FR-002, SC-003).
- Production code (`src/`) must remain untouched (spec FR-001, SC-002).
