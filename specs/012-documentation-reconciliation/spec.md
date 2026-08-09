# Feature Specification: Documentation Reconciliation

**Feature Branch**: `docs/012-documentation-reconciliation`

**Created**: 2026-08-08

**Status**: Draft

**Input**: User description: "Full reconciliation of code → ALL documentation (text, code comments, and everything)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - README Matches Actual CLI (Priority: P1)

A new user reads the README and every command, flag, and example works exactly as documented. The README documents all 8 CLI subcommands (`init`, `status`, `query`, `graph`, `hook`, `mcp`, `update`, `verify`) with accurate flags and exit codes. No documented feature is missing from code; no code feature is undocumented.

**Why this priority**: The README is the primary onboarding surface. If it lies about behavior, users waste time and lose trust. This is the core of the reconciliation.

**Independent Test**: For each of the 8 CLI subcommands, run `rgt <cmd> --help` and verify the output matches the README's description of that command.

**Acceptance Scenarios**:

1. **Given** a fresh install, **When** the user runs `rgt --help`, **Then** all 8 subcommands shown match the README's CLI Reference section.
2. **Given** the README's Quickstart example flow, **When** each command is run exactly as written, **Then** every command succeeds with the documented exit codes.
3. **Given** the README's install section, **When** each install method is used, **Then** `rgt` is installed via the documented method (`curl | sh`, `brew install rafael-bianchi/rgt/rgt`, `cargo install --git`).

---

### User Story 2 - AGENTS.md Matches Code (Priority: P1)

The AGENTS.md file (developer-facing documentation) lists commands, module structure, and conventions that match the actual codebase. No stale commands, outdated install paths, or obsolete hook formats remain.

**Why this priority**: AGENTS.md guides AI coding agents that develop RGT. Stale info causes agents to make incorrect changes.

**Independent Test**: Grep AGENTS.md for every CLI command and module path mentioned — verify each exists in code.

**Acceptance Scenarios**:

1. **Given** AGENTS.md's CLI Commands table, **When** each command is checked against `src/main.rs`, **Then** all commands and flags match.
2. **Given** AGENTS.md's install commands, **When** checked against current distribution, **Then** they use `brew install rafael-bianchi/rgt/rgt` and `cargo install --git https://github.com/rafael-bianchi/rgt`.

---

### User Story 3 - Code Comments Document Public API (Priority: P2)

Every `pub fn` in the `src/` directory has a `///` doc comment explaining its purpose, parameters, return value, and error conditions. Functions that compute or verify values document their inputs/outputs and any failure modes.

**Why this priority**: Doc comments are the API contract for library consumers and the basis for `cargo doc`. Missing comments make the API undiscoverable.

**Independent Test**: For every `pub fn` in `src/`, check that the preceding line has a `///` doc comment. Flag any that don't.

**Acceptance Scenarios**:

1. **Given** the list of public functions in `src/verify/`, **When** checked, **Then** `verify_expression` and `verify_date_diff` have doc comments.
2. **Given** the list of public functions in `src/updater/`, **When** checked, **Then** `parse_checksums`, `find_checksum`, `compute_sha256`, `verify_checksum`, `fetch_latest_release`, `fetch_release_by_tag`, `find_asset`, `find_checksum_asset`, `download_asset`, and `verify_asset_checksum` have doc comments.
3. **Given** the list of public functions in `src/mcp/handlers.rs`, **When** checked, **Then** `handle_record_value`, `handle_record_derivation`, and `handle_query_provenance` have doc comments.

---

### Edge Cases

- What happens when a documented command was renamed? The README must use the current name, and the old name must be removed.
- What happens when an install path changed? All documentation (README, AGENTS.md) must use the current path.
- What happens when a public function is part of an internal module (e.g., `updater/checksum.rs` used by `verify`)? It still needs a doc comment if it's `pub` within the crate.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The README MUST document all 8 CLI subcommands (`init`, `status`, `query`, `graph`, `hook`, `mcp`, `update`, `verify`) with accurate flags and descriptions matching `src/main.rs`.
- **FR-002**: The README MUST use the current install commands: `curl -fsSL .../install.sh | sh`, `brew install rafael-bianchi/rgt/rgt`, and `cargo install --git https://github.com/rafael-bianchi/rgt`.
- **FR-003**: AGENTS.md MUST use current install commands and CLI command tables matching code.
- **FR-004**: Every `pub fn` in `src/` MUST have a `///` doc comment. Functions verified missing comments MUST be updated.
- **FR-005**: CONTRIBUTING.md MUST exist with project structure, test commands, and contribution guidance (already created — verify accuracy).
- **FR-006**: Local tooling directories (`.kilocode/`, `.kilo/`) MUST be in `.gitignore` — no tooling artifacts committed.

### Key Entities

- **CLI Surface**: The 8 subcommands defined in `src/main.rs` `Commands` enum. The single source of truth for user-facing behavior.
- **Public API**: All `pub fn` in `src/` modules. The contract for library consumers and `cargo doc` output.
- **Install Paths**: The distribution channels documented in README and AGENTS.md (shell, Homebrew main-repo formula, Cargo git install).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of CLI subcommands in `src/main.rs` are documented in the README with matching flags.
- **SC-002**: 100% of `pub fn` in `src/` have `///` doc comments (verified by a script).
- **SC-003**: Zero occurrences of `rafael-bianchi/tap` or bare `cargo install rgt` in README, AGENTS.md, or CONTRIBUTING.md.
- **SC-004**: `git status --short` shows no untracked tooling directories.
- **SC-005**: `cargo doc --no-deps` builds successfully without warnings for undocumented public items.

## Assumptions

- The code and tests are the single source of truth. Documentation must match them, not the reverse.
- Doc comments are the primary documentation for library consumers; README is for end users.
- `.kilocode/` is a legacy tooling directory and should be gitignored (matching `.kilo/`).
