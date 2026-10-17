# Quickstart: Validating Value Extraction Correctness

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove extraction correctness. Invariants in [contracts/extraction-invariants.md](./contracts/extraction-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected:
- `test_record` (updated): the three-date fixture asserts `extracted.len() == 3` (all `Date`) — fails before the fix, passes after.
- New unit tests: `start: 2026-01-15` → 1 node (Date only); `revenue,120000,discount,120000` → 2 distinct values (occurrences 0 and 1); two runs produce identical results.
- `src/types/node.rs` unit tests: occurrence-0 ID unchanged, occurrence-1 ID distinct, deterministic.

## Manual scenario 1 — no spurious date numbers (US1)

```bash
printf 'start: 2026-01-15\n' > /tmp/f.csv
./target/debug/rgt record /tmp/f.csv
./target/debug/rgt graph -f text | rg -c 'DATE|NUMBER'
```
Expected: one DATE node and zero NUMBER nodes from that line (previously 1 date + 3 spurious numbers).

## Manual scenario 2 — same-line identical values tracked separately (US2)

```bash
printf 'revenue,120000,discount,120000\n' > /tmp/f2.csv
./target/debug/rgt record /tmp/f2.csv
./target/debug/rgt status --json
```
Expected: two distinct root nodes for `120000` (previously merged into one).

## Definition of done

- All automated tests pass (this page, above).
- Manual scenarios 1–2 show exactly one Date (no spurious numbers) and two distinct nodes for identical same-line values.
- Every invariant in `contracts/extraction-invariants.md` has at least one automated test.
