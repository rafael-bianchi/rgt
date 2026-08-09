# Tasks: Remove Unused sha2 Dependency

**Input**: Design documents from `specs/007-remove-unused-sha2/`

**Prerequisites**: spec.md ✅, plan.md ✅, research.md ✅, data-model.md ✅, quickstart.md ✅

**Tests**: Not applicable — investigation-only task with no code changes.

**Organization**: Single user story (P1) confirming sha2 usage.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Verification Environment)

**Purpose**: Confirm current state of the repository

- [x] T001 Verify sha2 dependency declared in Cargo.toml at line 32: `sha2 = "0.10"`
- [x] T002 Verify sha2 has no secondary declarations in Cargo.toml (e.g., under [dev-dependencies] or [build-dependencies])

---

## Phase 2: User Story 1 - Confirm dependency usage (Priority: P1) 🎯 MVP

**Goal**: Verify whether `sha2` is actively referenced in source code. If unused, remove it. If used, document finding and close issue #4.

**Independent Test**: `grep -rn "sha2\|Sha256\|Sha512" src/` returns results confirming usage in `src/updater/checksum.rs`.

### Investigation for User Story 1

- [x] T003 [US1] Run `grep -rn "sha2\|Sha256\|Sha512" src/` to confirm sha2 references in src/updater/checksum.rs
- [x] T004 [US1] Confirm updater module links checksum.rs via `mod updater` in src/main.rs and src/lib.rs
- [x] T005 [US1] Verify sha2 usage chain: src/updater/github.rs calls checksum::parse_checksums, checksum::find_checksum, and checksum::verify_checksum
- [x] T006 [US1] Run `cargo build --release` to confirm compilation succeeds with sha2 present
- [x] T007 [US1] Run `cargo test --all` to confirm all tests pass, including src/updater/checksum.rs unit tests
- [x] T008 [US1] Update GitHub issue #4 with investigation findings: sha2 is actively used by updater checksum verification
- [x] T009 [US1] Close GitHub issue #4 as "not a bug" via `gh issue close 4 --reason "not planned" --comment "..."`

**Checkpoint**: Issue #4 closed with documented finding. No code changes applied.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — verification-only, can start immediately
- **Phase 2 (User Story 1)**: T003-T005 can run in parallel (independent grep/read operations); T006-T007 depend on T003 confirmation; T008-T009 depend on all prior tasks

### Parallel Opportunities

- T001 and T002 can run in parallel (different files)
- T003, T004, T005 can run in parallel (independent investigations)
- T006 and T007 can run in parallel (different cargo commands)

---

## Parallel Example: User Story 1

```bash
# Launch all investigation tasks together:
Task: "T003: grep sha2 usage in src/"
Task: "T004: confirm mod updater in main.rs + lib.rs"
Task: "T005: verify checksum calls in github.rs"

# Then verify builds:
Task: "T006: cargo build --release"
Task: "T007: cargo test --all"
```

---

## Implementation Strategy

### MVP (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T002)
2. Complete Phase 2: Investigation (T003-T007)
3. Complete Phase 2: Closure (T008-T009)
4. **DONE**: No code changes; issue documented and closed

---

## Notes

- This feature produces zero code changes — all tasks are verification and issue closure
- The `sha2` dependency serves a functionally distinct purpose from `blake3`: release checksum verification vs file-change detection
- Constitution Principle I (Rust-Only Single Binary) is not violated; sha2 is a compile-time dependency
