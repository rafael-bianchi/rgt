# Feature Specification: Init Observability

**Feature Branch**: `feat/028-init-observability`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "Make a broken or not-yet-active RGT install visible instead of silently capturing nothing: diagnose a non-resolvable `rgt` on PATH, prevent it by writing absolute-path hook commands, scope the Claude Code hook matcher so it doesn't spawn on every tool call, remind users to restart their AI tool after init, and lock in the Copilot CLI event-shape assumption with a test. Findings §2.5–§2.8."

Source context: `docs/findings-hooks-and-value-extraction.md` §2.5 (PATH no-op), §2.6 (empty matcher), §2.7 (Copilot CLI dialect unverified), §2.8 (restart reminder). Plan: `docs/spec-plan-from-findings.md`, spec #10, P2.

## Clarifications

### Session 2026-08-29

- Q: When hooks are installed but the graph has zero recorded values, should `rgt doctor` treat that as a hard failure (non-zero exit) or an advisory warning (zero exit)? → A: Two severity tiers — unambiguous problems (not initialized, unreadable config) exit non-zero; "hooks present but no captures yet" is a printed warning with a PATH hint and exits zero.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Diagnose a Silent Capture Failure (Priority: P1)

A developer has installed RGT hooks (`rgt init` reported success) but provenance capture silently records nothing because the hook subprocess can't resolve `rgt` on its PATH (common with minimal/non-login shells, editor-embedded tool subprocesses, and Windows). Today there is zero signal: `rgt status` shows an empty graph and nothing explains why. The developer needs a single diagnostic command that detects "hooks are installed but nothing has ever been captured" and tells them what to check.

**Why this priority**: §2.5 Medium — the tool currently lies by omission; users believe capture is active when it is not. Diagnosis (not just prevention) is the highest-trust fix and is the standard `doctor` pattern.

**Independent Test**: In a project with hook config files present but an empty graph, `rgt doctor` reports a warning naming PATH resolution as the likely cause and exits zero.

**Acceptance Scenarios**:

1. **Given** hook config files are installed and the graph has zero recorded nodes, **When** the developer runs `rgt doctor`, **Then** it reports a "capture may not be active" warning pointing at PATH resolution and exits zero (advisory, not a hard failure).
2. **Given** hooks are installed and the graph has recorded nodes, **When** the developer runs `rgt doctor`, **Then** it reports the environment healthy and exits zero.
3. **Given** RGT is not initialized in the project at all, **When** the developer runs `rgt doctor`, **Then** it reports "not initialized" and exits non-zero.

---

### User Story 2 - Hooks Reference the CLI by Absolute Path (Priority: P1)

A developer initializes RGT and their hook commands invoke the CLI by absolute path, so a later PATH change or a minimal subprocess environment can't silently disable capture. Today every shell/plugin hook shells out to the bare name `rgt` and depends on PATH resolution at invocation time.

**Why this priority**: §2.5 prevention half — removes the failure mode at its root for the hook agents that spawn a subprocess; low risk, high value.

**Independent Test**: After `rgt init`, the written shell/plugin hook commands reference the CLI by its resolved absolute path, not the bare `rgt` name.

**Acceptance Scenarios**:

1. **Given** a fresh environment, **When** the developer runs `rgt init`, **Then** every shell/plugin hook command that invokes the CLI references it by absolute path.
2. **Given** an already-initialized environment, **When** the developer re-runs `rgt init --force`, **Then** the hook commands are re-written with the current absolute path (idempotent, no duplicate hooks).

---

### User Story 3 - Hook Matcher Fires Only on Relevant Tools (Priority: P2)

A developer's Claude Code hook currently fires a subprocess on *every* tool call (Bash, Edit, Write, WebFetch, …) because its matcher is empty (matches everything). Most of those calls never yield a capturable value. The matcher should be scoped to the tools RGT actually captures — including `Bash` (for Bash-read capture) — so the hook no longer spawns on irrelevant tools.

**Why this priority**: §2.6 Low (perf/behavior surprise) — no correctness bug, but it avoids a pointless subprocess on every tool call and makes intent explicit.

**Independent Test**: The written Claude Code hook matcher matches the file/Bash tools but not, e.g., WebFetch/WebSearch (asserted against the generated artifact).

**Acceptance Scenarios**:

1. **Given** a fresh init, **When** the Claude Code hook is written, **Then** its matcher is scoped to the read/write tools plus `Bash` (not empty).
2. **Given** the scoped matcher, **When** a `Bash` tool call occurs, **Then** the hook still fires (Bash-read capture is preserved).

---

### User Story 4 - Init Reminds the User to Restart (Priority: P2)

A developer runs `rgt init` and immediately uses their AI tool in the same session, expecting capture to work — but hook config is read at session start, so capture stays off with no signal. `rgt init`'s success output must explicitly tell them to restart their AI tool for capture to take effect.

**Why this priority**: §2.8 Low — a one-line reminder that eliminates a silent no-op workflow; also reinforces the already-documented README note in-product.

**Independent Test**: When `rgt init` newly configures a hook, its output contains an explicit restart instruction; on a pure no-op re-run it does not nag.

**Acceptance Scenarios**:

