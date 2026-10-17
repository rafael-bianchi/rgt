# Data Model: Attestation Dependency Hardening

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

## 1. DependencyPin

`attestation-verify` exact-pinned to `=0.1.0` — any upgrade is a deliberate `Cargo.toml` change.

## 2. RiskAcceptance (SECURITY.md)

The recorded decision to ship with an unreviewed, security-critical dependency until a real fixture closes the loop. Fields: dependency maturity, untested-in-production policy, the fail-closed rationale, the remediation trigger.

## 3. AttestationFixture

The `tests/fixtures/rgt/{rgt-bundle.json, artifact.sha256}` pair produced by the capture script and consumed by the gated self-test. Absent today (no attested release exists); the script makes capture one command when one ships.

## 4. CaptureScript

`scripts/capture-rgt-attestation-fixture.sh` — digest → attestations API → bundle download → snap decompress → fixture write; fails cleanly (no corruption) on a no-attestation release.
