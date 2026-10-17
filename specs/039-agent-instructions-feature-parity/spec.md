# Feature Specification: Agent Instructions Feature Parity

**Feature Branch**: `feat/039-agent-instructions-feature-parity`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "The installed agent-instructions text for 6 of 13 agent integrations has diverged from reality — `CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, `VIBE_PROMPT`, `COPILOT_CLI_RULES` (src/hooks/glue/mod.rs), and `codex_section()` (src/hooks/installer.rs) all only describe record/derive/status/query/graph — none mention doctor, gc, or --number-format. Also folds in CONTRIBUTING.md's stale file/test listings."

Source context: `docs/weakpoints-and-growth-opportunities.md` §3 items 2 and 5. Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #6, P1.

## Clarifications

### Session 2026-08-31

- Q: The repo's own `AGENTS.md` is already current (it documents `--number-format`, `gc`, `doctor`). Should the installed glue constants be brought to exactly match that canonical text, or to a variant? → A: Bring each glue artifact to the same feature set as the canonical `AGENTS.md` content (its "Recording Values / Recording Derivations / Inspecting the Graph" structure with `--number-format`, `rgt gc [--vacuum]`, and `rgt doctor`), while preserving each artifact's existing heading style (`###` vs `##`) and the `with_instruction` non-text-source block that already wraps them. The canonical AGENTS.md is the single source of truth for the feature list; the glue variants are that content in each agent's format.
- Q: Windsurf's instructions text is an inline string in `installer.rs:496` (not a named constant), with the same staleness (only mentions record/derive/status). Include it? → A: Yes — it is the same class of installed glue text and the same root cause; leaving it stale would be a half-fix. It is brought to the same feature set (record/derive/status/query/graph/doctor/gc/--number-format) in its compact style.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Installed Agent Instructions Describe the Full Toolset (Priority: P1)

A user runs `rgt init` and, for the Cline/Roo Code, Google Antigravity, Kilo Code, Mistral Vibe, GitHub Copilot CLI, Codex, or Windsurf integration, the rules/prompt/instructions file RGT writes into their project only teaches `record`/`derive`/`status`/`query`/`graph`. The user never learns that `rgt doctor` exists to diagnose their most likely failure mode, that `rgt gc` can reclaim the store, or that `--number-format` handles their locale. Meanwhile the repo's own `AGENTS.md` (the codex-installed shape) already teaches all of it. After this fix, whatever an agent installs tells the same complete story the repo's own docs do.

**Why this priority**: P1 per the growth plan — this is literally what gets written into real users' projects across 6+ agent integrations, independent of which README they read; higher-impact than the translated-README gap and the same root-cause class.

**Independent Test**: `rgt init --agent <name>` for cline, antigravity, kilocode, vibe, copilot, codex, and windsurf produces instructions text mentioning `doctor`, `gc`, and `--number-format` (contract-tested), and the repo `AGENTS.md`/`CONTRIBUTING.md` listings are current.

**Acceptance Scenarios**:

1. **Given** a fresh project, **When** `rgt init --agent cline` runs, **Then** the written `.clinerules` RGT block mentions `rgt doctor`, `rgt gc [--vacuum]`, and `--number-format`.
2. **Given** the same, **When** `rgt init --agent antigravity`, `--agent kilocode`, `--agent vibe`, `--agent copilot` (CLI instructions file), `--agent codex` (AGENTS.md), and `--agent windsurf` run, **Then** each written artifact mentions the same three features.
3. **Given** the repo's source tree, **When** `CONTRIBUTING.md` is read, **Then** its `src/cli/` listing includes `doctor.rs` and `gc.rs`, and its test examples reference the real test files (including `hook_editors.rs`, `store_integrity.rs`, `test_update_attestation.rs`, `test_update_semver.rs`, `test_doctor_contract.rs`, `test_store_maintenance_contract.rs`).
4. **Given** the updated glue constants, **When** the full test suite runs, **Then** no existing assertion about the previous instruction text breaks (tests that assert `rgt record <file>` presence still pass, since those strings are preserved).

