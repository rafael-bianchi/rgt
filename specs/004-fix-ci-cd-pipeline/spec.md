# Feature Specification: Fix CI/CD Pipeline Post-Deployment Issues

**Feature Branch**: `004-fix-ci-cd-pipeline`

**Created**: 2026-07-31

**Status**: Draft

**Input**: User description: "Fix CI/CD pipeline issues discovered after initial deployment. (1) CI workflow must trigger on push to develop/main, not just pull_request. (2) CI security job hardcodes origin/main in git diff commands — must use actual base branch. (3) CD workflow's release-please fails because GitHub Actions is not permitted to create pull requests in this repository."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - CI Validates All Commits, Not Just Pull Requests (Priority: P1)

As a contributor who pushes directly to the develop or main branch, I want the CI pipeline to run on every push so that formatting, linting, tests, and security scans are validated whether the change arrives via pull request or direct commit.

**Why this priority**: Direct pushes to develop and main are valid workflows for maintainers. Missing CI validation on direct commits creates a blind spot where untested code can reach the release branch.

**Independent Test**: Push a commit directly to the develop branch. Verify that the CI pipeline triggers within seconds and all jobs (fmt, clippy, test, security) execute.

**Acceptance Scenarios**:

1. **Given** a commit pushed directly to the develop branch, **When** the push event is received, **Then** the CI pipeline triggers and the fmt, clippy, test, and security jobs all run.
2. **Given** a commit pushed directly to the main branch, **When** the push event is received, **Then** the CI pipeline triggers and all jobs run.

---

### User Story 2 - Security Scans Use Correct Base Branch (Priority: P1)

As a contributor submitting a pull request targeting the develop branch, I want the CI security scan to compare my changes against the correct base branch (develop), so that only my actual changes are flagged without false positives from divergence against main.

**Why this priority**: When all security scans hardcode `origin/main` as the comparison base, pull requests targeting develop show false positives for changes already present on develop but not yet on main. This misdirects reviewers and undermines trust in the security scan.

**Independent Test**: Create a PR targeting develop with changes only to a non-critical file. Verify the critical files check reports "No critical files modified" because the diff is against develop, not main.

**Acceptance Scenarios**:

1. **Given** a pull request targeting the develop branch, **When** the security job runs, **Then** the git diff commands compare against the develop branch, not hardcoded main.
2. **Given** a commit pushed directly to the develop branch, **When** the security job runs, **Then** the git diff commands fall back to comparing against the main branch as a reasonable default.
3. **Given** a pull request targeting the main branch, **When** the security job runs, **Then** the git diff commands compare against main, matching the actual target.

---

### User Story 3 - Release Automation Creates Pull Requests Successfully (Priority: P2)

As a project maintainer, I want the continuous delivery pipeline to automatically create release pull requests when conventional commits are merged to main, so that version bumps and changelogs are generated without manual intervention.

**Why this priority**: The release-please automation is the core of the CD pipeline. Without it, maintainers must manually tag releases and write changelogs, which is the workflow this pipeline was designed to replace.

**Independent Test**: Merge a `feat:` commit to main. Verify the CD pipeline creates a release pull request with the correct version bump and changelog entries.

**Acceptance Scenarios**:

1. **Given** the repository has "Allow GitHub Actions to create and approve pull requests" enabled in repository settings, **When** a conventional commit is pushed to main, **Then** the release-please job successfully creates a PR updating the version and changelog.
2. **Given** the repository settings have been configured correctly, **When** the CD workflow runs on any branch, **Then** the release-please job no longer fails with a permissions error.

---

### Edge Cases

- **Push to develop with no prior PR**: When a direct push to develop occurs without a preceding PR, the security job's git diff falls back to comparing against main, which is expected and acceptable.
- **Concurrent pushes to main**: If two commits land on main in quick succession, the release-please action handles the race condition by detecting the existing release PR and updating it rather than creating a duplicate.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CI pipeline MUST trigger on push events to the develop and main branches, in addition to pull request events.
- **FR-002**: The CI security job's critical files check MUST diff against the pull request's base branch when triggered by a pull request event, and fall back to main when triggered by a push event.
- **FR-003**: The CI security job's dangerous patterns scan MUST use the same dynamic base branch logic as the critical files check.
- **FR-004**: The CI security job's new dependency detection MUST use the same dynamic base branch logic for its Cargo.toml diff.
- **FR-005**: The repository MUST be configured to allow GitHub Actions to create and approve pull requests, enabling the release-please automation to function.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of pushes to develop and main trigger the CI pipeline within 30 seconds of the push event.
- **SC-002**: PRs targeting develop report security scan results that are scoped to the develop-to-feature diff, with zero false positives from changes already merged to develop but not yet landed on main.
- **SC-003**: The CD pipeline successfully creates a release pull request within 2 minutes of a conventional commit reaching main, with zero permission errors.

## Assumptions

- The repository uses the main and develop branching model established in the prior CI/CD hardening feature.
- The GitHub Actions default GITHUB_TOKEN provides sufficient scope when the repository setting is enabled; a GitHub App token is not required for this project's scale.
- All three fixes are configuration-level changes to workflow files and repository settings; no application code changes are needed.
