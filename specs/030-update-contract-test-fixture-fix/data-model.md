# Data Model: Update Contract Test Fixture Fix

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data is involved. This feature is confined to the test suite; the only "entity" is a transient test helper.

## 1. NewerStubTag (transient, test-only)

- A helper `newer_tag_for(current: &str) -> String` in `tests/contract/test_update_contract.rs` that parses a semver string and returns `v{major}.{minor+1}.0`.
- `newer_stub_tag() -> String` wraps it with `env!("CARGO_PKG_VERSION")`.
- Invariant: the returned tag is strictly newer than the input under SemVer precedence.

## 2. Tracked upstream behavior (unchanged)

The update gating logic (`src/cli/update.rs`, `src/updater/version.rs`) is production code and is NOT modified. The fixture change makes the *test stubs* track the real gate's semantics (tag must be strictly newer than `CARGO_PKG_VERSION`).
