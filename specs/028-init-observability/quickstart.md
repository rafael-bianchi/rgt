# Quickstart: Init Observability

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable end-to-end validation. See [contracts/doctor-invariants.md](./contracts/doctor-invariants.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir to avoid touching the repo store:

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
BIN=/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt
```

## Scenario 1 — `rgt doctor` reports "not initialized" (exit 1)

```bash
"$BIN" doctor; echo "exit=$?"
```

Expected: a "not initialized … run `rgt init`" message and `exit=1`.

## Scenario 2 — fresh init, then `doctor` warns (exit 0)

```bash
"$BIN" init --agent codex >/dev/null
"$BIN" doctor; echo "exit=$?"
```

Expected: hooks present but zero captures → a PATH warning, `exit=0`.

## Scenario 3 — after a capture, `doctor` is healthy (exit 0)

```bash
printf 'amount,42\n' > data.csv
"$BIN" record data.csv
"$BIN" doctor; echo "exit=$?"
```

Expected: healthy report, `exit=0`.

## Scenario 4 — hook command references an absolute path

```bash
"$BIN" init --force --agent codex >/dev/null
# The written hook config (e.g. Claude Code settings.json) should reference the
# binary by absolute path. For a rules-file agent this does NOT apply.
```

Expected: subprocess-spawning agents' `command` uses the absolute `rgt` path (see contract II).

## Scenario 5 — restart reminder on first configure only

```bash
"$BIN" init --force --agent codex   # newly configured -> prints restart reminder
"$BIN" init --agent codex           # no-op -> no reminder
```

Expected: reminder appears the first time, not the second.

## Automated checks

```bash
cargo test --test test_doctor_contract
cargo test --test test_hook_parsers
cargo test --test test_hook_editors
```

Cleanup: `rm -rf "$SCRATCH"`.
