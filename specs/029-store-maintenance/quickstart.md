# Quickstart: Store Maintenance

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable end-to-end validation. See [contracts/store-maintenance-invariants.md](./contracts/store-maintenance-invariants.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir:

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
BIN=/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt
```

## Scenario 1 — the store records its schema version

```bash
"$BIN" init --agent codex >/dev/null 2>&1
sqlite3 .rgt/store.db "PRAGMA user_version;"
```

Expected: prints `1` (a fresh store is stamped with the current schema version).

## Scenario 2 — a newer store fails with a clear error

```bash
"$BIN" init --agent codex >/dev/null 2>&1
sqlite3 .rgt/store.db "PRAGMA user_version = 99;"
"$BIN" status; echo "exit=$?"
```

Expected: a message like "store schema version 99 is newer than this build supports" and `exit=1` (not "no such column").

## Scenario 3 — `rgt gc` removes only obsolete nodes

```bash
"$BIN" init --agent codex >/dev/null 2>&1
printf 'a,1\n' > data.csv && "$BIN" record data.csv >/dev/null
rm data.csv
"$BIN" status >/dev/null 2>&1     # marks the node stale (FILE_DELETED)
"$BIN" gc                          # collects the obsolete node
```

Expected: `gc` reports `1` node removed; a second `rgt gc` reports `0`.

## Scenario 4 — `--vacuum` is opt-in

```bash
"$BIN" gc --vacuum
```

Expected: deletes any remaining obsolete nodes and then reclaims pages; the default `rgt gc` (no flag) does not run `VACUUM`.

## Automated checks

```bash
cargo test --test test_store_maintenance_contract
cargo test --test test_invalidation
cargo test --test test_store_integrity
```

Cleanup: `rm -rf "$SCRATCH"`.
