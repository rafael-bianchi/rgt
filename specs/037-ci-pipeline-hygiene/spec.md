# Feature Specification: CI Pipeline Hygiene

**Feature Branch**: `feat/037-ci-pipeline-hygiene`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Three independent CI weaknesses bundled as one workflow-hardening pass: (a) `ci.yml`'s 'Test Presence Check' job is a literal no-op that always passes; (b) no code-coverage tooling/badge exists; (c) third-party GitHub Actions are pinned by tag, not commit SHA, in jobs holding `contents: write`/`id-token: write`/`secrets: inherit`."

Source context: `docs/weakpoints-and-growth-opportunities.md` §2 items 4, 5, 6. Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #4, P2.

## Clarifications

### Session 2026-08-31

- Q: The "Test Presence Check" job (`ci.yml`) is a no-op that always passes. Implement it for real, or delete it? → A: Implement it for real. The job name promises a check contributors rely on, and this repo's own history shows the value (e.g. spec 030's test-fixture fix and the verification pass's recurring "missing regression test" findings). A real check flags new/modified `src/*.rs` files that have no corresponding test file, surfacing the exact class of gap this project has repeatedly caught late.
- Q: Code-coverage tooling (`cargo llvm-cov` + badge). Given the domain is numeric correctness, worth adding? → A: Yes — add a coverage job that runs `cargo llvm-cov` and reports a line-coverage percentage, plus a README badge pointing at a coverage artifact. But the badge source matters: use `codecov`/`coveralls`-style upload only if credentials are available; otherwise emit the coverage number in a PR comment / step summary (no external service dependency). Decide at implementation: prefer no-new-secret (step summary + a `coverage` artifact) unless the maintainer later wires an upload token.
- Q: Action pinning to commit SHA. Which jobs? → A: Pin third-party actions to commit SHA **in the jobs holding elevated permissions** (`release.yml` has `contents: write`/`id-token: write`/`attestations: write`; `cd.yml` has `contents: write`/`pull-requests: write`). `ci.yml` and `pr-target-check.yml` hold only read/scoped perms; the report's own note says `pr-target-check`'s `pull_request_target` is safe. To keep it simple and consistent, pin third-party actions everywhere, but the mandatory set is the elevated-permission workflows.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Test-Presence Gate Actually Catches Missing Tests (Priority: P2)

A contributor opens a PR that adds `src/cli/foo.rs` with no corresponding test file (or modifies an existing source file without touching its tests). Today the "test presence" CI job prints "Skipped - add custom check logic as needed." and always passes — false confidence. After this fix, the job examines the changed Rust files and fails (or warns loudly) when a new/modified `src/` file has no test coverage, exactly as the job's name and summary promise.

**Why this priority**: P2 per the plan — "false confidence is worse than an absent gate" (growth report §2.4).

**Independent Test**: Run the check logic locally against a synthetic diff (a new `src/*.rs` file with no test) and confirm it flags it; confirm it passes for a normal diff with tests.

**Acceptance Scenarios**:

1. **Given** a PR diff adding `src/cli/foo.rs` with no `tests/**` file referencing it, **When** the check-test-presence job runs, **Then** it reports the missing coverage and fails (non-zero).
2. **Given** a PR diff modifying `src/verify/mod.rs` that also updates `tests/`, **When** the job runs, **Then** it passes.
3. **Given** a PR with only `docs/`/`specs/`/workflow changes, **When** the job runs, **Then** it passes (no false positive).

---

### User Story 2 - Coverage Is Measured and Visible (Priority: P2)

A maintainer wants to see whether the correctness-critical code paths are actually exercised. Today there is no coverage tooling or badge anywhere. After this fix, a CI job runs `cargo llvm-cov` over the test suite and reports the line-coverage percentage in the PR summary (and stores a coverage artifact), and the README carries a coverage badge that reflects it (or an explicit "coverage reported in CI" note if no external upload service is wired).

**Why this priority**: P2 per the plan — "matters more than average given the domain is numeric correctness" (growth report §2.5).

**Independent Test**: The coverage job runs locally-equivalent `cargo llvm-cov` and produces a percentage; the README badge link is valid or the note is accurate.

**Acceptance Scenarios**:

1. **Given** the CI workflow, **When** the coverage job runs, **Then** it executes `cargo llvm-cov` and writes the coverage percentage to the job summary.
2. **Given** the README, **When** it is read, **Then** it displays a coverage badge (or an explicit "coverage is reported in CI, no external service" note) rather than nothing.
3. **Given** the coverage run, **When** it completes, **Then** no new secret/token is required for the default path (step summary + artifact).

---

### User Story 3 - Third-Party Actions Are Pinned to Commit SHA (Priority: P2)