---

### Edge Cases

- The `with_instruction` wrapper already appends the non-text-source block + end marker; the glue edits must NOT duplicate or disturb it (only the body between the RGT marker and that block changes).
- Heading style per artifact: Cline/Copilot use `###`, Antigravity/Kilo/Vibe use `##` — keep each artifact's existing style.
- Compact style (Windsurf/Vibe) vs. structured style (Cline/Antigravity/Kilo/Copilot): each artifact's prose shape is preserved, only the feature mentions are added.
- Tests that assert exact instruction substrings (e.g. `rgt record <file>`, `rgt derive --parents`) must keep passing — those strings are retained.
- `codex_section()` is written into the project `AGENTS.md` (or a fresh one); the repo's committed `AGENTS.md` is already current and must remain consistent with what the code produces.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The glue constants `CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, `VIBE_PROMPT`, `COPILOT_CLI_RULES` (`src/hooks/glue/mod.rs`) and `codex_section()` (`src/hooks/installer.rs`) MUST each mention `rgt doctor`, `rgt gc [--vacuum]`, and `--number-format`, matching the feature set of the canonical `AGENTS.md`.
- **FR-002**: The Windsurf instructions base string (`src/hooks/installer.rs:496`) MUST be brought to the same feature set in its compact style.
- **FR-003**: The `with_instruction` non-text-source block and end-marker wrapping MUST be preserved unchanged (each glue artifact keeps its existing wrapper).
- **FR-004**: `CONTRIBUTING.md` MUST list `doctor.rs` and `gc.rs` in the `src/cli/` tree and reference the real current test files in its test examples.
- **FR-005**: Existing user-facing strings (`rgt record <file>`, `rgt derive --parents`, `rgt status`, `rgt query`, `rgt graph`) MUST be preserved so existing tests and any agent already relying on them keep working.
- **FR-006**: No behavior change to the installer/glue logic — text content only (a rebuild is required for the fix to take effect for users, per the plan's note; the repo docs should note that re-running `rgt init` picks it up).

### Key Entities *(include if feature involves data)*

- **GlueArtifact**: an instruction text RGT writes at `rgt init` — a rules-file block, prompt, or instructions file. Its "feature set" must match the canonical `AGENTS.md`.
- **CanonicalInstructions**: the repo's `AGENTS.md` RGT Integration section — the single source of truth for which features the installed text teaches.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A contract test (or grep across the written artifacts) confirms each of the 7 glue artifacts mentions `doctor`, `gc`, and `--number-format` (unit/contract-tested).
- **SC-002**: `CONTRIBUTING.md`'s file tree and test examples match the real `src/` and `tests/` contents (grep-verified).
- **SC-003**: The full test suite passes with no changes to tests that assert instruction content (only the instruction strings themselves change).
- **SC-004**: `cargo fmt`/`clippy` pass (the Rust string constants compile cleanly).

## Assumptions

- The canonical `AGENTS.md` is correct and authoritative for the feature list (verified current in-session).
- Editing Rust string constants is the fix; no installer logic changes. Users must re-run `rgt init` (or `--force`) to pick up the corrected text — noted in the plan and in the implementation, consistent with the idempotent-write behavior specs 019/028 established.
- No test currently asserts the *absence* of `doctor`/`gc`/`--number-format` in glue text (checked: existing tests assert positive presence of `rgt record <file>` etc., not negative).

## Out of scope

- The translated-README resync (spec 038 — parallel, separate).
- The Copilot CLI dialect verification (an open assumption, not a text-staleness fix — see AGENTS.md's own note).
- Cursor's missing instructions file (spec 019's deliberate gap, tracked elsewhere).
- Any installer/hook behavior change beyond the instruction text.
