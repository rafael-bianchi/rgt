# Tasks: Fix CD Release Automation & Develop-to-Main Promotion

**Input**: Design documents from `specs/016-fix-cd-promotion/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: No unit/contract/integration tests (this is a GitHub Actions workflow change). Validation is via live CD runs per quickstart.md scenarios — these are the "tests" for this feature. Each user story phase includes its verification commands.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Establish the current CD state before changes

- [x] T001 Confirm current `target-branch: main` in `.github/workflows/cd.yml` line 99 (the flaw to fix)
- [x] T002 Verify current branch sync: `git fetch origin main develop` and `git rev-list --count origin/main..origin/develop` — record the baseline (expected 0 after recent promotion)
- [x] T003 Identify the stale release-please remote branch: `git ls-remote --heads origin | grep -i release-please` — confirm `release-please--branches--develop--components--rgt` exists

---

## Phase 2: User Story 1 - Release-Please Sees Develop Commits (Priority: P1) 🎯 MVP

**Goal**: Change `target-branch: main` → `target-branch: develop` in `.github/workflows/cd.yml` so release-please analyzes develop's commits and creates release PRs against develop.

**Independent Test**: A develop push with conventional commits produces a release-please log showing `commits: N` (N>0) and `Building candidate release pull request` — not "No commits, skipping". A release PR against `develop` is created.

### Tests for User Story 1 (live CD validation) ⚠️

> **NOTE**: These are live-workflow verification steps (quickstart Scenario 1-2), run after implementation.

- [x] T004 [US1] Verify release-please analyzes develop commits: after a develop push with conventional commits, inspect the release-please job log and confirm `commits: N` (N>0) and `Building candidate release pull request`
- [x] T005 [US1] Verify a release PR is created against `develop` (base=develop, head=`release-please--branches--develop--components--rgt`)

### Implementation for User Story 1

- [x] T006 [US1] Change `target-branch: main` to `target-branch: develop` in the release-please step (`.github/workflows/cd.yml` line 99)

**Checkpoint**: release-please job config targets develop; workflow YAML parses.

---

## Phase 3: User Story 2 - Auto Develop-to-Main Promotion After Release (Priority: P1)

**Goal**: Add a `promote-to-main` job to `.github/workflows/cd.yml` that merges develop → main after release-please creates a release (fast-forward preferred, PR fallback for divergence).

**Independent Test**: After a release PR merges and release-please creates a release, the `promote-to-main` job runs and main reaches 0 commits behind develop automatically (quickstart Scenario 3). When no release is created, the job is skipped (quickstart Scenario 4).

### Tests for User Story 2 (live CD validation) ⚠️

> **NOTE**: Live-workflow verification (quickstart Scenario 3-4).

- [ ] T007 [US2] Verify auto-promotion: merge a release PR, wait for the CD run, confirm `git rev-list --count origin/main..origin/develop` == 0 and the `promote-to-main` job ran
- [ ] T008 [US2] Verify no premature promotion: push a conventional commit without merging a release PR; confirm the `promote-to-main` job is skipped and main is unchanged

### Implementation for User Story 2

- [x] T009 [US2] Add a `promote-to-main` job to `.github/workflows/cd.yml` with `needs: release-please` and `if: needs.release-please.outputs.release_created == 'true'`
- [x] T010 [US2] Implement the promotion logic in the job: checkout with `fetch-depth: 0`, check if `main` is an ancestor of `develop` (fast-forward possible via `git merge-base --is-ancestor origin/main origin/develop`), then either `git push origin develop:main` or open a PR via `gh pr create --base main --head develop`
- [x] T011 [US2] Confirm the `promote-to-main` job does NOT depend on `build-release`/`update-latest-tag` (only `needs: release-please`) so it isn't gated on macOS-gated asset builds

**Checkpoint**: Workflow YAML parses; `promote-to-main` job present with correct needs/if.

---

## Phase 4: User Story 3 - Clean State for Future Releases (Priority: P2)

**Goal**: Delete the stale `release-please--branches--develop--components--rgt` remote branch so release-please starts from a clean state.

**Independent Test**: `git ls-remote --heads origin | grep -i release-please` shows no stale branches after cleanup (quickstart Scenario 5).

### Tests for User Story 3 (verification)

- [x] T012 [US3] Verify the stale release-please branch is removed: `git ls-remote --heads origin | grep -i release-please` shows no `release-please--branches--develop--components--rgt` at SHA `c68f702`; only a branch re-created by the new flow (if any) may exist

### Implementation for User Story 3

- [x] T013 [US3] Delete the stale `release-please--branches--develop--components--rgt` remote branch: `git push origin --delete release-please--branches--develop--components--rgt` (confirm it's stale first — not the head of any open PR)

**Checkpoint**: Stale branch removed.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation of the end-to-end release cycle and consistency

- [x] T014 Validate the modified `.github/workflows/cd.yml` YAML parses: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/cd.yml'))"` (or Ruby fallback)
- [ ] T015 Verify CI is green on the fix PR (fmt, clippy, test on all 3 OSes, security, test presence)
- [ ] T016 Run quickstart.md validation scenarios end-to-end (see `specs/016-fix-cd-promotion/quickstart.md`) — a full release cycle completes with zero manual intervention
- [ ] T017 [P] Confirm `main` and `develop` are in sync (`git rev-list --count origin/main..origin/develop` == 0) after the validation cycle
- [ ] T018 Document the final flow state in `specs/016-fix-cd-promotion/research.md` (confirm SC-003: release-please log shows commits > 0)

