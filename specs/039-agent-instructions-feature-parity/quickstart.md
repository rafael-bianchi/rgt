# Quickstart: Agent Instructions Feature Parity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/glue-parity-contract.md](./contracts/glue-parity-contract.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`
- A scratch project dir:

```bash
SCRATCH=$(mktemp -d) && cd "$SCRATCH"
BIN=/Users/rafaelbianchi/dev/repos/rgt/target/debug/rgt
```

## Scenario 1 — installed instructions teach the full set

```bash
"$BIN" init --agent cline --force
"$BIN" init --agent antigravity --force
"$BIN" init --agent kilocode --force
"$BIN" init --agent vibe --force
"$BIN" init --agent windsurf --force
grep -l "doctor" .clinerules .agents/rules/*.md .kilocode/rules/*.md 2>/dev/null
```

Expected: the written rules files mention `doctor`, `gc`, and `--number-format`.

## Scenario 2 — codex writes a complete AGENTS.md

```bash
"$BIN" init --agent codex --force
grep -E "doctor|gc|number-format" AGENTS.md
```

Expected: the generated AGENTS.md includes all three features.

## Scenario 3 — contract tests pass

```bash
cargo test --test test_hook_installer_contract
cargo test
```

## Automated checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
grep -E "doctor\.rs|gc\.rs" CONTRIBUTING.md   # tree listing current
```

Cleanup: `rm -rf "$SCRATCH"`.
