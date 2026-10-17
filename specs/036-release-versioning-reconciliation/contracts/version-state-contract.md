# Contract: Release Versioning State

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `scripts/check-version-state.sh`.

## I. Single source of truth (SC-001)

- `Cargo.toml` version == manifest `.` key == formula `version` == latest changelog heading.
- The manifest MUST NOT contain a `rgt` key.

## II. Formula SHA correctness (SC-002)

- The formula's sha256 for aarch64-apple-darwin, x86_64-unknown-linux-musl, and aarch64-unknown-linux-gnu equal the corresponding hashes in the v0.5.0 `checksums.txt` (verified when the checksums URL is reachable; offline run warns and passes).

## III. No production-code change (SC-004)

- `git diff --stat -- src/` is empty for this feature's commit.

## IV. Changelog completeness (SC-003)

- `CHANGELOG.md`'s 0.5.0 entry mentions `doctor`, `gc`, and `--number-format`.

## V. Determinism (Constitution IX)

- The check script never depends on the network: SHA verification is opt-in via `RGT_CHECKSUMS_URL` and degrades to a warning when unreachable.
