# Feature Specification: Installer Config Preservation

**Feature Branch**: `feat/019-installer-config-preservation`

**Created**: 2026-08-27

**Status**: Draft

**Input**: User description: "rgt init (any agent, with or without --force) must never destroy, overwrite, or corrupt existing user configuration — it must merge RGT's entries additively, hard-error loudly (never silently defaulting to empty) when a config file can't be parsed, and replace only RGT's own marked block on re-init."

Source context: `docs/findings-hooks-and-value-extraction.md` §2.1–§2.4 (plan: `docs/spec-plan-from-findings.md`, spec #1, P0).

## Clarifications

### Session 2026-08-27

- Q: How should `rgt init` handle JSON-family configs that contain comments or trailing commas (valid JSONC, e.g. VS Code settings)? → A: Tolerate JSONC — parse files with comments/trailing commas, merge additively, and preserve the comments in the rewritten file; hard-error only on genuinely unparsable JSON.
- Q: What should `rgt init --force` do with a legacy RGT block (open marker present, no end marker)? → A: Leave the file untouched and error with a manual-remediation hint — never guess a boundary.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Existing User Configuration Is Never Destroyed (Priority: P1)

A developer already uses an AI coding tool with their own configuration: other hooks (e.g., RTK, per the repo's own documented pairing), custom keybindings, permissions, model settings, or hand-edited rules files with personal notes. They install RGT and run `rgt init` (and later `rgt init --force` to refresh). Every byte of their pre-existing configuration survives; RGT's entries are added alongside, never replacing or removing anything they did not create.

**Why this priority**: This is the highest-blast-radius defect in the product (§2.1/§2.2). The installer touches configuration for all 13 supported agents, and today it can silently delete a user's other hooks, keybindings, and settings. It corrupts user config for *every* agent, not just RGT's own state — and it is the foundation every later config-writing feature (non-text instructions, Bash-read capture, observability) builds on.

**Independent Test**: Contract/integration tests (extending `tests/contract/hook_installer.rs`, `tests/integration/test_hooks_installer.rs`) that (1) populate realistic agent config files with non-RGT content (a foreign `PreToolUse`/`PostToolUse` hook group, a `keybindings` map, a `[plugins]` table with other plugins), (2) run the installer, and (3) assert the non-RGT content is byte-for-byte identical after install.

**Acceptance Scenarios**:

1. **Given** a Claude Code `settings.json` containing a non-RGT `PostToolUse` hook group (e.g., RTK's), **When** `rgt init --agent claude-code` runs, **Then** the non-RGT hook group remains present and functional alongside RGT's new hooks.
2. **Given** a Cursor `hooks.json` containing pre-existing hook entries, **When** `rgt init --agent cursor` runs, **Then** the pre-existing entries remain present alongside RGT's entries.
3. **Given** an existing config with unrelated top-level settings (keybindings, model config, permissions), **When** the installer runs for that agent, **Then** every unrelated setting remains byte-for-byte identical.
4. **Given** the same scenario with `--force`, **When** the installer runs, **Then** the same preservation guarantee holds (non-RGT content is never removed by `--force`).

---

### User Story 2 - Malformed User Configuration Fails Loudly, Never Silently (Priority: P1)

A developer's agent config file is genuinely unparsable — a partially-written file from a crash, invalid JSON that isn't just comments/trailing commas, a hand-edited TOML with a duplicate key — before they ever run RGT. They run `rgt init`. RGT must NOT treat the file as empty and overwrite it with only RGT's keys (today's behavior: `settings.json` collapses to RGT keys only, silently deleting everything else). Instead, RGT must refuse to touch the file, back it up if any write was attempted, report exactly which file failed and why, and exit with a non-zero status. JSON-family files that only differ from strict JSON by comments or trailing commas (JSONC) are NOT "malformed" — RGT merges them and preserves those comments.

**Why this priority**: The current failure mode is the worst possible outcome of a bug: silent, permanent data loss with zero signal. The constitution mandates "silent failures are prohibited" and meaningful exit codes (0 = success, 1 = expected error, 2 = invalid input). Loud, recoverable failure is strictly required.

**Independent Test**: Unit/contract tests feeding malformed configs (truncated/crash-written files, invalid TOML, genuinely invalid JSON) to each affected agent writer, asserting: no write to the file, an error naming the file, and a non-zero exit code; plus positive tests confirming JSONC files (comments, trailing commas) are merged with comments preserved.

**Acceptance Scenarios**:

1. **Given** a `settings.json` containing JSONC comments and trailing commas, **When** `rgt init --agent claude-code` runs, **Then** RGT configures the agent successfully, keeping the comments and all non-RGT settings intact.
2. **Given** a `settings.json` that is genuinely invalid JSON (e.g., truncated/crash-written), **When** `rgt init --agent claude-code` runs, **Then** the file is left untouched, the command reports an error naming the file and the parse problem, and exits non-zero.
3. **Given** a settings file that fails to read (permissions, concurrent write), **When** the installer runs, **Then** it errors loudly rather than proceeding as if the file were empty.
4. **Given** any agent config RGT cannot safely merge, **When** the installer runs, **Then** RGT never writes a truncated/overwritten version of that file; if any write was attempted, a recoverable backup copy exists.
5. **Given** a malformed file for one agent while other agents' configs are healthy, **When** `rgt init` (no `--agent`) runs, **Then** healthy agents are still configured, the failed agent is reported, and the overall command exits non-zero.

---

### User Story 3 - Re-Init Is Safe: No Duplication, No Truncation (Priority: P1)

A developer re-runs `rgt init` after already being configured, and runs `rgt init --force` to refresh. RGT's entries are not duplicated, and user content placed below RGT's own block in a rules/instructions file (e.g., personal notes under `## RGT Integration` in `AGENTS.md`, `.clinerules`, `.windsurfrules`) survives a `--force` refresh.

**Why this priority**: `--force` is the documented way to refresh an install, so it must be safe by default. Today `truncate_from_marker` deletes everything from RGT's marker line onward — including user notes the rules files' own instructions tell agents to append — and Codex's `--force` path overwrites `AGENTS.md` outright.

**Independent Test**: Integration tests that (1) write an RGT block plus trailing user content, (2) re-run the installer with `--force`, and (3) assert RGT's block is refreshed in place while the trailing user content remains unchanged; and that a second run without `--force` changes nothing.

**Acceptance Scenarios**:

1. **Given** a rules file containing RGT's marked block followed by user-authored notes, **When** `rgt init --force` runs, **Then** RGT's block is updated in place and the user's notes below it are preserved.
2. **Given** an already-configured agent, **When** `rgt init` runs again without `--force`, **Then** no duplicate RGT entries are added and no file is modified.
3. **Given** a project `AGENTS.md` with user content, **When** `rgt init --force` runs with Codex in scope, **Then** the user content is preserved (RGT's section is merged, not the whole file replaced).
4. **Given** a Hermes `config.toml` where `plugins.enabled` is formatted differently than RGT expects (multi-line array, comments), **When** `rgt init --agent hermes` runs, **Then** RGT is added to the existing array without breaking the file's syntax or dropping other entries.

---

### Edge Cases

- What happens when the agent config file exists but is a directory or unreadable (permission denied)? → Loud error naming the path (FR-003); the file is never treated as empty and never overwritten.
- What happens when two `rgt` processes run `rgt init` concurrently against the same config file? → Each merge is additive and writes a backup first, so no content is lost; exact write serialization is a plan-level concern.
- What happens when a rules file already contains an RGT marker but the block's own end marker is missing or user content is interleaved? → Legacy block: `rgt init --force` leaves the file untouched and errors with a manual-remediation hint; it never guesses a boundary (FR-006).
- What happens when a config file is valid but its schema is unexpected (e.g., `hooks` key is an object instead of an array)? → Treated as unmergeable → loud error, file untouched (FR-003/FR-007).
- What happens when the file is empty or whitespace-only — is that "malformed" (loud error) or "effectively absent" (safe to initialize)? → Effectively absent; safe to initialize.
- What happens when a JSONC file (comments/trailing commas) also contains genuinely invalid JSON elsewhere — is it merged or rejected? → Rejected: genuinely invalid JSON is a loud error regardless of JSONC tolerance (FR-010/FR-003).
- What happens on a non-writable config directory — clean error, no partial writes? → Clean error naming the path and the failing write; no partial writes.
- What happens when the RGT marker appears mid-line or in a comment rather than as a standalone block? → Not recognized as an RGT block; treated as user content, never truncated.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The installer MUST merge RGT's hook entries into existing agent configuration additively — RGT-managed entries are added or replaced, and every non-RGT entry (other hooks, keybindings, unrelated settings, other plugins) is preserved byte-for-byte.
- **FR-002**: The installer MUST NOT replace a whole config file or whole config key when it only needs to add RGT entries (covers Claude Code `settings.json`, Cursor `hooks.json`, and Copilot Chat settings).
- **FR-003**: When an existing config file cannot be parsed — after tolerating comments and trailing commas in JSON-family files — the installer MUST NOT treat it as empty and write over it; it MUST leave the file untouched, report the file path and the reason, and exit non-zero.
- **FR-010**: The installer MUST accept JSON-family config files that contain comments or trailing commas (JSONC), merge RGT's entries additively, and preserve those comments and all non-RGT content in the rewritten file. Hard-error behavior (FR-003) applies only to files that remain unparsable after this tolerance.
- **FR-004**: When the installer writes to an existing user file, it MUST first create a recoverable backup copy before that write (additive merges included).
- **FR-005**: Re-running `rgt init` without `--force` MUST be idempotent: no duplicate RGT entries, no file modification.
- **FR-006**: Re-running `rgt init --force` MUST refresh RGT's block in place and preserve all user content outside RGT's block, including content below the block. When an existing RGT block lacks its end marker (a legacy block), `rgt init --force` MUST NOT guess a boundary: it leaves the file untouched and reports a clear error with a manual-remediation hint.
- **FR-007**: The installer MUST write syntactically valid output for every agent format (JSON, TOML, markdown/plain text). It MUST NOT produce duplicate TOML table keys, unbalanced JSON, or broken rules files.
- **FR-008**: When configuring multiple agents in one run, a failure on one agent MUST NOT silently abort or skip the others; each agent's outcome (configured / skipped / failed with reason) MUST be reported, and the overall command MUST exit non-zero if any failure occurred.
- **FR-009**: All error paths MUST produce user-recoverable messages: what went wrong, which file, and how to fix it (per the constitution). No silent failures, no generic I/O-only messages for parser errors.

### Key Entities *(include if feature involves data)*

- **Agent configuration file**: A user-owned file the installer writes to — `settings.json` (Claude Code, Copilot Chat), `hooks.json` (Cursor), `config.toml`/`hooks.toml` (Hermes, Gemini, Mistral Vibe), rules/instructions files (`AGENTS.md`, `.clinerules`, `.windsurfrules`, `.agents/rules/*`, `.kilocode/rules/*`, prompts). Contains user-authored content RGT must never destroy.
- **RGT-managed entry**: A hook, plugin, rule block, or prompt RGT wrote and can identify as its own (marked by an RGT marker or `rgt hook ...` command). The only thing the installer may create or replace.
- **User-authored content**: Everything in a config file that RGT did not create — other tools' hooks, settings, keybindings, personal notes. Must be preserved byte-for-byte.
- **RGT integration block**: A delimited section in a text/rules file marked as RGT's (open marker and matching end marker). The only region `--force` may replace.
- **Backup copy**: A recoverable snapshot of a config file taken before any write that modifies an existing file, used to undo a failed or partial update.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the config-file invariant scenarios (foreign hooks, keybindings, plugins, notes — with and without `--force`) preserve non-RGT content byte-for-byte.
- **SC-002**: 0 cases of silent failure: every scenario where the installer cannot safely merge (genuinely unparsable JSON/TOML after JSONC tolerance, unreadable file, unsupported schema) results in a visible error naming the file and a non-zero exit code.
- **SC-003**: 100% of `rgt init` re-runs (no `--force`) on an already-configured environment leave every file byte-for-byte unchanged (no duplication).
- **SC-004**: 100% of `rgt init --force` re-runs preserve all user content outside RGT's own marked block.
- **SC-005**: 100% of files written by the installer after the change parse cleanly with the target tool's format (valid JSON / valid TOML / valid rules file), and JSONC files rewritten by the installer retain their comments.
- **SC-006**: A documented backup recovery path exists and is exercised in tests: any write that modifies an existing file is preceded by a backup that can restore the original file.

## Assumptions

- The fix targets the four finding families in scope (§2.1 silent settings destruction, §2.2 whole-key overwrite, §2.3 Hermes TOML splice, §2.4 `--force` truncation), including the Codex `AGENTS.md` `--force` whole-file overwrite, which is the same user-facing guarantee ("no user content destroyed under `--force`").
- Preserving non-RGT content byte-for-byte is required even when RGT re-serializes a file it must modify (i.e., RGT must not reformat unrelated portions of a JSON/TOML file it touches). Where byte-preservation of the untouched remainder is impractical for a given format, the installer must back up the original rather than silently rewrite it.
- On malformed input, the installer should still configure healthy agents and report the failed one with a non-zero overall exit (FR-008), rather than aborting the entire run — this is the recoverable, non-silent behavior consistent with the constitution.
- An empty or whitespace-only config file is treated as "effectively absent" and safe to initialize (documented edge case), while a file with unparseable *content* is treated as malformed (loud error).
- JSON-family config files (Claude Code `settings.json`, Copilot Chat VS Code settings) are assumed to potentially contain JSONC comments and trailing commas; those are tolerated and preserved (FR-010), so the loud-error path is reserved for genuinely invalid JSON.
- Backup copies are written alongside the original with a clearly recognizable name (`<path>.rgt.bak`) before ANY write that modifies an existing file — additive merges included (FR-004) — and are never auto-deleted by the installer; cleanup is out of scope for this feature.
- No new runtime dependencies are required to satisfy these requirements (project-local parsing/formatting where feasible); any new dependency must clear the constitution's licensing and justification bar.
- Out of scope: RGT hook runtime behavior, extraction, storage, and non-installer findings (§3, §4, §6–§8 of the findings doc) — those are separate specs in the plan.