1. **Given** at least one hook is newly configured, **When** `rgt init` completes, **Then** its output includes an explicit "restart your AI tool now for capture to take effect" instruction.
2. **Given** all hooks are already installed (no changes), **When** `rgt init` re-runs, **Then** it does not repeat the restart reminder as if newly configured.

---

### User Story 5 - Copilot CLI Dialect Assumption Is Locked In (Priority: P3)

The Copilot CLI dual-dialect parser guesses at the event shape and has no test fixture based on a real captured payload. Until a real payload is captured, the assumed shape must be documented as an assumption and pinned by a test so any future divergence is a deliberate, tested change.

**Why this priority**: §2.7 Low — not a proven bug; reduces the risk that a silent shape mismatch goes unnoticed.

**Independent Test**: A unit test asserts the assumed Copilot CLI event shape (JSON-stringified `toolArgs` and/or command-dialect fallback) produces the documented capture, with the assumption recorded in the spec.

**Acceptance Scenarios**:

1. **Given** a synthetic Copilot CLI event in the assumed shape, **When** it is normalized, **Then** it produces the documented capture (or `content: None` for the documented gap).
2. **Given** the assumption is documented, **When** a reviewer reads the spec, **Then** they can see the Copilot CLI dialect is an assumption pending live verification.

---

### Edge Cases

- `rgt doctor` run in a directory with no `.rgt` store (never initialized)?
- `rgt doctor` run when hooks are present but the CLI is genuinely not on PATH (Windows vs. Unix)?
- An absolute path containing spaces or non-ASCII characters written into a hook command (quoting/cross-platform)?
- `rgt init --force` re-writing an absolute path after the binary moves (stale path)?
- The restart reminder when only rules-file agents (no subprocess hooks) are configured?
- The Copilot CLI matcher (`.*`) vs. the CLI dialect — both must remain consistent?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CLI MUST provide a `rgt doctor` diagnostic subcommand that reports the health of hook installation and capture with two severity tiers: unambiguous problems (not initialized, unreadable/malformed config) are **errors** that exit non-zero, while "hooks present but no captures yet" is a **warning** that exits zero.
- **FR-002**: `rgt doctor` MUST detect the "hooks installed but no values ever recorded" signature and MUST emit an advisory warning advising the user to verify the CLI is resolvable from the same shell context the AI tool uses (PATH resolution). It MUST likewise emit an advisory warning (exit zero) when no hooks are installed at all.
- **FR-003**: `rgt init` MUST write hook commands that invoke the CLI by its absolute path (resolved at init time from the running executable), rather than a bare command name, for every shell/plugin hook agent.
- **FR-004**: The Claude Code hook matcher MUST be scoped to the tools RGT captures — at minimum the read/write tools plus `Bash` — and MUST NOT remain empty (matches-everything). Bash-read capture MUST be preserved.
- **FR-005**: `rgt init` MUST print an explicit restart instruction when it newly configures at least one hook, and MUST NOT repeat it on a pure no-op re-run.
- **FR-006**: A test MUST pin the assumed Copilot CLI event shape, and the spec/README MUST document that the Copilot CLI dialect is an assumption pending live verification against the real tool.

### Key Entities *(include if feature involves data)*

- **Diagnostic**: A single health check result — a name, a status (healthy/warning/failure), and a human-readable message — that `rgt doctor` aggregates into an overall exit status.
- **HookArtifact**: A written agent hook configuration (command string + matcher) whose shape `rgt init` controls and `rgt doctor` inspects.
- **InstallState**: The observable facts `rgt doctor` reasons over — whether the store is initialized, which hook artifacts are present, and whether any values have been recorded.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `rgt doctor` exits non-zero on unambiguous problems (not initialized) and exits zero (with a printed PATH warning) in the "hooks installed, zero recorded values" scenario (contract-tested).
- **SC-002**: After `rgt init`, no shell/plugin hook command references the CLI by bare name — every one uses an absolute path (contract-tested across the hook agents).
- **SC-003**: The Claude Code hook matcher is no longer empty and does not match non-file/Bash tools (unit-tested against the generated artifact).
- **SC-004**: `rgt init` output contains a restart instruction when hooks are newly configured, and omits it on a no-op re-run (contract-tested).
- **SC-005**: A unit test pins the Copilot CLI dialect assumption, and the assumption is recorded in the spec (unit-tested).

## Assumptions

- `rgt doctor` is a new subcommand (matching the `brew doctor`/`flutter doctor` precedent) rather than overloading `rgt status`; `status` remains focused on the graph.
- Absolute-path resolution uses the running executable's own path; this is machine-specific but safe because hooks live in the user's own per-machine config directories.
- The Claude Code tools RGT captures are named `Read`, `Edit`, `Write`, and `Bash`; the matcher must keep `Bash` to preserve the Bash-read capture delivered in 026.
- The Copilot CLI dual-dialect assumption is accepted as-is for this feature; live verification against a real payload is a separate follow-up (no captured payload exists yet).
- The `rgt doctor` PATH check is symptom-based (hooks present but zero captures) rather than attempting a cross-platform PATH lookup; it points the user at the right diagnostic.

## Out of scope

Non-ISO date recognition (§3.3's invisible-dates half), store maintenance/GC (§7.2 LOWs → 029), and any change to the Copilot CLI dialect logic itself (only documenting + testing the assumption).
