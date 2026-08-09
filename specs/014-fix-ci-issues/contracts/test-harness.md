# Contract: Cross-Platform Hook Installer Tests

**Feature**: 014-fix-ci-issues
**Contract type**: Test-harness behavior contract

## Synopsis

`tests/integration/test_hooks_installer.rs` must run identically on macOS, Linux, and Windows. The harness must isolate each test to its own temp directory on all platforms and must not cascade failures.

## Home-Directory Override Contract

**Precondition**: The current working directory is the test's temp dir.

**Action**: Set the platform-correct home environment variable to the temp dir.

| Platform | Variable | Value |
|---|---|---|
| `cfg(unix)` (macOS/Linux) | `HOME` | `dir.path()` |
| `cfg(windows)` | `USERPROFILE` | `dir.path()` |

**Postcondition**: `dirs::home_dir()` returns a path inside the test's temp dir on all platforms.

**Verification**: A test that calls `detect_and_configure_hooks(true, true, Some("claude-code"))` then reads `.claude/settings.json` from the temp dir must succeed (no `Os { code: 3, NotFound }`).

## Mutex Guard Contract

**Precondition**: Tests in this file run in parallel within one process.

**Action**: Each test acquires the global CWD mutex before changing the current directory and holds it for the test duration.

**Behavior on poison**: If a prior test panicked while holding the guard, the next acquisition MUST recover via `into_inner()` rather than fail with `PoisonError`.

**Postcondition**: A panic in one test does not cause additional failures in other tests.

**Verification**: Force one test to fail (e.g., assert on a nonexistent file); the remaining tests in the file still run and report their true pass/fail status.

## Path Construction Contract

- All paths constructed with `Path::join` (native separators).
- No literal `/` separators in test code.
- Assertions use `path.exists()` / `fs::read_to_string(path)` — never string-concatenated paths.

## Acceptance Mapping

| Acceptance Scenario | Contract Clause |
|---|---|
| US1/AC1 (7 tests pass on Windows) | Home override + path construction |
| US1/AC2 (no PoisonError cascade) | Mutex guard contract |
| US1/AC3 (override honored on all 3 OSes) | Home override |
| US1/AC4 (paths platform-independent) | Path construction |
