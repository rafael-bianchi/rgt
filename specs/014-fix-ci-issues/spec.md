# Feature Specification: Fix Pre-existing CI Issues

**Feature Branch**: `fix/014-ci-issues`

**Created**: 2026-08-09

**Status**: Draft

**Input**: User description: "Fix two pre-existing CI issues: (1) Windows CI test failures in `test_hooks_installer.rs` due to path-separator assumptions and a `Mutex` `PoisonError`; (2) the `pr-target-check.yml` workflow fails with 'Resource not accessible by integration' because it declares `permissions: {}` but attempts to add labels and comments via `GITHUB_TOKEN`."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Windows CI Passes Reliably (Priority: P1)

A developer opens or updates a pull request against `develop` or `main`. GitHub Actions runs the CI suite including the `test (windows-latest)` job. The hook installer integration tests must pass on Windows, not just on macOS/Linux, so that the Windows test matrix job is green and merges are not blocked.

**Why this priority**: Branch protection is now active on `main`, and CI status is the gate for merging. A red Windows job blocks the release pipeline. This is the blocking issue.

**Independent Test**: On a Windows runner (or with the equivalent test environment), run `cargo test --test test_hooks_installer` and confirm all tests pass. No path-separator panics, no `Mutex` `PoisonError`.

**Acceptance Scenarios**:

1. **Given** the CI suite runs on `windows-latest`, **When** `test_hooks_installer` executes, **Then** all 7 tests pass with no `Os { code: 3, kind: NotFound }` panics from `read_json`.
2. **Given** a test panics while holding the CWD mutex guard, **When** the next test in the same process attempts to acquire the same mutex, **Then** it does not fail with `PoisonError` — the lock is recovered or the guard is scoped so a panic in one test does not poison subsequent tests.
3. **Given** the tests run on Windows, **When** the test sets the home-directory override, **Then** the override is honored on all three platforms (macOS, Linux, Windows) so hook files are written to and read from the same temp directory.
4. **Given** the hook installer writes config files, **When** tests assert on their paths, **Then** path construction works identically across platforms (no hard-coded `/` separators that break on Windows).

---

### User Story 2 - PR Target Check Workflow Has Proper Permissions (Priority: P1)

A pull request targeting `main` (from a branch other than `develop`) triggers the `pr-target-check.yml` workflow via `pull_request_target`. The workflow must successfully add the `wrong-base` label and post a comment telling contributors to target `develop`. Currently it fails with `HttpError: Resource not accessible by integration` because the workflow declares `permissions: {}` while the `github-script` step needs write access to issues.

**Why this priority**: The workflow is meant to enforce the contribution convention (target `develop`, not `main`). Its failure means contributors get no automated guidance, and the label is not applied. This is a governance gate and should work whenever a PR mis-targets `main`.

**Independent Test**: Open a test PR against `main` (or use `workflow_dispatch`-compatible trigger), observe that the `PR Target Branch Check` workflow run succeeds, the `wrong-base` label is applied to the PR, and the explanatory comment appears. A correctly-targeted PR (`base=develop`) must still skip the workflow cleanly.

**Acceptance Scenarios**:

1. **Given** a PR with `base=main` and `head≠develop`, **When** the workflow runs, **Then** it completes successfully (no `Resource not accessible by integration` error) and the `wrong-base` label is added to the PR.
2. **Given** a PR with `base=main` and `head≠develop`, **When** the workflow runs, **Then** a comment is posted explaining that contributions should target `develop`, with a link to `CONTRIBUTING.md`.
3. **Given** a PR with `base=develop` (any head), **When** the workflow runs, **Then** the job is skipped cleanly (no label, no comment, no failure).
4. **Given** the workflow has minimal privilege, **When** it runs, **Then** it requests only the permissions it needs (issues + PRs write) and does not grant broader repository access than necessary.

---

### Edge Cases

- **Mutex poisoning from earlier test failure**: A failing test that panics while holding the CWD mutex must not cause a cascade of `PoisonError` failures in subsequent tests.
- **Windows home-directory env var**: On Windows, the home override must use the correct environment variable (`USERPROFILE`) that `dirs::home_dir()` reads, in addition to `HOME` used on Unix.
- **PR from fork**: A `pull_request_target` event from a fork still needs the label/comment step to work with the elevated token (write permissions must be declared at workflow or job level).
- **Repeated PR edits**: `pull_request_target` fires on `edited` too; the workflow must be idempotent (re-adding an existing label must not error).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CI test suite MUST pass on `windows-latest` for `tests/integration/test_hooks_installer.rs` with no path-related panics.
- **FR-002**: The CWD mutex guard in `tests/integration/test_hooks_installer.rs` MUST be resilient to poisoning — a panic in one test MUST NOT cause `PoisonError` failures in the remaining tests in the file.
- **FR-003**: The home-directory override used by hook installer tests MUST resolve the platform-correct home environment variable (`HOME` on Unix, `USERPROFILE` on Windows) so `dirs::home_dir()` resolves to the temp directory on all CI platforms.
- **FR-004**: Path assertions in hook installer tests MUST be platform-independent (no assumptions about `/` vs `\` separators).
- **FR-005**: The `pr-target-check.yml` workflow MUST declare the permissions needed for its `github-script` step (issues and pull-requests write) so label-add and comment-post operations succeed.
- **FR-006**: The `pr-target-check.yml` workflow MUST add the `wrong-base` label to a PR targeting `main` when the head branch is not `develop`.
- **FR-007**: The `pr-target-check.yml` workflow MUST post a comment explaining the correct contribution target (`develop`) with a link to `CONTRIBUTING.md` when a PR mis-targets `main`.
- **FR-008**: The `pr-target-check.yml` workflow MUST skip cleanly (no label, no comment, success status) for PRs targeting `develop`.
- **FR-009**: The workflow MUST NOT request broader permissions than required (least privilege).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of `test_hooks_installer` tests pass on the `windows-latest` CI job (previously 0/7 passed due to path panics).
- **SC-002**: A single test failure in the hooks installer test file does not cascade into additional failures — the CI job reports exactly the number of genuinely failing tests.
- **SC-003**: The `PR Target Branch Check` workflow succeeds for a mis-targeted PR (label applied + comment posted) with zero permission errors.
- **SC-004**: The `PR Target Branch Check` workflow skips cleanly for correctly-targeted PRs without touching the PR.
- **SC-005**: No new CI failures are introduced on macOS/Linux by the Windows/home-override changes (existing green jobs remain green).
- **SC-006**: All changes pass `cargo fmt`, `cargo clippy --all-targets`, and the full `cargo test` suite on the local (macOS) environment before merge.

## Assumptions

- The `dirs` crate's `home_dir()` is the source of truth for the home directory on all platforms; the fix is to make the tests set the platform-correct environment variable, not to change `home_dir()` behavior in production code.
- The hook installer production code (`src/hooks/installer.rs`) is correct as-is; only the test harness needs platform-awareness. If a production fix is also required for true cross-platform path correctness, it will be evaluated separately.
- `pr-target-check.yml` is the only workflow with the permission-scoping bug; other workflows (`CI`, `CD`, `Release`, `Copilot`) are out of scope for this feature unless the same bug pattern is found in them.
- The `pull_request_target` trigger is intentionally used (it runs with elevated permissions from the base repo); the fix is to declare the correct `permissions:` block, not to change the trigger type.
- Branch protection on `main` remains as configured during this feature; CI green is the merge gate and must be satisfied for the fix PR itself.
