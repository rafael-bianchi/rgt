# Quickstart: Validating Change-Detection Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove detection correctness. Invariants in [contracts/detection-invariants.md](./contracts/detection-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.
- A scratch dir for manual runs.

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected (new/expanded `tests/unit/detection.rs`):
- tier1/tier2/evaluate_file_change unit tests: forced re-hash detects a same-size/same-mtime rewrite; fast path skips hashing when fresh; `NotFound` vs `CheckFailed`; dev/ino change detection; hash-from-bytes equals content hash.

## Manual scenario 1 — same-mtime rewrite detected (US1)

```bash
SCRATCH=$(mktemp -d); cd "$SCRATCH"
printf 'amount,100\n' > f.csv
./target/debug/rgt init --agent codex --force >/dev/null 2>&1
./target/debug/rgt record f.csv
# force the same size + same mtime, different content
printf 'amount,999\n' > f.csv
touch -r f.csv f.csv 2>/dev/null   # best-effort preserve mtime (or use `SetFileTime`)
./target/debug/rgt status --stale-only
```
Expected: within the re-hash window, `f.csv`'s node is reported stale (not "all fresh").

## Manual scenario 2 — unreadable vs deleted (US3)

```bash
chmod 000 f.csv       # permission error (not NotFound)
./target/debug/rgt status   # should NOT mark the node as FILE_DELETED
chmod 644 f.csv; rm f.csv
./target/debug/rgt status   # now FILE_DELETED
```
Expected: permission failure yields a distinct non-deletion result; actual deletion yields `FILE_DELETED`.

## Definition of done

- All automated tests pass (this page, above).
- Every invariant in `contracts/detection-invariants.md` has at least one automated test.
