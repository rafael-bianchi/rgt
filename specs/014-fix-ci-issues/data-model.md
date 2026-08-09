# Data Model: Fix Pre-existing CI Issues

**Feature**: 014-fix-ci-issues

## Overview

This feature has no persistent data model — it fixes a test harness and a CI workflow. This document formalizes the runtime state these components manage, so the contracts and quickstart can reference concrete invariants.

## Test Harness State (tests/integration/test_hooks_installer.rs)

### Process-global CWD Mutex

| Attribute | Value |
|---|---|
| Identifier | `static CWD_MUTEX: Mutex<()>` |
| Scope | Process-global (static) |
| Purpose | Serialize `std::env::set_current_dir` across parallel tests in the same process |
| Poison behavior | Recovered via `lock().unwrap_or_else(\|e\| e.into_inner())` — a panic in one test does not poison subsequent tests |
| Guard lifetime | Held for the full test body (returned by `set_cwd` and bound to `_guard`) |

### Home-Directory Override

| Attribute | Value |
|---|---|
| Environment variable (Unix) | `HOME` |
| Environment variable (Windows) | `USERPROFILE` |
| Value | `dir.path().to_str().unwrap()` — the temp dir for the current test |
| Consumer | `dirs::home_dir()` inside `detect_and_configure_hooks` |
| Invariant | After the override, `dirs::home_dir()` MUST resolve to the test's temp dir on every platform |

### Hook Config Outputs (written by installer, read by tests)

| Agent | File (relative to home/temp dir) | Asserted by test |
|---|---|---|
| Claude Code | `.claude/settings.json` | `test_claude_code_hook_config` |
| Cursor | `.cursor/hooks.json` | `test_cursor_hook_config` |
| Codex | `AGENTS.md` | `test_codex_hook_config` |
| Windsurf | `.windsurfrules` | `test_windsurf_hook_config` |

**Path invariant**: All file paths are built with `Path::join` (platform-native separators). Tests assert `path.exists()` and read via `fs::read_to_string` — no hard-coded `/` separators.

## CI Workflow Permission State (.github/workflows/pr-target-check.yml)

### GITHUB_TOKEN Scopes

| Scope | Required By | Purpose |
|---|---|---|
| `contents: read` | (implicit checkout ops) | Read repo contents |
| `issues: write` | `github.rest.issues.addLabels` | Add the `wrong-base` label |
| `pull-requests: write` | `github.rest.issues.createComment` | Post the guidance comment on the PR |

**Invariant**: The workflow MUST NOT declare scopes beyond these three (least privilege, FR-009).

### Workflow Decision Table

| PR base ref | PR head ref | Action |
|---|---|---|
| `main` | `≠ develop` | Add label + post comment (job runs) |
| `develop` | any | Skip cleanly (job skipped by `if`) |
| any other | any | Skip cleanly |

## State Transitions

### Mutex State

```
Poisoned (panic while held) ──into_inner()──> Recovered (usable by next test)
```

### PR Label State (per PR)

```
no label ──addLabels──> "wrong-base" present (re-add is idempotent no-op)
```
