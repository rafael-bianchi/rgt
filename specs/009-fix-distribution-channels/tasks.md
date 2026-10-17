# Tasks: Fix Distribution Channels

**Input**: Design documents from `/specs/009-fix-distribution-channels/`

**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Tests are optional — no new tests needed. Existing `cargo test --all` verifies no regressions.

**Organization**: Tasks are grouped by user story. US5 (CI fix) runs first to unblock all PRs, then remaining stories run in parallel.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/`, `.github/workflows/` at repository root
- Documentation: `README.md`, `AGENTS.md`, `install.sh`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Verify current state. No project initialization needed.

- [x] T001 Verify current CD workflow state — list stuck/pending runs on `main` via `gh run list --workflow cd.yml --branch main --limit 5`
- [ ] T002 [P] Verify current clippy errors in `tests/unit/test_date_diff.rs` via `cargo clippy --all-targets 2>&1 | grep -E "erasing_op|identity_op"`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: None needed — infrastructure exists. Proceed to user stories.

**Checkpoint**: Foundation ready.

---

## Phase 3: User Story 5 - CI Passes on All Branches (Priority: P1) 🎯 MVP

**Goal**: Fix `clippy::erasing_op` + `clippy::identity_op` in `tests/unit/test_date_diff.rs:35`. Remove dead code from `src/updater/github.rs`. CI must pass on all branches.

**Independent Test**: Run `cargo clippy --all-targets` — zero errors/warnings from test_date_diff.rs and updater/github.rs.

### Implementation for User Story 5

- [ ] T003 [US5] Fix `0 * 60 + 5` erasing_op and `1 * 3_600` identity_op in `tests/unit/test_date_diff.rs:35` — replace `Duration::seconds(1 * 3_600 + 0 * 60 + 5)` with `Duration::seconds(3_600 + 5)`
- [ ] T004 [P] [US5] Remove dead code from `src/updater/github.rs` — delete unused fields `size`, `name`, `draft`, `prerelease`, `published_at`, unused struct `UpdateCheckResult`, and unused function `check_update`
- [ ] T005 [US5] Run `cargo clippy --all-targets` to verify zero errors

**Checkpoint**: CI green. All branches unblocked for PRs.

---

## Phase 4: User Story 1 - Shell Installer Error Messages (Priority: P1)

**Goal**: `install.sh` error messages for unsupported platforms reference `cargo install --git https://github.com/rafael-bianchi/rgt` instead of bare `cargo install rgt`.

**Independent Test**: Grep `install.sh` for `cargo install rgt` (without `--git`) — must return zero matches.

### Implementation for User Story 1

- [ ] T006 [US1] Replace `cargo install rgt` with `cargo install --git https://github.com/rafael-bianchi/rgt` on lines 139 and 152 of `install.sh`

**Checkpoint**: Shell installer error messages use correct Cargo command.

---

## Phase 5: User Story 2 + 4 - Homebrew Formula & CD Workflow (Priority: P1)

**Goal**: Replace the `homebrew` job's `repository-dispatch` to `rafael-bianchi/homebrew-tap` with an in-repo step that computes SHA256 digests from release artifacts and commits updated `Formula/rgt.rb` back to the main repo. Cancel stuck CD runs on `main`.

**Independent Test**: Inspect `release.yml` — no `repository-dispatch` or `rafael-bianchi/homebrew-tap` references remain. The `homebrew` job uses `actions/checkout` and commits to the main repo.

### Implementation for User Story 2 + 4

- [ ] T007 [US2][US4] Replace `homebrew` job in `.github/workflows/release.yml:211-235` — remove `peter-evans/repository-dispatch@v3` step, replace with: checkout repo → compute SHA256s from downloaded artifacts → sed-replace placeholders in `Formula/rgt.rb` → commit and push
- [ ] T008 [US4] Cancel all stuck/pending CD runs on `main` via GitHub Actions UI or `gh run cancel`

**Checkpoint**: CD workflow ready. Homebrew formula updated in-repo during releases.

---

## Phase 6: User Story 3 + 6 - Documentation & Cargo Install Commands (Priority: P2)

**Goal**: All active documentation uses correct install commands: `brew install rafael-bianchi/rgt/rgt` and `cargo install --git https://github.com/rafael-bianchi/rgt`. No references to `rafael-bianchi/tap`, bare `cargo install rgt`, or `homebrew-tap` remain.

**Independent Test**: `grep -r "rafael-bianchi/tap" README.md AGENTS.md install.sh` returns zero matches. `grep "cargo install rgt" README.md AGENTS.md | grep -v "\-\-git"` returns zero matches.

