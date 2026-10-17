# Research: Update Contract Test Fixture Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `tests/contract/test_update_contract.rs` (the two failing tests), `src/cli/update.rs` (`execute_update_with_io` gating), and `src/updater/version.rs` (`latest_is_newer`).

## 1. Root cause (FR-001)

**Finding**: `execute_update_with_io` gates auto-update on `version::latest_is_newer(latest_version, current_version)` where `current_version = env!("CARGO_PKG_VERSION")` (`src/cli/update.rs:133,166`). The two failing tests stub a release tagged `v0.4.1`. When the project was at 0.4.0 that tag was genuinely newer and the tests proceeded to the download stage; after the bump to 0.5.0 the gate short-circuits with "already at the latest version" and the tests fail at `unwrap_err()` on an `Ok` value.

## 2. Deriving the "newer" stub tag at test time (FR-002, FR-003)

**Decision**: Add a helper inside the test module:

```rust
fn newer_stub_tag() -> String {
    let cur = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION must be a valid semver");
    format!("v{}.{}.0", cur.major, cur.minor + 1)
}
```

`cur.minor + 1` yields a strictly-newer tag under semver precedence for every current and foreseeable release shape: for `0.5.0` → `v0.6.0`; for a pre-release `0.5.0-rc.1` → `v0.6.0` (still strictly greater — the bumped minor dominates, and any pre-release is below its own core anyway). The `semver` crate is already a dependency (used by `src/updater/version.rs`), so Constitution XI is satisfied with no new crate.

**Rationale**: Hardcoding a "future" literal (e.g. `v0.6.0`) merely relocates the failure to the next version bump — exactly the silent-staleness class this P0 unblock exists to fix. Deriving from the compile-time constant makes the tests version-proof. `env!("CARGO_PKG_VERSION")` is deterministic (Constitution IX).

**Alternatives considered**:
- Hardcode `v0.6.0`: rejected — breaks at the next bump; no durability.
- Bump the patch instead of minor: viable but smaller margin; minor is a clearer signal of "definitely newer" and matches how real releases advance.
- Reuse `version::latest_is_newer`'s own parser instead of `semver` directly: no — the helper wants to *construct* a version, not compare; `semver::Version` is the existing parsed type.

## 3. Confinement to the two "proceeds past gating" tests (FR-004)

**Finding**: Other tests in the file intentionally use older or explicit-target stubs where "older" semantics is the point:
- `older_or_equal_tag_is_up_to_date_not_an_update` (v0.4.0 / v0.3.9) — asserts up-to-date.
- `skip_checksum_bypasses_the_checksum_refusal` (v0.4.1, `--skip-checksum`) — asserts it gets past the checksum refusal *even when* older/explicit.
- `attestation_failure_is_a_hard_refusal_unless_skipped` (v0.4.1) — refusal semantics, not newer-gating.
- `checksum_mismatch_aborts_before_install` (v0.4.1) — verification-before-replace ordering.
- `explicit_older_version_proceeds_with_warning_not_refusal` (v0.3.0) — explicit-target older path.

**Decision**: Only `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace` switch to `newer_stub_tag()`. The others keep their literals (their assertions depend on "older/explicit" behavior, which is unaffected by the 0.4.1 → 0.5.0 bump). This is verified by the full 13-test suite staying green.

**Rationale**: Minimal, surgical change; preserves each test's intent; avoids churn in tests whose semantics don't depend on "newer than current".

## 4. Helper correctness guard (SC-003)

**Decision**: Add a tiny unit test inside the same file (or `tests/unit/test_update_semver.rs`) that runs `newer_stub_tag()` against several synthetic `CARGO_PKG_VERSION`-style inputs via a parameterized internal function (`fn newer_tag_for(current: &str) -> String`), asserting `latest_is_newer` (or direct semver comparison) says the derived tag is strictly newer for `0.5.0`, `1.2.3`, `0.0.1`, and `0.5.0-rc.1`.

**Rationale**: Proves the helper's invariant (SC-003) without needing an actual future version bump; guards against a future helper regression.
