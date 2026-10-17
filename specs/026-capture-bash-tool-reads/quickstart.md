# Quickstart: Validating Bash-Tool Read Capture

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove Bash reads are captured. Invariants in [contracts/bash-capture-invariants.md](./contracts/bash-capture-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected (new `hook_parsers` tests):
- The §8.1 Bash payload normalizes to a non-empty path + content for `None`/`claude-code`/`cursor`.
- `rtk read /p/f` → `/p/f`; `rtk cat /p/f` → none; `grep` → path.
- stdout → content; empty stdout → path-only; non-read command → no-op.

## Manual scenario 1 — Bash read captured (US1)

```bash
SCRATCH=$(mktemp -d); cd "$SCRATCH"
printf 'amount,120000\n' > report.csv
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt init --agent codex --force >/dev/null 2>&1
# Simulate a Claude Code Bash `cat` PostToolUse event:
cat > /tmp/event.json <<EOF
{"event":"PostToolUse","tool_name":"Bash","tool_input":{"command":"cat $SCRATCH/report.csv"},"tool_response":{"stdout":"amount,120000\n"}}
EOF
cat /tmp/event.json | /Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt hook post --agent claude-code
/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt status --json | rg -o '"total_nodes": [0-9]+'
```
Expected: `total_nodes` increments (the Bash read's values are now captured).

## Manual scenario 2 — RTK wrapper (US2)

Feed an `rtk read` command in place of `cat` in the event above; the path must still be extracted and the values captured.

## Definition of done

- All automated tests pass (this page, above).
- Every invariant in `contracts/bash-capture-invariants.md` has at least one automated test.
