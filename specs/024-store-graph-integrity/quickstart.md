# Quickstart: Validating Store & Graph Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove the store/graph integrity fixes. Invariants in [contracts/store-graph-invariants.md](./contracts/store-graph-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.
- A scratch dir for `rgt` manual runs.

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected (new/extended):
- Store/query tests: self-loop and cycle edge inserts rejected (nothing persisted); `build_from_db` errors on a seeded cycle; `ON CONFLICT` updates value columns; node IDs ≥ 32 hex.
- `db.rs` test: every connection has a non-zero `busy_timeout`.
- `status.rs` test: a failing invalidation check yields a non-zero error.
- Hook test: batch insert is one transaction.

## Manual scenario 1 — cycle rejected (US1)

```bash
SCRATCH=$(mktemp -d); cd "$SCRATCH"
./target/debug/rgt init --agent codex --force >/dev/null 2>&1
# record two values, derive A from B, then attempt a cycle (B from A) — the
# second derive must be rejected with a clear error and exit non-zero
./target/debug/rgt record <(printf 'a,100\nb,200\n') ...   # illustrative
```
Expected: a cycle-forming `rgt derive` is rejected at insert time (non-zero exit, nothing persisted).

## Manual scenario 2 — ID width (US3)

```bash
./target/debug/rgt record <(printf 'x,42\n') ... 
./target/debug/rgt status --json | rg -o 'node_raw_[0-9a-f]{12,}'
```
Expected: node IDs are ≥ 32 hex chars (previously 12).

## Definition of done

- All automated tests pass (this page, above).
- Every invariant in `contracts/store-graph-invariants.md` has at least one automated test.
