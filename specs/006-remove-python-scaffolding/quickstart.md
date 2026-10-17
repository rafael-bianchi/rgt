# Quickstart: Validating Python Scaffolding Removal

**Feature Branch**: `006-remove-python-scaffolding` | **Date**: 2026-08-01

## Prerequisites

- Repo checked out on feature branch `006-remove-python-scaffolding`

## Scenario 1: Verify Files Are Deleted

**Goal**: Confirm the three Python scaffolding files no longer exist.

**Steps**:
```bash
ls main.py pyproject.toml .python-version 2>&1
```

**Expected outcome**: `No such file or directory` for all three.

---

## Scenario 2: Verify .gitignore Entries

**Goal**: Confirm `.specify/` and `.agents/` are listed in `.gitignore`.

**Steps**:
```bash
grep -E "\.specify|\.agents" .gitignore
```

**Expected outcome**: Both `.specify/` and `.agents/` entries present.

---

## Scenario 3: Build and Tests Pass

**Goal**: Verify no regressions from the cleanup.

**Steps**:
```bash
cargo build
cargo test --all
```

**Expected outcome**: Build succeeds, all tests pass.

---

## Scenario 4: Verify No Python References in Codebase

**Goal**: Confirm no Python files or references remain.

**Steps**:
```bash
find . -not -path './target/*' -not -path './.git/*' \( -name "*.py" -o -name "pyproject.toml" -o -name ".python-version" \) 2>/dev/null
```

**Expected outcome**: No files found (empty output).

---

## Troubleshooting

| Issue | Fix |
|---|---|
| `cargo test` fails | Unlikely — no code changed. Check for unrelated test failures. |
| GitHub linguist still shows Python | Wait for GitHub to recalculate. Language stats update on push and may take minutes. |
