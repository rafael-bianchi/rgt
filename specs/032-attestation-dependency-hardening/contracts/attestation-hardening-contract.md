# Contract: Attestation Hardening Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

## I. Exact pin (FR-001, SC-001)

- `Cargo.toml` contains `attestation-verify = "=0.1.0"` (grep-verified).

## II. Risk documentation (FR-002, SC-002)

- `SECURITY.md` exists and mentions: `attestation-verify`, the untested-in-production policy, the fail-closed rationale, and `scripts/capture-rgt-attestation-fixture.sh`.

## III. Capture script (FR-003, SC-003)

- `scripts/capture-rgt-attestation-fixture.sh` exists and is executable.
- Its digest/URL decomposition is unit-tested.
- Pointed at today's v0.5.0 (no attestation), it exits non-zero with a clear "no attestation" message and writes nothing.

## IV. Self-skip preserved (FR-004)

- `verifies_real_rgt_bundle_when_fixture_present` still skips when the fixture is absent, with a message referencing the capture path.

## V. Non-regression (SC-004)

- Full `cargo test` green; `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
