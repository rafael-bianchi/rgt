# Contract: Update-Contract Test Fixture Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/contract/test_update_contract.rs` and a unit test for the derivation helper.

## I. Fixture newer-than-current invariant (FR-001, FR-002, FR-003)

- `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace` MUST stub a release tag derived from `CARGO_PKG_VERSION` that is strictly newer under SemVer precedence.
- The derived tag for `0.5.0` is `v0.6.0`; the gate must let the pipeline proceed to the download/verify stage.
- The invariant holds for pre-release current versions too (e.g. `0.5.0-rc.1` → `v0.6.0` is strictly newer).

## II. Production-code untouched (FR-001, SC-002)

- `cargo test --test test_update_contract` passes with zero changes to `src/`.

## III. Older/explicit-stub tests unchanged (FR-004)

- `older_or_equal_tag_is_up_to_date_not_an_update`, `skip_checksum_bypasses_the_checksum_refusal`, `attestation_failure_is_a_hard_refusal_unless_skipped`, `checksum_mismatch_aborts_before_install`, and `explicit_older_version_proceeds_with_warning_not_refusal` keep their literal stub tags and their existing semantics.

## IV. Non-regression (Constitution IX, XI)

- The whole `test_update_contract` target (13 tests) is green.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
