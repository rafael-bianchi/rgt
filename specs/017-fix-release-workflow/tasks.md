# Tasks: Fix Release Workflow — Tag-Based Builds & Dev Pre-release Gating

**Input**: Design documents from `specs/017-fix-release-workflow/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: No unit/contract/integration tests (GitHub Actions workflow changes). Validation is via live workflow runs per quickstart.md scenarios.

**Organization**: Tasks are grouped by user story. US1 and US2 touch different files (`release.yml` vs `cd.yml`) — parallel-safe.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Establish the current workflow state before changes

- [x] T001 Confirm the 3 binary-build checkouts in `.github/workflows/release.yml` currently have NO `ref` (lines 58, 130, 154 — `actions/checkout@v4` with no `with.ref`)
- [x] T002 Confirm the `pre-release` job `if` condition in `.github/workflows/cd.yml` currently includes the develop-push trigger (`github.ref == 'refs/heads/develop'`)
- [x] T003 Record the current release build state: `gh release view v0.3.2 --json assets --jq '.assets | length'` (expected 0 — assets were cancelled)

---

## Phase 2: User Story 1 - Release Binaries Always Match the Tag (Priority: P1) 🎯 MVP

**Goal**: Make `.github/workflows/release.yml` binary-build jobs checkout `ref: ${{ inputs.tag }}` so binaries are compiled from the exact released commit.

**Independent Test**: A release build for `v0.3.2` checks out commit `fbeb396` (the tag) even when develop is ahead, and the release ends with all platform assets attached (quickstart Scenario 1).

### Tests for User Story 1 (live workflow validation) ⚠️

> **NOTE**: Live-workflow verification (quickstart Scenario 1, 4, 5), run after implementation.

- [ ] T004 [US1] Verify tag-based checkout: trigger `gh workflow run release.yml --ref v0.3.2 -f tag=v0.3.2`, confirm the build job's checkout resolves to the tag commit `fbeb396` (not a newer develop commit)
- [ ] T005 [US1] Verify the release ends with all platform assets attached: `gh release view v0.3.2 --json assets --jq '.assets | length'` >= 6 archives + checksums

### Implementation for User Story 1

- [x] T006 [US1] Add `with: { ref: ${{ inputs.tag }}}` to the `build` matrix job's `actions/checkout@v4` step in `.github/workflows/release.yml` (line 58)
- [x] T007 [US1] Add `with: { ref: ${{ inputs.tag }}}` to the `build-deb` job's `actions/checkout@v4` step in `.github/workflows/release.yml` (line 130)
- [x] T008 [US1] Add `with: { ref: ${{ inputs.tag }}}` to the `build-rpm` job's `actions/checkout@v4` step in `.github/workflows/release.yml` (line 154)
- [x] T009 [US1] Confirm the `release` (line 179) and `homebrew` (line 220) jobs' checkouts are intentionally LEFT unchanged (they don't compile binaries — research decision)

**Checkpoint**: 3 binary-build checkouts pin the tag ref; YAML parses.

---

## Phase 3: User Story 2 - Dev Pre-Release Builds Run Only on Demand (Priority: P2)

**Goal**: Gate the `pre-release` job in `.github/workflows/cd.yml` to `workflow_dispatch` only, so dev pre-releases are not built on every develop push.

**Independent Test**: A push to develop skips the `Pre-release` job; a `workflow_dispatch` on develop runs it (quickstart Scenarios 2-3).

### Tests for User Story 2 (live workflow validation) ⚠️

> **NOTE**: Live-workflow verification (quickstart Scenario 2-3), run after implementation.

- [ ] T010 [US2] Verify push skips pre-release: push a commit to develop, inspect the CD run's `Pre-release` job — expected SKIPPED
- [ ] T011 [US2] Verify dispatch runs pre-release: `gh workflow run CD --ref develop`, inspect the CD run's `Pre-release` job — expected RUNS (produces dev pre-release)

### Implementation for User Story 2

- [x] T012 [US2] Change the `pre-release` job `if` condition in `.github/workflows/cd.yml` from `github.ref == 'refs/heads/develop' || (workflow_dispatch && github.ref != 'refs/heads/main')` to `workflow_dispatch && github.ref != 'refs/heads/main'`

**Checkpoint**: `pre-release` job gated to manual dispatch; YAML parses.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation of the release pipeline and consistency

- [x] T013 Validate both modified workflow YAMLs parse: `ruby -e "require 'yaml'; YAML.load_file('.github/workflows/release.yml'); YAML.load_file('.github/workflows/cd.yml'); puts 'YAML valid'"`
- [ ] T014 Verify CI is green on the fix PR (fmt, clippy, test on all 3 OSes, security, test presence)
- [ ] T015 Run quickstart.md validation scenarios end-to-end (see `specs/017-fix-release-workflow/quickstart.md`) — including the re-triggered v0.3.2 release build completing with assets (SC-006)
- [ ] T016 Confirm `rgt update` consumers unaffected: `rgt update --check` (or the update code path) still resolves the latest stable release with its platform asset (SC-004)
- [ ] T017 [P] Document the final workflow state in `specs/017-fix-release-workflow/research.md` (confirm SC-001: tag checkout verified; SC-002: push skips pre-release)

**Checkpoint**: Full release pipeline validated; v0.3.2 binaries available; CI green.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — establishes baseline
- **US1 (Phase 2)**: Depends on Setup
- **US2 (Phase 3)**: Depends on Setup. **Independent of US1** (different file: `cd.yml` vs `release.yml`) — can run in parallel
- **Polish (Phase 4)**: Depends on both user stories

### User Story Dependencies

- **US1 (P1)**: Independent after Setup
- **US2 (P2)**: Independent after Setup — no dependency on US1

### Within Each User Story

- Verification steps guide implementation
- T006 must precede T007/T008 only for consistency review (all 3 are in different jobs but same file — apply sequentially to avoid edit conflicts)

### Parallel Opportunities

- US1 (Phase 2) and US2 (Phase 3) can be developed in parallel (disjoint files)
- T007 and T008 edit the same file (`release.yml`) in distinct job blocks — apply sequentially to avoid edit conflicts; no longer marked [P]
- T016 and T017 (consumer check + doc note) can run in parallel

---

## Parallel Example: US1 + US2

```bash
# Developer A: User Story 1 (edits to .github/workflows/release.yml)
Tasks: T006, T007, T008, T009

# Developer B: User Story 2 (edits to .github/workflows/cd.yml)
Tasks: T012
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: US1 (T004-T009)
3. **STOP and VALIDATE**: re-trigger the v0.3.2 release build from the tag — binaries build from `fbeb396` and attach (fixes the immediate 0-assets gap)
4. Open PR — tag-correct binaries are the MVP

### Incremental Delivery

1. Setup → baseline confirmed
2. US1 → release binaries match their tags (correctness)
3. US2 → dev pre-releases only on demand (efficiency)
4. Polish → full pipeline validated, v0.3.2 binaries live

### Parallel Team Strategy

With two developers:

1. Team completes Setup together
2. Developer A: US1 — tag refs in `release.yml`
3. Developer B: US2 — pre-release gating in `cd.yml`
4. Both merge into one PR, then Polish validates the full release pipeline

---

## Notes

- [P] tasks = different files, no dependencies (or distinct blocks within one file)
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- No Rust test files — validation is via live workflow runs
- T013 YAML validation uses Ruby (Python yaml module may be absent, as seen in feature 016)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
