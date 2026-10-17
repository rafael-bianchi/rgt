# Feature Specification: Hook Editor Path Quoting Fix

**Feature Branch**: `feat/031-hook-editor-path-quoting-fix`

**Created**: 2026-08-31

**Status**: Superseded by `049-init-doctor-and-config-editor-hardening`

**Input**: User description: "Fix the mismatch between `shell_quote` (which wraps the resolved `rgt` path in literal double quotes for hook `command` strings) and `is_rgt_command` (which tokenizes hook `command` strings via plain `split_whitespace()`, not quote-aware). When the resolved `rgt` binary path contains a space, a subsequent `rgt init` fails to recognize its own previously-written hook entry and could append a duplicate."

Source context: `docs/verification-of-fixes-develop.md` §2 "New issue found, not in the original report." Plan: `docs/spec-plan-followups-from-verification.md`, spec #2, P1.

## Clarifications

### Session 2026-08-31

- Q: Fix by making `is_rgt_command`'s tokenizer quote-aware (option a), or by avoiding the ambiguity entirely — matching on the unquoted resolved path directly rather than re-parsing the shell-quoted command string (option b)? → A: Make the tokenizer quote-aware (option a). It is the smaller, more robust fix that keeps `is_rgt_command` self-contained and future-proof against any future path (not just the one RGT itself wrote today); option (b) would couple the editor to installer state and require threading the resolved path through the merge path. The unit-level fix is validated with a deliberately spaced install path per the plan's recommendation.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - RGT Recognizes Its Own Hooks When Installed Under a Spaced Path (Priority: P1)

A developer installs `rgt` under a path containing a space (e.g. `/Applications/My App/rgt` on macOS, or a Windows `Program Files` layout) and runs `rgt init`. The installer writes hook `command` strings with the path shell-quoted — `"/Applications/My App/rgt" hook post` — so the hooks execute correctly. The developer then runs `rgt init` again (or adds another agent). Today, `is_rgt_command` tokenizes the command with `split_whitespace()`, which splits inside the quoted path, so it fails to recognize the entry as RGT-owned and may append a duplicate hook entry instead of updating in place — silently.

**Why this priority**: P1 — real latent bug in the just-shipped absolute-path hook-install fix (spec 028). No known trigger on this machine (install path has no spaces), but it is a plausible real-world case, and the failure mode (silent duplicate entries on re-init) is exactly the class of bug specs 019/028 were built to eliminate.

**Independent Test**: A unit test invoking the `is_rgt_command` predicate against a spaced-path command string (e.g. `"/Applications/My App/rgt" hook post`) returns true, and a full re-init against a settings.json already containing such an entry updates in place (no duplicate).

**Acceptance Scenarios**:

1. **Given** a hook `command` string `"/Applications/My App/rgt" hook post`, **When** the RGT-ownership predicate checks it, **Then** it is recognized as an RGT-managed entry (returns true).
2. **Given** a hook `command` string with a spaced path in the `hooks[]` nested form, **When** the predicate checks it, **Then** it is also recognized.
3. **Given** a bare non-RGT command (`rtk hook post`), **When** the predicate checks it, **Then** it is NOT misidentified as RGT-owned (unchanged behavior).
4. **Given** an existing settings.json with a spaced-path RGT hook entry, **When** `rgt init` runs again, **Then** no duplicate entry is created and the entry is updated/kept in place.

---

### Edge Cases

- A command with a quoted path AND a trailing `--agent` argument: `"/Applications/My App/rgt" hook post --agent claude-code` must still be recognized (the predicate only inspects the executable + `hook pre|post` tokens).
- A path containing an escaped quote (extremely rare): must not panic; recognition may conservatively fall back to false.
- A bare unquoted path with no spaces (`/usr/local/bin/rgt hook post`): must continue to be recognized (current behavior preserved).
- A Windows-style path `C:\Program Files\rgt.exe` (backslashes, drive letter): must be recognized for both the bare-name and absolute-path cases.
- The `--agent` variant used by the gemini/vibe/copilot TOML and JSON paths (`"..." hook pre --agent X`): recognition must not depend on the argument that follows `pre|post`.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `is_rgt_command` MUST correctly recognize RGT-owned hook commands whose executable path is shell-quoted and contains spaces, in both the top-level `command` and nested `hooks[].command` forms.
- **FR-002**: `is_rgt_command` MUST NOT regress recognition of existing forms: bare `rgt`/`rgt.exe`, unquoted absolute paths, Windows drive-letter paths, and commands with a trailing `--agent` argument.
- **FR-003**: `is_rgt_command` MUST NOT falsely classify a bare non-RGT executable (e.g. `rtk hook post`) as RGT-owned. (Note: per the existing design, any *absolute path* pointing at `hook pre|post` is treated as RGT-owned — RGT always writes absolute paths since spec 028; that classification is intentionally unchanged by this spec, which only fixes the quoted-spaced-path recognition.)
- **FR-004**: The fix MUST be confined to the command-recognition logic (`src/hooks/editor.rs`); no change to how `shell_quote` writes commands (`src/hooks/installer.rs`), since that behavior is correct and covered by existing tests.
- **FR-005**: The behavior MUST be covered by tests, including a deliberately spaced install path (per the plan's recommendation that this case not be closed from code reasoning alone).

### Key Entities *(include if feature involves data)*

- **HookCommand**: the `command` string in a hook config entry — `<exe> hook pre|post [--agent <name>]`, where `<exe>` may be shell-quoted.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `is_rgt_command` returns true for a spaced-path quoted command and false for a bare non-RGT command (unit-tested; both the existing and new forms).
- **SC-002**: A re-init (`rgt init` twice) against a spaced-path install produces exactly one RGT hook entry, no duplicate (contract-tested via the installer's idempotency/merge path).
- **SC-003**: All existing hook-editor and hook-installer tests remain green (no regression in the merge/update behavior specs 019/028 established).

## Assumptions

- The shell-quoting written by `shell_quote` (literal surrounding `"`) is correct and unchanged; only the reader (`is_rgt_command`) is made quote-aware.
- A minimal, dependency-free quote-aware tokenizer in `src/hooks/editor.rs` is preferable to pulling in a shell-parsing crate (Constitution XI).
- The bug has no known production trigger on the current development machine; the fix is preventive and test-verified, not empirically reproduced.

## Out of scope

- Changing the quoting format `shell_quote` produces (covered and correct per existing tests).
- The related `--stale-only` dead-branch in `src/cli/status.rs` and the `benches/bench_detection.rs` rehash-path coverage gap (separate findings from the same verification report, tracked elsewhere).
- The agent-instructions glue staleness (spec 039) — separate spec.