**Checkpoint**: Full release cycle validated; main/develop in sync; CI green.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — establishes baseline
- **US1 (Phase 2)**: Depends on Setup
- **US2 (Phase 3)**: Depends on US1 (both edit `.github/workflows/cd.yml` — same file, must be sequential)
- **US3 (Phase 4)**: Depends on Setup. **Independent of US1/US2** (remote branch operation, different concern) — can run in parallel with them
- **Polish (Phase 5)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: Independent after Setup
- **US2 (P1)**: Depends on US1 (same file `cd.yml` — sequential edit)
- **US3 (P2)**: Independent — can run in parallel with US1/US2 (does not touch `cd.yml`)

### Within Each User Story

- Verification steps guide implementation
- Single-file edits applied carefully (same file `cd.yml` for US1 and US2)

### Parallel Opportunities

- US3 (T012, T013) can run in parallel with US1+US2 (different concern: remote branch vs `cd.yml`)
- T014 and T015 (YAML validation + CI check) can run in parallel
- T017 and T018 (sync check + doc note) can run in parallel

---

## Parallel Example: US3 alongside US1+US2

```bash
# Developer A: User Stories 1 + 2 (edits to .github/workflows/cd.yml)
Tasks: T006, T009, T010, T011 (sequential — same file)

# Developer B: User Story 3 (remote branch cleanup, no cd.yml edit)
Tasks: T013  # git push origin --delete release-please--branches--develop--components--rgt
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: US1 (T004-T006)
3. **STOP and VALIDATE**: `target-branch: develop` makes release-please see develop commits (quickstart Scenario 1)
4. Open PR — the corrected release-please analysis is the MVP

### Incremental Delivery

1. Setup → baseline confirmed
2. US1 → release-please analyzes develop (root cause fixed)
3. US2 → auto promotion develop → main (full automation)
4. US3 → clean branch state
5. Polish → full cycle validated

### Parallel Team Strategy

With two developers:

1. Team completes Setup together
2. Developer A: US1 + US2 (sequential edits to `cd.yml`)
3. Developer B: US3 (branch cleanup, independent)
4. Both merge into one PR, then Polish phase validates the full release cycle

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- No Rust test files — validation is via live CD runs (deterministic per the workflow behavior, per Constitution IX's determinism spirit)
- No force-push on main, no premature promotion (FR-004/FR-005)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