A security review of `release.yml` (which holds `contents: write`, `id-token: write`, `attestations: write`) finds that every third-party action is referenced by a mutable tag (`@v4`, `@v2`, `@stable`). A compromised or hijacked tag could inject malicious steps into a release that can sign attestations and write releases. After this fix, each third-party action is pinned to its exact commit SHA (with the tag as a trailing comment for readability).

**Why this priority**: P2 per the plan — "standard supply-chain hardening, not an active exploit" (growth report §2.6).

**Independent Test**: `grep -E "uses: [a-z-]+/[-a-z]+@[0-9a-f]{40}"` matches every third-party `uses:` line in `release.yml` and `cd.yml` (and ideally all workflows).

**Acceptance Scenarios**:

1. **Given** `release.yml`, **When** every third-party `uses:` line is inspected, **Then** each references a 40-char commit SHA (with the version tag preserved in a comment).
2. **Given** `cd.yml`, **When** its third-party `uses:` lines are inspected, **Then** the same SHA pinning holds.
3. **Given** any action in the repo, **When** its SHA is verified, **Then** it matches the released tag it claims to be (no fake/typosquatted SHA).

---

### Edge Cases

- **Coverage without external service**: if no upload token exists, the coverage job must not fail the pipeline (report in summary + artifact; badge shows a "coverage in CI" note). Wiring an external badge is a follow-up the maintainer can do with a token.
- **Test-presence false positives**: files that legitimately have no direct test (e.g. `main.rs`, `lib.rs`, a new module re-exported and tested elsewhere) — the check should flag `src/*.rs` (excluding `main.rs`/`lib.rs`) with no test reference, and the failure message should be actionable (name the file and suggest a test path), not a bare failure.
- **Action SHA drift**: a SHA pin that doesn't match the tag's commit would break the workflow — the implementation must look up each tag's real commit SHA, not invent one.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The `check-test-presence` job in `ci.yml` MUST be implemented for real: it examines changed Rust source files (vs. the PR base) and fails when a new/modified `src/*.rs` file (excluding `main.rs`/`lib.rs`) has no corresponding test file under `tests/`; docs/workflow-only diffs pass; the failure message names the file and suggests a test path.
- **FR-002**: A new CI job MUST run code coverage (`cargo llvm-cov`, or `tarpaulin` if llvm-cov is unavailable on the runner) over the test suite and report the coverage percentage in the job summary, without requiring a new secret for the default path.
- **FR-003**: The README MUST carry a coverage badge (or an explicit "coverage reported in CI" note) that reflects the coverage job, and a link the maintainer can later upgrade to an external upload service.
- **FR-004**: Every third-party action `uses:` line in `release.yml` and `cd.yml` MUST be pinned to a 40-char commit SHA with the original version tag preserved in a trailing comment.
- **FR-005**: The SHA pins MUST correspond to the real commits of the tags they replace (verified during implementation, not invented); `ci.yml` and `pr-target-check.yml` SHOULD also be pinned for consistency.

### Key Entities *(include if feature involves data)*

- **TestPresenceDiff**: the set of changed Rust source files in a PR and the test-file coverage mapping used by the check.
- **CoverageArtifact**: the `cargo llvm-cov` HTML/`lcov` output stored as a CI artifact and summarized.
- **ActionPin**: a `uses: owner/repo@<sha> # tag` reference for each third-party action.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The test-presence check fails on a synthetic missing-test diff and passes on a normal diff (verified by running the check logic locally against constructed diffs).
- **SC-002**: `grep -E "uses: [a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+@[0-9a-f]{40}" .github/workflows/release.yml .github/workflows/cd.yml` matches every third-party action line (no `@v\d` tags remain in those workflows).
- **SC-003**: A coverage percentage is produced by `cargo llvm-cov` in CI and appears in the job summary; the README shows a coverage badge/note.
- **SC-004**: `cargo test` stays green (no source-code changes; workflow/test-tooling only).

## Assumptions

- `cargo llvm-cov` is installable on the ubuntu runner (`cargo install cargo-llvm-cov` or the `dtolnay`/`actions-rust-lang`-provided setup); `tarpaulin` is the fallback.
- Resolving each action tag → commit SHA is done via the GitHub API/releases during implementation (no credentials needed for public repos).
- The `ci.yml` job already has the `github.base_ref` gate; the check reads the PR diff via `actions/checkout` with `fetch-depth` and `origin/main...HEAD` (matching the existing security job's diff pattern).
- No application code changes; this is workflows + README + optional `scripts/`.

## Out of scope

- Wiring an external coverage-upload token (e.g. Codecov) — a maintainer follow-up requiring a secret.
- Changing which jobs hold elevated permissions (a workflow restructuring, not hygiene).
- The `pull_request_target` design of `pr-target-check.yml` (already assessed safe by the growth report).
