# Quickstart: Hook Editor Path Quoting Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/command-recognition-contract.md](./contracts/command-recognition-contract.md) for the authoritative invariants.

## Prerequisites

- Rust toolchain; built test binary via `cargo test` (no manual scratch needed).

## Scenario 1 — predicate recognizes a spaced-path command

```bash
cargo test --test test_hook_editors is_rgt_command_spaced
cargo test --test test_hook_editors element_has_rgt_command_spaced
```

Expected: both pass — the quote-aware predicate recognizes RGT's own spaced-path commands.

## Scenario 2 — re-init is idempotent under a spaced install path

```bash
cargo test --test test_hook_installer_contract spaced
```

Expected: the contract test passes — a second `rgt init` with a spaced `injected_exe()` does not duplicate the hook entry.

## Scenario 3 — no regressions

```bash
cargo test --test test_hook_editors
cargo test --test test_hook_installer_contract
cargo test
```

Expected: all green.

## Automated checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```
