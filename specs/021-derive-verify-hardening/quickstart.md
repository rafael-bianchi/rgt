# Quickstart: Validating Derive & Verify Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Runnable guide to prove `rgt verify`/`rgt derive` are safe and trustworthy. Invariants in [contracts/verify-behavior.md](./contracts/verify-behavior.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain; build in debug: `cargo build`.
- A scratch project dir (the commands open `.rgt/store.db` in the cwd).

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected (new/extended tests):
- expression unit tests: integer coercion (`2*50`→100), builtin rejection (`random()`), parent cap (27 → err, 26 → ok).
- date_diff unit tests: negative duration → err (convention).
- contract tests: invalid `--operation` → exit 2; over-deep/long `--expression` → clean error (no crash).

## Manual scenario 1 — crash regression (US1)

```bash
./target/debug/rgt verify --parents <any-id> --operation EXPRESSION \
  --expression "$(python3 -c "print('('*200000+'1'+')'*200000)")" --result 1; echo "exit=$?"
```
Expected: a clean error naming the depth limit, `exit=2` (not 134/SIGABRT).

## Manual scenario 2 — operation casing (US2)

```bash
./target/debug/rgt verify --parents <id> --operation expression --expression "a + 999999" --result 1; echo "exit=$?"
```
Expected: `Error: unknown operation 'expression' — valid operations: EXPRESSION, DATE_DIFF`, `exit=2` (no "skipping", no exit 0).

## Manual scenario 3 — integer result (US3)

With a tracked parent value, or a pure literal:
```bash
./target/debug/rgt verify --parents '' --operation EXPRESSION --expression "2*50" --result 100; echo "exit=$?"
```
Expected: verifies (exit 0) — integer coercion works.

## Manual scenario 4 — determinism (US3)

```bash
./target/debug/rgt verify --parents <id> --operation EXPRESSION --expression "random()" --result 1; echo "exit=$?"
```
Expected: rejected ("unknown function" / builtins disabled), exit 1 or 2 — never a random success.

## Definition of done

- All automated tests pass (this page, above).
- Manual scenarios 1–4 produce the stated outcomes.
- Every invariant in `contracts/verify-behavior.md` has at least one automated test.
