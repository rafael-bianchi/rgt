# Data Model: Init Observability

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

No new persisted entities. This feature introduces transient diagnostics and touches existing hook-artifact shapes.

## 1. Diagnostic (new, transient)

A single health check result produced by `rgt doctor`, not persisted.

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `&'static str` | Check identifier (e.g., "store", "hooks", "captures"). |
| `status` | enum `DiagnosticStatus` | `Healthy` \| `Warning` \| `Error`. |
| `message` | `String` | Human-readable explanation / next step. |

- Aggregation rule: any `Error` → process exit 1; `Warning`s print but exit 0; all `Healthy` → exit 0.
- A `Warning` is emitted for the "hooks present + zero recorded values" signature; an `Error` for "not initialized" or an unreadable config.

## 2. HookArtifact (existing, shape modified)

The agent hook configuration RGT writes. Two fields are relevant here:

- `command`: for the eight subprocess agents (claude-code, cursor, copilot, gemini, vibe, opencode, pi, hermes), now an **absolute path** to the CLI plus the subcommand (e.g. `"/abs/path/rgt" hook post`). Instructions-only agents (codex, windsurf, cline, antigravity, kilocode) keep plain instruction text (`rgt record <file>` etc.).
- `matcher`: Claude Code only, changed from `""` to `Read|Edit|Write|Bash`.

## 3. InstallState (transient, observed by `doctor`)

The facts `doctor` reasons over, gathered read-only at run time:

- `store_initialized`: `.rgt/store.db` exists and opens.
- `hooks_present`: ≥1 RGT hook artifact exists (contains the RGT marker for text/JSON artifacts, or exists for plugin files).
- `nodes_recorded`: count of tracked nodes > 0.

Mapping to diagnostics:

| `store_initialized` | `hooks_present` | `nodes_recorded` | Result |
|---------------------|-----------------|------------------|--------|
| false | — | — | `Error` "not initialized" (exit 1) |
| true | false | — | `Warning` "hooks not installed" (exit 0) |
| true | true | false | `Warning` "capture may not be active; check PATH" (exit 0) |
| true | true | true | `Healthy` (exit 0) |
