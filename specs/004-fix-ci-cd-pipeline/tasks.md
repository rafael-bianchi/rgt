# Tasks: Fix CI/CD Pipeline Post-Deployment Issues

**Input**: Design documents from `/specs/004-fix-ci-cd-pipeline/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `quickstart.md`

**Organization**: Tasks are grouped by user story. All three are independent and can be implemented in parallel.

## Task Format: `[ID] [P?] [Story?] Description with file path`

- **[P]**: Parallelizable (different files, no dependencies)
- **[Story]**: User story label (e.g. `[US1]`, `[US2]`, `[US3]`)

---

## Phase 1: User Story 1 - CI Triggers on Push (Priority: P1)

**Goal**: CI workflow runs on push events to develop and main, not just pull requests.

**Independent Test**: Push a commit to develop. Verify CI triggers and all jobs execute.

- [X] T001 [US1] Add `push: branches: [develop, main]` trigger alongside existing `pull_request` trigger in `.github/workflows/ci.yml`

---

## Phase 2: User Story 2 - Security Scans Use Correct Base Branch (Priority: P1)

**Goal**: Critical files check, dangerous patterns scan, and new dependency detection all diff against the PR's actual base branch instead of hardcoded `origin/main`.

**Independent Test**: Open a PR targeting develop with changes only to a non-critical file. Verify security job reports no critical files modified.

- [X] T002 [US2] Replace hardcoded `origin/main` with `origin/${{ github.base_ref || 'main' }}` in critical files check (git diff --name-only) in `.github/workflows/ci.yml`
- [X] T003 [US2] Apply the same `origin/${{ github.base_ref || 'main' }}` replacement to dangerous patterns scan and new dependency detection (git diff commands) in `.github/workflows/ci.yml`

---

## Phase 3: User Story 3 - Release Automation Creates Pull Requests (Priority: P2)

**Goal**: The release-please action in CD can create pull requests after enabling the repository permission.

**Independent Test**: After the setting is enabled, push a `feat:` commit to main. Verify CD creates a release PR.

- [ ] T004 [US3] Enable "Allow GitHub Actions to create and approve pull requests" in repository Settings > Actions > General (manual repository configuration, no file change)

---

## Phase 4: Polish & Validation

**Purpose**: Validate all three fixes work end-to-end.

- [ ] T005 [P] Push a test commit to develop and verify CI triggers within 30s, security job uses develop as base, and CD creates release PR on push to main

---

## Implementation Strategy

### All Three Fixes in Parallel

1. T002 + T003 (ci.yml edits — same file, sequential)
2. T001 (ci.yml edit — same file, can combine with T002/T003)
3. T004 (repository settings — independent, manual action)

All four tasks modify different lines in one file, plus one repository setting. Execute as a single batch.
