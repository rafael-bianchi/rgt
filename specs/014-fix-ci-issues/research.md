# Research: Fix Pre-existing CI Issues

**Feature**: 014-fix-ci-issues
**Date**: 2026-08-09

## Decision: Platform-aware home-directory override in tests

**Decision**: Replace the single `std::env::set_var("HOME", ...)` call in `tests/integration/test_hooks_installer.rs` with a helper that sets the platform-correct variable:
- Unix (macOS/Linux): `HOME`
- Windows: `USERPROFILE`

**Rationale**: The `dirs` crate's `home_dir()` implementation reads `HOME` on Unix but `USERPROFILE` on Windows (falling back to `HOMEDRIVE`/`HOMEPATH`). The tests currently only set `HOME`, so on Windows the installer writes hook configs to the real `%USERPROFILE%` directory while the test reads from its temp dir — producing `Os { code: 3, kind: NotFound }` in `read_json`. Setting `USERPROFILE` on Windows redirects `home_dir()` to the temp dir, matching the test's expectations.

**Implementation shape**: A `set_home(dir)` helper (or a shared test utility) invoked by every test, e.g.:

```rust
fn set_home(dir: &std::path::Path) {
    #[cfg(target_os = "windows")]
    std::env::set_var("USERPROFILE", dir);
    #[cfg(not(target_os = "windows"))]
    std::env::set_var("HOME", dir);
}
```

**Alternatives considered**:
- Change production `home_dir()` behavior: Rejected — `dirs::home_dir()` is the documented source of truth; overriding it would be platform-incorrect.
- Use `dirs::home_dir()`'s internal logic directly: Rejected — relies on undocumented internals.
- Refactor `detect_and_configure_hooks` to accept an explicit home path: Rejected — changes the public API for a test-only concern; spec Assumption 2 keeps production code untouched.

## Decision: Poison-tolerant CWD mutex guard

**Decision**: Change the CWD mutex acquisition from `CWD_MUTEX.lock().unwrap()` to a poison-recovering pattern:

```rust
fn set_cwd(dir: &std::path::Path) -> std::sync::MutexGuard<'static, ()> {
    let guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_current_dir(dir).unwrap();
    guard
}
```

**Rationale**: `Mutex::lock()` returns `Err(PoisonError)` if a previous holder panicked while holding the guard. In the current suite, test 1 panicked (due to the Windows path bug) while holding the guard, poisoning the mutex — every subsequent test then failed with `PoisonError` at `set_cwd`. `into_inner()` recovers the lock and lets the remaining tests run; the actual root cause is still reported once for the genuinely-failing test. This prevents cascading failures (spec FR-002 / SC-002) and ensures CI reports exactly the number of genuinely failing tests.

**Alternatives considered**:
- Leave as-is (unwrap): Rejected — produces cascading `PoisonError` failures that obscure the real count and can hide regressions.
- Scope the guard to only the `set_current_dir` call (drop guard immediately): Rejected — the guard must be held for the whole test to serialize CWD changes across parallel tests; this is the existing design and dropping early reintroduces races.
- Use `parking_lot::Mutex` (no poisoning): Rejected — adds a new dependency for a two-line test fix; violates no-new-deps goal.

## Decision: `pr-target-check.yml` — declare workflow-level permissions

**Decision**: Change `permissions: {}` to:

```yaml
permissions:
  contents: read
  issues: write
  pull-requests: write
```

**Rationale**: The `github-script` step calls `github.rest.issues.addLabels` (needs `issues: write`) and `github.rest.issues.createComment` (needs `issues: write`; posting comments on PRs also falls under `pull-requests: write` when the target is a PR). With `permissions: {}` the `GITHUB_TOKEN` is scoped to nothing, hence `HttpError: Resource not accessible by integration`. Declaring these at workflow level applies to all jobs; there is only one job, so job-level would be equivalent. `contents: read` is required for `actions/checkout`-style operations and is the least-privilege default that keeps the token usable.

**Alternatives considered**:
- Job-level `permissions:`: Equivalent for a single-job workflow; workflow-level is simpler and less error-prone if jobs are added later.
- `pull-requests: write` only: Insufficient — `addLabels` targets issues API (`/issues/{issue_number}/labels`) and fails without `issues: write` even on PRs.
- Keep `permissions: {}` and use a PAT secret: Rejected — overkill; the repo's `GITHUB_TOKEN` with declared scopes is sufficient and avoids a standing credential (aligns with the earlier PAT discussion).

## Decision: Idempotency for repeated `edited` events

**Decision**: Guard the label-add with an existence check so `pull_request_target` firing on `edited` events re-runs without erroring when the label already exists.

**Rationale**: `pull_request_target` triggers on `opened` and `edited`. Re-adding an already-present `wrong-base` label via the REST API is safe (GitHub treats it as a no-op success), but an explicit check makes the intent clear and is cheap. The comment step is naturally idempotent (duplicate comments are acceptable).

**Alternatives considered**:
- No guard: The REST API already no-ops on existing labels; a guard is defensive. Kept minimal — no comment dedup logic.

## Decision: FR-006 and FR-007 implemented as one step

**Decision**: The label-add (FR-006) and comment-post (FR-007) are two distinct requirements but are implemented in a single `github-script` step in the workflow.

**Rationale**: Both operations share the same permission scope (`issues: write`) and the same conditional trigger, so splitting them into separate steps would add noise without isolation value.

**Alternatives considered**:
- Separate steps: Rejected — no isolation benefit since both run under identical conditions and permissions; adds YAML verbosity for no testability gain.

## Decision: SC-002 poison-tolerance verification approach

**Decision**: SC-002 (a single test failure does not cascade into additional failures) is verified via the manual quickstart Scenario 1 check rather than a permanent automated test.

**Rationale**: A permanent automated poison-cascade test would require injecting a deliberately-failing assertion into the suite. That failing assertion is indistinguishable from a real regression in CI — it would permanently mark the CI job red, making it unactionable and requiring the test to be constantly skipped or suppressed, which defeats its purpose. The poison-tolerant `into_inner()` recovery is exercised implicitly by every test run; the manual quickstart check verifies the cascade behavior on demand.

**Alternatives considered**:
- Permanent failing-test fixture with `#[ignore]`: Rejected — an ignored test is never run in CI, so it provides no regression protection while adding confusion.
- Subprocess test that runs the test binary with a known-failing filter and asserts the sibling tests still report their true status: Rejected — requires spawning `cargo test` from within a test, creating a heavy, slow, and platform-fragile test (violates Constitution IX determinism guidance).