### Implementation for User Story 3 + 6

- [ ] T009 [P] [US6] Update Homebrew install command in `README.md:32` — change `brew install rafael-bianchi/tap/rgt` to `brew install rafael-bianchi/rgt/rgt`
- [ ] T010 [P] [US6] Update Cargo install command in `README.md:37` — change `cargo install rgt` to `cargo install --git https://github.com/rafael-bianchi/rgt`
- [ ] T011 [P] [US6] Update Homebrew install command in `AGENTS.md:238` — change `brew install rafael-bianchi/tap/rgt` to `brew install rafael-bianchi/rgt/rgt`
- [ ] T012 [P] [US6] Update Cargo install command in `AGENTS.md:239` — change `cargo install rgt` to `cargo install --git https://github.com/rafael-bianchi/rgt`

**Checkpoint**: Documentation uses correct install commands.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Full verification of all changes.

- [ ] T013 Run `cargo fmt --all -- --check` to verify formatting
- [ ] T014 Run `cargo clippy --all-targets` to verify zero errors
- [ ] T015 Run `cargo test --all -- --test-threads=1` to verify all tests pass
- [ ] T016 Run `grep -rn "rafael-bianchi/tap" README.md AGENTS.md install.sh` to verify zero tap references
- [ ] T017 Run `grep -rn "cargo install rgt" README.md AGENTS.md install.sh | grep -v "\-\-git"` to verify zero bare cargo install references
- [ ] T018 Run quickstart.md Scenario 4 — verify `release.yml` homebrew job structure

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — verify current state
- **Foundational (Phase 2)**: Empty
- **US5 - CI Fix (Phase 3)**: Can start immediately — unblocks all PRs
- **US1 - Shell Installer (Phase 4)**: Independent — different file from US5
- **US2+4 - Homebrew + CD (Phase 5)**: Independent — different files from US5/US1
- **US3+6 - Docs + Cargo (Phase 6)**: Independent — different files from all above
- **Polish (Phase 7)**: Depends on all user stories

### User Story Dependencies

- **US5 (P1)**: No dependencies — fix CI first
- **US1 (P1)**: No dependencies on other stories
- **US2+4 (P1)**: No dependencies on other stories (changes `release.yml`, doesn't touch code)
- **US3+6 (P2)**: No dependencies on other stories (changes docs only)

### Parallel Opportunities

- **Phase 1**: T001 and T002 run in parallel
- **Phase 3**: T004 runs in parallel with T003 (different files)
- **Phases 4, 5, 6**: All three phases can run in parallel — they touch completely different file sets:
  - US1: `install.sh`
  - US2+4: `.github/workflows/release.yml`
  - US3+6: `README.md`, `AGENTS.md`
- **Phase 6 internally**: T009-T012 all run in parallel (4 independent edits)

---

## Parallel Example: Maximum Parallelism

```bash
# After Phase 3 (CI fix) completes:
# Launch all remaining user stories simultaneously:

# Agent A: Shell installer
Task: "Fix install.sh error messages (T006)"

# Agent B: CD workflow + Homebrew
Task: "Replace homebrew dispatch with in-repo commit (T007), cancel stuck runs (T008)"

# Agent C: Documentation
Task: "Update README.md and AGENTS.md install commands (T009-T012)"
```

---

## Implementation Strategy

### MVP First (CI Fix Only)

1. Complete Phase 1: Setup
2. Complete Phase 3: US5 (CI fix — T003-T005)
3. **STOP and VALIDATE**: `cargo clippy --all-targets` passes, `cargo test --all` passes
4. CI is green — all PRs unblocked

### Incremental Delivery

1. Setup → verify current state
2. US5 → CI green (MVP!)
3. US1 → Shell installer fixed
4. US2+4 → Homebrew + CD fixed
5. US3+6 → Docs updated
6. Polish → full verification

### Single Developer Strategy

Since all user stories after US5 touch different files:
1. Fix CI first (US5) — unblocks everything
2. Batch US1 (install.sh) + US3+6 (docs) as one commit — all documentation changes
3. Do US2+4 (CD workflow) as a separate commit — workflow changes need careful review
4. Polish

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- US5 must run first to unblock CI for all other branches
- US1, US2+4, US3+6 are fully independent — can run in any order after US5
- No new tests needed — existing `cargo test --all` provides regression coverage
- T008 (cancel stuck CD runs) is a manual GitHub Actions UI task
- `src/updater/github.rs` dead code removal (T004) — delete unused code, don't wire the updater
