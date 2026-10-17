# Research: Remove Python Scaffolding Files

**Feature Branch**: `006-remove-python-scaffolding` | **Date**: 2026-08-01

## Purpose

Verify that removing `main.py`, `pyproject.toml`, and `.python-version` is safe and will not break any existing functionality.

## Research Questions

### RQ-1: Are the Python files referenced by any Rust source code, build scripts, or CI?

**Decision**: Safe to delete. No references found.

**Rationale**: A grep across the entire codebase for these filenames returns zero results:

```bash
rg "main\.py|pyproject\.toml|\.python-version" src/ tests/ .github/ Cargo.toml
# → no matches
```

The CI workflow (`ci.yml`) only runs Rust-specific commands (`cargo fmt`, `cargo clippy`, `cargo test`). No Python setup steps exist. The Spec Kit scaffolding tools (`.specify/scripts/bash/*.sh`) are Bash scripts and do not reference Python.

---

### RQ-2: Is `.specify/` directory adequately protected from accidental commits?

**Decision**: `.specify/` is listed in `Cargo.toml` `exclude` for the published crate, but NOT in `.gitignore`. Should be added to `.gitignore` to prevent accidental commits during development.

**Rationale**: `Cargo.toml exclude` only prevents files from being included in a `cargo package` distribution. It does not prevent files from being committed to the git repository. `.specify/` contains templates, scripts, and workflow configurations that are development tooling, not product code. Adding to `.gitignore` is the correct protection.

---

### RQ-3: Is `.agents/` directory adequately protected?

**Decision**: Should be added to `.gitignore`.

**Rationale**: The Spec Kit init process warned about potential credential leakage from `.agents/`. This directory contains agent-specific configuration that should not be committed.

---

### RQ-4: Will GitHub linguist correctly reclassify the repo after removal?

**Decision**: Yes. GitHub linguist uses file counts and sizes to determine language percentages. Removing 235 bytes of Python files while retaining thousands of lines of Rust will result in the repo being classified as 100% Rust.

**Rationale**: GitHub linguist recalculates language statistics on push. The three Python files total 235 bytes vs. the Rust codebase which is multiple megabytes. After deletion, Python will drop to 0%, and Rust will be the sole detected language.

---

## Summary of Decisions

| ID | Decision | Rationale |
|---|---|---|
| D-001 | Delete `main.py`, `pyproject.toml`, `.python-version` | No code references, misleading to contributors |
| D-002 | Add `.specify/` to `.gitignore` | Dev tooling, not product code |
| D-003 | Add `.agents/` to `.gitignore` | Potential credential leakage risk |
