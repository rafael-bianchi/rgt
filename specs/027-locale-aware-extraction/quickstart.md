# Quickstart: Locale-Aware Extraction

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable end-to-end validation that locale/sign extraction works. See [contracts/number-format-invariants.md](./contracts/number-format-invariants.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir (avoid polluting the repo store):

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
```

## Scenario 1 — Signs are part of the number (always on)

```bash
printf -- '-42\n(500)\n' > neg.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record neg.csv
```

Expected: two nodes recorded, `-42` and `-500` (not `42`/`500`).

## Scenario 2 — US vs EU separators via flag

```bash
printf '1,234.56\n' > us.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record --number-format us us.csv   # -> 1234.56 (one node)

printf '1.234,56\n' > eu.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record --number-format eu eu.csv   # -> 1234.56 (one node)
```

Expected: each yields a single `1234.56` node; no bogus split nodes.

## Scenario 3 — Persistence for the passive path

```bash
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt init --number-format eu >/dev/null
printf '1.234,56\n' > p.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record p.csv   # flagless -> uses eu -> 1234.56
```

Expected: flagless record parses `eu` (persisted). Override check:

```bash
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record --number-format us p.csv
```

Expected: `us` flag wins over the persisted `eu` (would treat `1.234,56` as malformed under `us` grouping → warning, no split nodes).

## Scenario 4 — `auto` inference (default)

`auto` infers **once per file**, so a file must be in one locale. Verify each locale separately:

```bash
printf '1,234.56\n' > auto_us.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record auto_us.csv   # -> 1234.56

printf '1.234,56\n' > auto_eu.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record auto_eu.csv   # -> 1234.56
```

Single-separator numbers fall back to `us` (`.` decimal, `,` thousands):

```bash
printf '1,234\n1.234\n' > auto_single.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record auto_single.csv
```

Expected: `1234` and `1.234` (not `1.234` for the first line).

> **Mixed-locale files**: a file containing both `1,234.56` and `1.234,56` resolves to one format (the first number with both separators wins); the other line is treated as malformed and skipped, not split.

## Scenario 5 — Malformed is skipped + warned, never split

```bash
printf '1.234.567,89\n' > bad.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt record --number-format us bad.csv
```

Expected: a warning on stdout; no nodes recorded for that malformed value (not split into `1`/`234`/`567`/`89`).

## Automated checks

```bash
cargo test --test test_hook_parsers
cargo test --test test_record
cargo test --test test_hooks_contract
cargo test --test test_cli_record_derive_contract
```

Cleanup: `rm -rf "$SCRATCH"`.
