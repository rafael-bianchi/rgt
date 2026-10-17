# Feature Specification: Update Contract Test Fixture Fix

**Feature Branch**: `feat/030-update-contract-test-fixture-fix`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Fix the two currently-failing tests in `tests/contract/test_update_contract.rs` (`genuine_newer_tag_proceeds_past_gating`, `failed_verification_never_reaches_replace`) — stale fixtures hardcode stub version `v0.4.1` as 'newer,' which stopped being true once `Cargo.toml` was bumped to `0.5.0`. Priority P0: `develop`'s test suite is red right now."

Source context: `docs/verification-of-fixes-develop.md` §4 "New issues found" (2 real test failures) and §7 scorecard. Plan: `docs/spec-plan-followups-from-verification.md`, spec #1, P0.

## Clarifications

### Session 2026-08-31

- Q: Fix the fixture by hardcoding a newer stub version, or derive it from `CARGO_PKG_VERSION` at test time so this never recurs on the next version bump? → A: Derive the "newer" stub version from the compiled `CARGO_PKG_VERSION` at test time (bump the minor component). This is a P0 unblock with a durability requirement: a hardcoded constant would simply move the failure to the next version bump, which is exactly the class of silent staleness this backlog exists to eliminate.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Test Suite Is Green Again (Priority: P1)

A contributor working on `develop` (currently at version 0.5.0) runs the update contract test suite and sees two failures: `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace`. Both fail because the stubbed "newer" release tag (`v0.4.1`) is no longer newer than the actual installed version — the semver gate now correctly reports "already at the latest version" and the tests never reach the assertions they were written to check. The contributor's mental model is "the anti-downgrade gate is working; the test fixture just went stale." The fix should make these two tests pass deterministically against any current version without hardcoding a "future" version number.

**Why this priority**: P0 per the handoff — a red test suite makes every other change on this branch untrustworthy; nothing else can be accepted as "passing" until this lands.

**Independent Test**: `cargo test --test test_update_contract` passes with zero failures on `develop` at version 0.5.0.

**Acceptance Scenarios**:

1. **Given** the `develop` branch at `CARGO_PKG_VERSION` 0.5.0, **When** `cargo test --test test_update_contract` runs, **Then** all 13 tests pass including `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace`.
2. **Given** the same test file, **When** a future maintainer bumps `CARGO_PKG_VERSION` again (e.g. to 0.6.0), **Then** these two tests still pass without further edits, because the "newer" stub version is derived from the compiled version constant rather than hardcoded.
3. **Given** the fixed fixture, **When** the tests exercise the auto-update path (`version = None`), **Then** the stub tag is still recognized as genuinely newer so the pipeline proceeds past gating to the download/verify stage exactly as the original tests intended.

---

### Edge Cases

- What if the current version is a pre-release (e.g. `0.5.0-rc.1`)? Deriving a "newer" tag by bumping the minor component of the *numeric* part must still yield a strictly-greater semver. If the derivation helper parses only the release digits and bumps minor, `0.5.0-rc.1` → `0.6.0` is still strictly newer (semver precedence: a pre-release is always less than the same core with no pre-release, and the bumped core dominates anyway).
- What if `CARGO_PKG_VERSION` fails to parse as a semver? The test helper should fail loudly with a clear message (panic with context) rather than silently producing a wrong version and re-introducing a false pass.
- Do any *other* tests in the file rely on the old `v0.4.1` stub meaning "newer"? `older_or_equal_tag_is_up_to_date_not_an_update`, `skip_checksum_bypasses_the_checksum_refusal`, `attestation_failure_is_a_hard_refusal_unless_skipped`, and `checksum_mismatch_aborts_before_install` use `v0.4.1` (or `v0.4.0`/`v0.3.9`) as an *older or explicit-target* version where "older" semantics is the point — those must keep working unchanged, and only the two genuinely-needs-newer tests should switch to the derived helper.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The two tests `genuine_newer_tag_proceeds_past_gating` and `failed_verification_never_reaches_replace` MUST pass when `CARGO_PKG_VERSION` is `0.5.0`, without changing the production code in `src/cli/update.rs`.
- **FR-002**: The test fixture MUST derive a "genuinely newer than current" release tag from `CARGO_PKG_VERSION` at test time (e.g. bumping the minor component of the parsed release version), instead of hardcoding a literal future version.
- **FR-003**: The derived tag MUST be strictly newer than `CARGO_PKG_VERSION` under semver precedence for all current and foreseeable release version shapes (release and pre-release), so the tests continue to pass on future version bumps without further edits.
- **FR-004**: Tests that intentionally use an older or explicit-target stub version (`older_or_equal_tag_is_up_to_date_not_an_update`, `skip_checksum_bypasses_the_checksum_refusal`, `attestation_failure_is_a_hard_refusal_unless_skipped`, `checksum_mismatch_aborts_before_install`, `explicit_older_version_proceeds_with_warning_not_refusal`) MUST keep their existing semantics and MUST NOT be forced through the new derivation helper.

### Key Entities *(include if feature involves data)*

- **NewerStubVersion**: A helper (function or macro) in the test module that parses `CARGO_PKG_VERSION` and returns a semver-strictly-greater tag string, used only by the two "proceeds past gating" tests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo test --test test_update_contract` reports all 13 tests passing on `develop` at version 0.5.0 (contract-tested; the suite itself is the verification).
- **SC-002**: No production code in `src/` changes as a result of this fix (test-fixture-only change).
- **SC-003**: A future simulated version bump (unit test that runs the derivation helper against multiple synthetic current versions, including a pre-release) produces a strictly-newer tag in every case (unit-tested).

## Assumptions

- The current version (0.5.0) is a plain release; the fix still guards pre-release shapes for future-proofing.
- Only the two "proceeds past gating" tests need the derived newer tag; all other tests in the file retain their explicit stub versions.
- The fix is confined to `tests/contract/test_update_contract.rs` (plus, if needed, a tiny helper within that same test module — no new shared test-utility file required for a two-call-site need).

## Out of scope

- Any change to update/verification production logic (`src/cli/update.rs`, `src/updater/`).
- The semver-gating behavior itself (`tests/unit/test_update_semver.rs` is already green and correct).
- The other new-issue follow-ups from the verification report (spec 031 path-quoting, spec 032 attestation-dependency hardening) — separate specs.
