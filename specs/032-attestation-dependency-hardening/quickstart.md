# Quickstart: Attestation Dependency Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/attestation-hardening-contract.md](./contracts/attestation-hardening-contract.md) for the authoritative invariants.

## Scenario 1 — the pin is exact

```bash
grep "attestation-verify" Cargo.toml   # attestation-verify = "=0.1.0"
```

## Scenario 2 — the risk is documented

```bash
grep -l "attestation-verify\|capture-rgt-attestation" SECURITY.md
```

## Scenario 3 — the capture script fails cleanly today (no attestation exists)

```bash
scripts/capture-rgt-attestation-fixture.sh v0.5.0; echo "exit=$?"
```

Expected: non-zero with a clear "no attestation" message; `tests/fixtures/rgt/` gains no new files (only the README).

## Scenario 4 — the gated test still skips, referencing the path

```bash
cargo test --test test_update_attestation verifies_real 2>&1 | grep -i "skip\|SECURITY"
```

## Automated checks

```bash
cargo test --test test_update_attestation
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```
