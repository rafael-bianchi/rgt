# Feature Specification: Remove Python Scaffolding Files

**Feature Branch**: `006-remove-python-scaffolding`

**Created**: 2026-08-01

**Status**: Draft

**Input**: User description: "chore: remove stale Python scaffolding files from repo root. main.py, pyproject.toml, and .python-version are leftover artifacts from Spec Kit project scaffolding and serve no purpose in a Rust-only project."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Clean Rust-only project identity (Priority: P1)

A contributor or CI tool inspects the repository and sees only Rust-related files (no Python artifacts). GitHub correctly identifies the repo as 100% Rust.

**Why this priority**: The presence of Python scaffolding files confuses contributors, CI tools, and GitHub's language detection. Removing them is the baseline for a clean project identity.

**Independent Test**: Delete the three files and verify GitHub linguist shows 100% Rust. Verify no CI workflow references Python setup steps.

**Acceptance Scenarios**:

1. **Given** the repository root contains `main.py`, `pyproject.toml`, and `.python-version`, **When** a maintainer deletes them and pushes, **Then** the repository no longer contains any Python-related configuration files.
2. **Given** the files are deleted, **When** GitHub linguist reanalyzes the repo, **Then** the language breakdown shows Rust as the primary language with no Python percentage.
3. **Given** the files are deleted, **When** CI runs (`ci.yml`), **Then** no build step references Python or pip.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System (repository) MUST NOT contain `main.py` in the root directory.
- **FR-002**: System MUST NOT contain `pyproject.toml` in the root directory.
- **FR-003**: System MUST NOT contain `.python-version` in the root directory.
- **FR-004**: `.gitignore` MUST include entries for `.specify/` and `.agents/` to prevent accidental commits of scaffolding and agent directories.
- **FR-005**: CI workflow (`ci.yml`) MUST NOT reference Python tooling or setup steps.

### Key Entities

N/A — this is a repository cleanup task with no data entities.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After deletion, a `find` command at the repo root returns zero results for `main.py`, `pyproject.toml`, and `.python-version`.
- **SC-002**: GitHub language detection shows Rust as the primary language (100%) or lists no Python percentage.
- **SC-003**: `cargo test --all` passes without errors (no regressions from file removal).

## Assumptions

- `main.py`, `pyproject.toml`, and `.python-version` are not referenced by any Rust source code or build scripts.
- `.specify/` and `.agents/` are development tooling that should not be committed to version control.
- CI workflow (`ci.yml`) does not currently perform Python-specific steps; this is a verification task, not a modification task.
- The `exclude` list in `Cargo.toml` already covers `.specify/` and `specs/` for the published crate.
