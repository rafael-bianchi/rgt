# Tasks: Documentation Reconciliation

**Input**: Design documents from `/specs/012-documentation-reconciliation/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Verification-only — documentation changes are verified via `cargo doc` and grep checks, not new unit tests.

**Organization**: Tasks grouped by user story. Note: the reconciliation implementation was performed during the specify phase (doc comments added, README/AGENTS verified, CONTRIBUTING created, .gitignore updated). These tasks formalize, verify, and commit that work.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths

## Phase 1: Setup

**Purpose**: Verify current state. No initialization needed.

- [x] T001 Run the doc-comment coverage check: `grep -rn "^pub fn" src/ --include="*.rs"` — confirm every match has a preceding `///` line
- [x] T002 [P] Verify git state — confirm `.kilocode/` and `.kilo/` are gitignored and no tooling dirs are untracked

---

## Phase 2: Foundational

**Purpose**: None needed — documentation-only changes, no blocking infrastructure.

**Checkpoint**: Ready for verification of each user story.

---

## Phase 3: User Story 1 — README Matches Actual CLI (Priority: P1)

**Goal**: README documents all 8 CLI subcommands (`init`, `status`, `query`, `graph`, `hook`, `mcp`, `update`, `verify`) with accurate flags matching `src/main.rs`.

**Independent Test**: `for cmd in init status query graph hook mcp update verify; do grep -ci "$cmd" README.md; done` — each returns ≥1.

### Verification

- [x] T003 [US1] Verify all 8 CLI subcommands are documented in `README.md` (CLI Reference section covers init, status, query, graph, hook, mcp, update, verify)
- [x] T004 [US1] Verify install commands in `README.md` match current distribution: `curl .../install.sh | sh`, `brew install rafael-bianchi/rgt/rgt`, `cargo install --git https://github.com/rafael-bianchi/rgt`

**Checkpoint**: README is accurate against code.

---

## Phase 4: User Story 2 — AGENTS.md Matches Code (Priority: P1)

**Goal**: AGENTS.md CLI tables and install paths match current code.

**Independent Test**: Grep AGENTS.md for CLI commands — each exists in `src/main.rs`.

### Verification

- [x] T005 [US2] Verify AGENTS.md CLI Commands table matches `src/main.rs` (init, status, query, graph, hook, mcp, update — and verify was added)
- [x] T006 [US2] Verify AGENTS.md install commands use `rafael-bianchi/rgt/rgt` and `cargo install --git`

**Checkpoint**: AGENTS.md is accurate.

---

## Phase 5: User Story 3 — Code Comments Document Public API (Priority: P2)

**Goal**: Every `pub fn` in `src/` has a `///` doc comment. `cargo doc --no-deps` builds with zero warnings.

**Independent Test**: The grep check from T001 returns zero `MISSING:` lines. `cargo doc --no-deps` has zero warnings.

### Verification

- [x] T007 [US3] Confirm doc comments exist for all `pub fn` in `src/verify/`, `src/updater/`, `src/cli/`, `src/hooks/`, `src/detection/`, `src/store/`, `src/mcp/`
- [x] T008 [US3] Run `cargo doc --no-deps` — confirm zero warnings (no `broken_intra_doc_links`)

**Checkpoint**: Public API fully documented.

---

## Phase 6: Polish & Commit

**Purpose**: Final verification and commit of the documentation work.

- [x] T009 Run `cargo fmt --all -- --check` to verify formatting
- [x] T010 Run `cargo clippy --all-targets` to verify no new errors
- [x] T011 Run `cargo test --all -- --test-threads=1` to verify no regressions
- [x] T012 Run quickstart.md validation scenarios — confirm all 6 pass (doc coverage, cargo doc, README commands, install paths, gitignore, tests)
- [x] T013 [P] Verify CONTRIBUTING.md accuracy — confirm it lists the real test command (`cargo test --all -- --test-threads=1`), current module structure (src/cli, src/verify, src/store, etc.), and the branch-from-develop workflow (FR-005)
- [x] T014 Stage and commit: `README.md`, `AGENTS.md` (if changed), `CONTRIBUTING.md`, `.gitignore`, `src/**/*.rs` doc comments, `specs/012-documentation-reconciliation/` with conventional commit message

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **US1 (Phase 3)**: Independent — README verification
- **US2 (Phase 4)**: Independent — AGENTS.md verification
- **US3 (Phase 5)**: Independent — doc comment verification
- **Polish (Phase 6)**: Depends on all user stories

### Parallel Opportunities

- **Phase 1**: T001 and T002 run in parallel
- **Phases 3, 4, 5**: All independent — README, AGENTS.md, and doc comments are verified separately
- **Phase 6**: T009-T012 run in parallel (independent checks)

---

## Implementation Strategy

### Verification-first

Since the implementation was completed during specify, this phase is verification and commit:

1. Phase 1 → confirm baseline state
2. Phases 3-5 → verify each user story's claims against code
3. Phase 6 → full check suite + commit

---

## Notes

- No new runtime tests needed — documentation-only changes.
- The `src/mcp/mod.rs` trailing-comma change and `tests/contract/test_update_contract.rs` formatting were pre-existing uncommitted changes from the previous feature — decide whether to include or exclude in the commit.
- Commit message suggestion: `docs: reconcile documentation with code behavior (README, AGENTS, CONTRIBUTING, doc comments)`
