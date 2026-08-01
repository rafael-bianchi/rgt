# Tasks: Remove Python Scaffolding Files

**Input**: Design documents from `/specs/006-remove-python-scaffolding/`

**Prerequisites**: plan.md, spec.md (1 user story: P1 clean project identity)

**Tests**: Not applicable — no code changes, only file deletions.

---

## Phase 1: User Story 1 - Clean Rust-Only Project Identity (Priority: P1) 🎯 MVP

**Goal**: Delete stale Python scaffolding files and update `.gitignore` so the repo presents as 100% Rust.

**Independent Test**: `ls main.py pyproject.toml .python-version` returns "No such file or directory". `.gitignore` contains `.specify/` and `.agents/` entries. `cargo test --all` passes.

### Implementation

- [x] T001 [P] [US1] Delete `main.py` from repository root
- [x] T002 [P] [US1] Delete `pyproject.toml` from repository root
- [x] T003 [P] [US1] Delete `.python-version` from repository root
- [x] T004 [P] [US1] Add `.specify/` entry to `.gitignore`
- [x] T005 [P] [US1] Add `.agents/` entry to `.gitignore`

### Validation

- [x] T006 [US1] Run `cargo build` to verify build succeeds after deletions
- [x] T007 [US1] Run `cargo test --all` to verify no regressions
- [x] T008 [US1] Run `find . -not -path './target/*' -name "main.py" -o -name "pyproject.toml" -o -name ".python-version"` to verify no Python files remain

**Checkpoint**: Repository is clean — zero Python artifacts, `.gitignore` updated.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (US1)**: All 8 tasks. T001-T005 can run in parallel (different files). T006-T008 run sequentially after T001-T005.

### Parallel Opportunities

```text
Phase 1 - Parallel deletions:
  Task: T001 (delete main.py)
  Task: T002 (delete pyproject.toml)
  Task: T003 (delete .python-version)
  Task: T004 (add .specify/ to .gitignore)
  Task: T005 (add .agents/ to .gitignore)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Delete three Python files (T001-T003)
2. Update `.gitignore` (T004-T005)
3. Validate build and tests (T006-T008)

This is the full scope — no further phases needed.

---

## Notes

- Zero code changes — `cargo test` passes trivially
- `.specify/` is already in `Cargo.toml exclude`; adding to `.gitignore` is redundancy, not duplication
- The three Python files total 235 bytes — no impact on build time or binary size
