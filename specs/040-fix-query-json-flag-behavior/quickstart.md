# Quickstart: Fix Query JSON Flag Behavior

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/query-output-contract.md](./contracts/query-output-contract.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir:

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
BIN=/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt
```

## Scenario 1 — default output is human-readable text

```bash
"$BIN" init --agent codex >/dev/null 2>&1
printf 'revenue,120000\n' > data.csv
"$BIN" record data.csv >/dev/null
ID=$("$BIN" status --json | "$BIN" query node_raw_ 2>/dev/null; true)
# simpler: query the recorded node by its id
"$BIN" query $(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes LIMIT 1")
```

Expected: a plain-text lineage trace — no leading `{`, no `"lineage"` key, showing the node's value, kind, staleness, and `parents: none`.

## Scenario 2 — `--json` is clean, parseable JSON

```bash
"$BIN" query $(sqlite3 .rgt/store.db "SELECT id FROM tracked_nodes LIMIT 1") --json | jq -r '.node_id, .total_ancestors, (.lineage_steps|length)'
```

Expected: the node's id, `0` ancestors, and `1` step — and NO top-level `lineage` key (jq would fail to select `.node_id` if wrapped).

## Scenario 3 — unknown node errors in both modes

```bash
"$BIN" query nonexistent; echo "exit=$?"
"$BIN" query nonexistent --json; echo "exit=$?"
```

Expected: both print a `Node not found` error and `exit=1`.

## Automated checks

```bash
cargo test --test test_query_contract
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Cleanup: `rm -rf "$SCRATCH"`.
