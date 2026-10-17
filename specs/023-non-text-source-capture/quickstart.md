# Quickstart: Validating Non-Text Source Capture

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove the non-text-source gap is handled. Invariants in [contracts/non-text-invariants.md](./contracts/non-text-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.
- A scratch dir for `rgt init`/`record` manual runs (run in the scratch cwd; see the 019 quickstart's `$HOME`/project-scope note).

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected (new/extended):
- `test_cli_record_derive` (extended): a binary fixture → "unsupported file format for `rgt record`" error (not the raw UTF-8 message).
- `hook_installer` (extended): the instruction text appears exactly once in each of the 7 rules artifacts + `CLAUDE.md`; idempotent on re-run; user content preserved.

## Manual scenario 1 — `rgt record` on a PDF (US1)

```bash
printf '%%PDF-1.4 binary-ish\n' > /tmp/report.pdf
./target/debug/rgt record /tmp/report.pdf; echo "exit=$?"
```
Expected: `Error: unsupported file format for rgt record ...` naming plain text/CSV / the agent's own read tool; `exit=1` (not the raw UTF-8 error).

## Manual scenario 2 — instructions present after init (US2/US3)

```bash
SCRATCH=$(mktemp -d); cd "$SCRATCH"
/path/to/target/debug/rgt init --agent cline --force
grep -c "Non-Text Sources" .clinerules
/path/to/target/debug/rgt init --agent claude-code --force
grep -c "Non-Text Sources" CLAUDE.md
```
Expected: both files contain the instruction exactly once; re-running `init` does not duplicate it.

## Definition of done

- All automated tests pass (this page, above).
- Manual scenarios 1–2 produce the stated outcomes.
- Every invariant in `contracts/non-text-invariants.md` has at least one automated test.
