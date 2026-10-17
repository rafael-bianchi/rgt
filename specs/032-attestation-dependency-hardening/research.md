# Research: Attestation Dependency Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `src/updater/attestation.rs`, `tests/unit/test_update_attestation.rs`, `tests/fixtures/rgt/README.md`, `.github/workflows/release.yml`, and live GitHub API verification this session.

## 1. Current state (FR-001)

**Finding**: `Cargo.toml` has `attestation-verify = "0.1.0"` — a **caret** pin (`>=0.1.0, <0.2.0`). The crate is brand-new (repo created 2026-07-29 per the verification report; `cargo search` confirms 0.1.0 is the only version). An incidental `cargo update` could pull a `0.1.x` with altered verification behavior into the updater's trust path without review.

**Decision**: Exact-pin `attestation-verify = "=0.1.0"`. Any future upgrade becomes a deliberate `Cargo.toml` change (Constitution XI's "new dependencies MUST be justified"; this extends to *changed* versions of a security-critical dep). The verification report's "pinned" description was slightly optimistic — this makes it actually exact.

**Alternatives considered**: Leave the caret (a future patch could be a real fix) — rejected: an unreviewed crate carrying the entire signature guarantee should not auto-move; security-fix bumps are cheap and deserve a review.

## 2. Policy correctness (Assumptions)

**Finding**: `src/updater/attestation.rs` pins owner id `19198157` and repo id `1318560598`, signer workflow `.github/workflows/release.yml`, `RefPolicy::Glob("refs/*")`.

**Decision**: No change. Verified against the GitHub API this session: repo id `1318560598`, owner id `19198157`; the release workflow does contain the `attest-build-provenance` step (release.yml:119). The one *potential* gap the README's capture procedure guards against (a real bundle whose `source_ref`/signer path differs) cannot be checked until a real fixture exists — that's exactly what the capture script un-blocks.

## 3. Interim risk acceptance (FR-002)

**Decision**: New `SECURITY.md` documenting: (a) the dependency's zero-adoption, brand-new status and its security-critical role; (b) that `rgt_policy()` has never been exercised against a real signed `rafael-bianchi/rgt` release (self-test self-skips); (c) the recorded risk acceptance — shipping is acceptable because the updater is fail-closed (a verification failure refuses the install) and the trust root is the independent Sigstore public-good log, so a dependency bug would surface as a refusal, not a silent acceptance; (d) the remediation trigger: run `scripts/capture-rgt-attestation-fixture.sh` against the first attested release, then `verifies_real_rgt_bundle_when_fixture_present` runs for real.

**Rationale**: The plan's scope note explicitly names SECURITY.md/CONTRIBUTING.md as the home for this; SECURITY.md is the conventional location for supply-chain risk acceptance, and a new reader finds it immediately.

## 4. Capture script (FR-003, SC-003)

**Decision**: `scripts/capture-rgt-attestation-fixture.sh TAG [ASSET]` automates the fixture README's four steps:
1. Download the released archive for `TAG` (default `rgt-v${TAG#v}-aarch64-apple-darwin.tar.gz`), compute its SHA-256.
2. `gh api "/repos/rafael-bianchi/rgt/attestations/sha256:<hex>" --jq '.attestations[0].bundle_url'`.
3. Download the bundle, `snap`-decompress it (reusing the same algorithm as `decompress_bundle`).
4. Write `tests/fixtures/rgt/rgt-bundle.json` + `artifact.sha256`, with a clear message to re-run the gated test.

Fail-open behavior: a release with no attestation (like today's v0.5.0) exits non-zero with "no attestation found" — never writes a corrupt fixture.

**Rationale**: The plan's option (b) is release-gated; a script makes the gated step a one-command action the moment the release ships, and its non-network parts are testable now (Constitution IX).

## 5. Self-skip message (FR-004)

**Decision**: Update the `eprintln!` in `verifies_real_rgt_bundle_when_fixture_present` to reference `SECURITY.md` and the capture script, keeping it a skip (not a hard failure — a hard failure would break every PR until a real release exists, wrong signal for a P2 task).

## 6. Test strategy (SC-001..SC-004)

**Decision**: A small unit test in `tests/unit/test_update_attestation.rs` (or a new `scripts`-adjacent test) covers the capture logic's deterministic parts: the digest-URL construction and a snappy round-trip (already covered by `decompress_round_trips_snappy_payload`). The "no attestation → clean failure" path is tested by running the script against today's v0.5.0 (no attestations exist — verified). Full `cargo test` stays green.
