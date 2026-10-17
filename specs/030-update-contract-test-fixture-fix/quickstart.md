# Quickstart: Update Contract Test Fixture Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/fixture-invariants.md](./contracts/fixture-invariants.md) for the authoritative invariants.

## Prerequisites

- Rust toolchain (project is already buildable).

## Scenario 1 — the two previously-failing tests now pass

```bash
cargo test --test test_update_contract
```

Expected: all 13 tests pass, including `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace`.

## Scenario 2 — the derivation helper is strict-newer across version shapes

```bash
cargo test --test test_update_contract newer_tag
```

Expected: the helper unit test(s) pass, asserting `newer_tag_for` produces a strictly-newer tag for `0.5.0`, `1.2.3`, `0.0.1`, and `0.5.0-rc.1`.

## Scenario 3 — no production code changed

```bash
git diff --stat -- src/    # expect: empty
```

Expected: no output — the fix is confined to `tests/`.

## Automated checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Cleanup: none (no scratch state created).
