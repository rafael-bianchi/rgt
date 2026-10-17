# Implementation Plan: Attestation Dependency Hardening

**Branch**: `feat/032-attestation-dependency-hardening` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/032-attestation-dependency-hardening/spec.md`

## Summary

Harden the update-trust path against the immaturity of its signature-verification dependency: exact-pin `attestation-verify` (`=0.1.0`), document the interim risk acceptance and remediation trigger in a new `SECURITY.md`, and add an automated fixture-capture script so un-skipping the gated self-test is a one-command step the moment a real attested release ships. No behavior change to the updater or verification policy.

## Technical Context

**Language/Version**: N/A — `Cargo.toml` pin + Markdown doc + a bash capture script.

**Primary Dependencies**: `attestation-verify` (exact-pinned to 0.1.0), `snap` (existing, reused by the script's decomposition).

**Storage**: N/A.

**Testing**: A unit test for the capture script's non-network decomposition (digest computation, URL construction, snappy round-trip); the script's "no attestation" failure is tested by pointing it at today's v0.5.0 (which has none).

**Target Platform**: All (CI/dev tooling).

**Project Type**: CLI + security docs/tooling.

**Performance Goals**: N/A.

**Constraints**: Constitution IX (the script's testable parts are covered; the network path is a dev/ops tool, not a test dependency). No updater behavior change.

**Scale/Scope**: `Cargo.toml`, new `SECURITY.md`, new `scripts/capture-rgt-attestation-fixture.sh`, a unit test, and a small self-skip message update.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Pin change + docs/tooling only. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | The update trust path is hardened against silent drift. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | Documents the supply-chain risk acceptance; no license change. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | Capture script decomposition unit-tested; full suite green. |
| X. Documentation matches behavior | PASS | SECURITY.md documents exactly what is and isn't verified. |
| XI. Idiomatic Rust, exit codes | PASS | No logic change; script fails cleanly with a clear message. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/032-attestation-dependency-hardening/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── attestation-hardening-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
Cargo.toml                                      # attestation-verify = "=0.1.0"
SECURITY.md                                     # NEW: risk acceptance + remediation trigger
scripts/capture-rgt-attestation-fixture.sh      # NEW: one-command fixture capture
src/updater/attestation.rs                       # (none — policy verified correct)
tests/unit/test_update_attestation.rs            # self-skip message updated to reference SECURITY.md
```

**Structure Decision**: Minimal, tooling/doc-heavy. The capture script mirrors `tests/fixtures/rgt/README.md`'s documented procedure, reusing the existing `snap` decompress path.

## Complexity Tracking

> None — no constitution violations requiring justification.
