# Feature Specification: Attestation Dependency Hardening

**Feature Branch**: `feat/032-attestation-dependency-hardening`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "`attestation-verify` v0.1.0 (spec 020's new signature-verification dependency) is a brand-new, seemingly zero-adoption third-party crate carrying the entire trust guarantee for `rgt update`'s signature check; its production verification policy (`rgt_policy()`) has never been exercised against a real signed `rafael-bianchi/rgt` release — the relevant self-test explicitly self-skips pending a fixture that doesn't exist yet."

Source context: `docs/verification-of-fixes-develop.md` §4 "Maturity concern" + §7. Plan: `docs/spec-plan-followups-from-verification.md`, spec #3, P2.

## Clarifications

### Session 2026-08-31

- Q: The plan's scope note says 032 "may be no code change available yet" — the missing fixture is gated on a real attested release, which is a release-process dependency, not fixable in this repo alone. What should the spec actually deliver? → A: The genuinely-available hardening now: (a) exact-pin `attestation-verify` (it is currently a caret `"0.1.0"` — a future 0.1.x could silently change security behavior of an unreviewed crate; an exact `=0.1.0` pin stops that drift); (b) document the interim risk acceptance in a new `SECURITY.md` (option c in the plan); (c) provide an automated fixture-capture script so the moment the first attested release ships, capturing the fixture and un-skipping the self-test is one command (directly serving option b's readiness without being gated on the release itself). The policy constants themselves are verified correct against the live GitHub API this session (owner id 19198157, repo id 1318560598) — no change needed there.
- Q: Should the self-skip test's silent `return` become a hard failure to force attention? → A: No — keep it a skip, but make it explicit and documented. A hard failure would break CI on every PR until a release ships, which is exactly the wrong signal for a P2 hardening task. The skip already `eprintln!`s an actionable message.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Update Trust Guarantee Is Protected From Silent Dependency Drift (Priority: P2)

A maintainer (or `cargo update`) looks at `Cargo.toml` and sees `attestation-verify = "0.1.0"` — a caret pin that permits any `0.1.x`. Because this crate is brand-new (repo created 2026-07-29, zero adoption) and carries the entire signature-verification guarantee for `rgt update`, a future `0.1.1`/`0.1.2` could alter verification behavior without a deliberate review. After this fix, the dependency is exact-pinned (`=0.1.0`), so any upgrade is an explicit, reviewed `Cargo.toml` change — never an incidental `cargo update` side effect. The interim risk (an unreviewed crate, an untested-against-reality policy) is documented in `SECURITY.md` so the decision to accept it is a recorded, informed one.

**Why this priority**: P2 per the plan — not a known bug (the implementation is architecturally sound per the verification report), but an unaudited dependency carrying a security-critical function, untested in the one scenario that matters most.

**Independent Test**: `Cargo.toml` pins `attestation-verify` exactly (`=0.1.0`); `SECURITY.md` documents the interim risk and the exact un-skip trigger; `scripts/capture-rgt-attestation-fixture.sh` automates the fixture capture from a real release.

**Acceptance Scenarios**:

1. **Given** `Cargo.toml`, **When** the `attestation-verify` line is read, **Then** it is `attestation-verify = "=0.1.0"` (exact pin, no caret).
2. **Given** `SECURITY.md`, **When** a security-conscious user reads it, **Then** it documents the dependency's maturity, the untested-in-production policy, the interim risk acceptance, and the exact trigger (first attested release → run the capture script → un-skip the self-test) that closes the gap.
3. **Given** a hypothetical real attested `rafael-bianchi/rgt` release, **When** the capture script runs with its tag, **Then** it produces `tests/fixtures/rgt/rgt-bundle.json` + `artifact.sha256` exactly as `tests/fixtures/rgt/README.md` describes (verified by running the script's steps against a live release — or, until one exists, by unit-testing its decomposition: digest computation and bundle-fetch URL construction).
4. **Given** the existing self-skip test, **When** it runs with no fixture present, **Then** it still skips with an actionable message (unchanged), and its skip message references the new `SECURITY.md` capture path.

---

### Edge Cases

- A future `0.1.x` of `attestation-verify` with a real security fix: the exact pin forces a deliberate, reviewed bump (the desired behavior) — `SECURITY.md` notes this is intentional.
- The capture script running against a release with no attestation (like today's v0.5.0): it must fail with a clear "no attestation exists yet" message, not produce a corrupt fixture.
- The capture script's `snap` decompression: reuse the existing `decompress_bundle` logic (already tested) rather than reimplementing.
- Offline environments: the script is a dev/ops tool, not part of `cargo test`; it needs network only when run.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `Cargo.toml` MUST pin `attestation-verify` to the exact version `=0.1.0` (removing the caret), so any upgrade is a deliberate, reviewed change.
- **FR-002**: A `SECURITY.md` MUST document: the `attestation-verify` dependency's immaturity and zero-adoption status; that `rgt_policy()` has never been exercised against a real signed `rafael-bianchi/rgt` release; the interim risk acceptance (the guarantee rests on an unreviewed 0.1.0 crate until a real attested release is captured as a fixture); and the exact remediation trigger (run `scripts/capture-rgt-attestation-fixture.sh` against the first attested release, then the self-skip test runs for real).
- **FR-003**: A `scripts/capture-rgt-attestation-fixture.sh` MUST automate the procedure in `tests/fixtures/rgt/README.md` (digest → GitHub attestations API → bundle download → `snap` decompress → write `rgt-bundle.json` + `artifact.sha256`), failing cleanly when a release has no attestation.
- **FR-004**: The existing `verifies_real_rgt_bundle_when_fixture_present` self-skip MUST remain a skip (not a hard failure) with an actionable message that references the capture path.

### Key Entities *(include if feature involves data)*

- **AttestationFixture**: the `tests/fixtures/rgt/{rgt-bundle.json, artifact.sha256}` pair the capture script produces and the gated test consumes.
- **RiskAcceptance**: the documented, recorded decision to ship with an unreviewed security-critical dependency until a real fixture closes the loop.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `Cargo.toml` shows `attestation-verify = "=0.1.0"` (grep-verified).
- **SC-002**: `SECURITY.md` exists and mentions `attestation-verify`, the untested policy, the risk acceptance, and the capture script (grep-verified).
- **SC-003**: The capture script exists, is executable, and its non-network decomposition (digest computation, URL construction, decompress round-trip) is covered by a unit test; it fails cleanly (non-zero, clear message) when a release has no attestation (tested by pointing it at today's v0.5.0, which has none).
- **SC-004**: Full `cargo test` stays green (the self-skip unchanged; the only source change is the Cargo.toml pin).

## Assumptions

- No real attested `rafael-bianchi/rgt` release exists yet (verified against the GitHub API this session: zero attestations for v0.5.0 assets) — so un-skipping the fixture test is genuinely out of reach and remains gated, exactly as the plan anticipated.
- `attestation-verify` is currently the only version on crates.io (0.1.0); the exact pin is future-proofing, not a downgrade.
- `SECURITY.md` is the right home (the plan suggests "CONTRIBUTING.md or SECURITY.md"); SECURITY.md is the conventional location for supply-chain risk acceptance.
- The policy constants (owner/repo ids, workflow path) are correct and unchanged (verified against the GitHub API this session).

## Out of scope

- Un-skipping `verifies_real_rgt_bundle_when_fixture_present` (impossible until a real attested release exists; the capture script + README make it a one-command step when one ships).
- Replacing `attestation-verify` with a different crate or hand-rolled Sigstore verification (a much larger decision the plan doesn't call for).
- Changing the verification policy or the updater's behavior.
