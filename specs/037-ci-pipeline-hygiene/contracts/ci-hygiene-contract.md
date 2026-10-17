# Contract: CI Hygiene Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants.

## I. Test-presence gate (FR-001, SC-001)

- `scripts/check-test-presence.sh` exits non-zero for a synthetic diff adding `src/*.rs` with no `tests/` reference.
- Exits zero for a docs-only diff and for a `src/*.rs` + `tests/` pair.
- `main.rs`/`lib.rs` are never flagged.

## II. Coverage (FR-002, FR-003, SC-003)

- `cargo llvm-cov --all` produces a line-coverage percentage.
- The coverage job writes the percentage to the job summary and uploads an artifact; no secret required.
- README shows a coverage badge or an explicit "coverage reported in CI" note.

## III. Action pinning (FR-004, FR-005, SC-002)

- Every third-party `uses:` in `release.yml` and `cd.yml` matches `uses: <owner>/<repo>@<40-hex-sha> (# tag comment)`.
- Each SHA is the real commit of the tag it replaces (verified via GitHub API).

## IV. Non-regression

- Full `cargo test` green (no application code changes).
