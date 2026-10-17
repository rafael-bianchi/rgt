# Tasks: CI Pipeline Hygiene

**Input**: Design documents from `/specs/037-ci-pipeline-hygiene/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ci-hygiene-contract.md, quickstart.md

**Tests**: Included — the check script is verified against synthetic diffs (Constitution IX).

**Organization**: Three independent user stories (test-presence, coverage, action pinning).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1/US2/US3)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Baseline and tooling.

- [ ] T001 Confirm baseline: `cargo test` green; `ci.yml`'s check-test-presence job is the documented no-op; inventory every third-party `uses:` line in `.github/workflows/*.yml` with its tag; confirm `cargo-llvm-cov` availability or note the install path

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The check script + SHA resolution (used by all three stories).

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T002 Create `scripts/check-test-presence.sh`: takes a diff file (or computes it from `origin/main...HEAD` with a `HEAD~1` fallback), extracts changed `src/*.rs` excluding `main.rs`/`lib.rs`, greps the `tests/` tree for each file's module reference, and fails (exit 1) with an actionable message (file + suggested test path) on a hard miss; passes for docs-only diffs
- [ ] T003 Resolve each third-party action tag in `release.yml`/`cd.yml` to its exact commit SHA via the GitHub API (verify the SHA matches the tag; no invented SHAs)

**Checkpoint**: The script runs locally against synthetic diffs; all SHA pins are recorded and verified.

---

## Phase 3: User Story 1 - The Test-Presence Gate Actually Catches Missing Tests (Priority: P2)

**Goal**: `ci.yml`'s check-test-presence job runs the real check.

**Independent Test**: `scripts/check-test-presence.sh` fails on a synthetic missing-test diff and passes on a normal/docs-only diff.

- [ ] T004 [US1] Rewrite the `check-test-presence` job in `.github/workflows/ci.yml` to invoke `scripts/check-test-presence.sh` (keep the `base_ref == 'develop'` gate and the checkout `fetch-depth`), feeding it the computed diff

**Checkpoint**: The job's YAML is syntactically valid; the script behavior is verified locally (SC-001).

---

## Phase 4: User Story 2 - Coverage Is Measured and Visible (Priority: P2)

**Goal**: A coverage job reports the percentage; the README shows it.

**Independent Test**: `cargo llvm-cov --all` produces a percentage locally.

- [ ] T005 [US2] Add a `coverage` job to `.github/workflows/ci.yml` (ubuntu, after `test`): install llvm-tools/`cargo-llvm-cov`, run `cargo llvm-cov --all`, extract the line-coverage percentage into `$GITHUB_STEP_SUMMARY`, upload the report as an artifact (no external upload, no secret)
- [ ] T006 [US2] Add a coverage badge/note to `README.md`'s badge row: an explicit "Coverage: reported per-PR in CI" note (or a badge if a hosted URL is available) with a link to the coverage job — no broken badge to a non-existent upload

**Checkpoint**: The coverage job YAML is valid; `cargo llvm-cov` runs locally; README shows the note (SC-003).

---

## Phase 5: User Story 3 - Third-Party Actions Are Pinned to Commit SHA (Priority: P2)

**Goal**: Elevated-permission workflows reference immutable SHAs.

**Independent Test**: `grep` over `release.yml`/`cd.yml` finds only SHA-pinned third-party `uses:` lines (SC-002).

- [ ] T007 [US3] Rewrite every third-party `uses:` line in `.github/workflows/release.yml` to `uses: owner/repo@<sha> # <tag>` using the T003 resolutions
- [ ] T008 [US3] Rewrite every third-party `uses:` line in `.github/workflows/cd.yml` the same way; for consistency also pin `ci.yml` and `pr-target-check.yml`

**Checkpoint**: `grep -E "uses: [a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[0-9a-f]{40}"` matches all third-party uses lines in the pinned workflows (SC-002).

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T009 Run the full contract: synthetic-diff check (fail + pass), grep for SHA pins, `cargo llvm-cov --all` percentage, and full `cargo test` green (no source changes)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS all three stories.
- **US1 (Phase 3)**: Depends on T002.
- **US2 (Phase 4)**: Depends on T001; independent of US1/US3.
- **US3 (Phase 5)**: Depends on T003.
- **Polish (Phase 6)**: Depends on all.

### User Story Dependencies

- US1/US2/US3 are independent after Foundational; can run in parallel.

### Within Each User Story

- Script/tooling first, then workflow wiring.

### Parallel Opportunities

- T004 (ci.yml), T005/T006 (coverage + README), T007/T008 (pinning) touch different files.

---

## Implementation Strategy

### Incremental Delivery

1. Foundational → check script + SHA resolution
2. US1 → real test-presence gate
3. US2 → coverage job + README note
4. US3 → SHA pinning
5. Polish → full contract verification

---

## Notes

- No application code changes; `cargo test` stays green.
- Coverage needs no new secret (summary + artifact); external badge upload is a maintainer follow-up.
- SHA pins must be the real commits of the tags they replace (verified, not invented).
- The test-presence check fails only on a hard miss (no `tests/` reference anywhere) to avoid false positives.
