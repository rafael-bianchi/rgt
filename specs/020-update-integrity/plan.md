# Implementation Plan: Update Integrity

**Branch**: `020-update-integrity` | **Date**: 2026-08-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/020-update-integrity/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Make `rgt update` verify what it installs, never silently downgrade, and never leave the binary missing (findings §7.1). Concretely:

1. **Checksum hard-error (CRITICAL)**: a release missing `checksums.txt` is refused with a clear error and non-zero exit — the current silent `if let Some(...)` fall-through (`src/cli/update.rs:82-94`) becomes a hard error; the explicit `--skip-checksum` flag is the only relaxation.
2. **Independent trust anchor (HIGH)**: verify the release's GitHub Artifact Attestation **in-binary** before install (user-confirmed 2026-08-28). Missing/unverifiable attestation → refused unless `--skip-attestation`. Research (see `research.md`) found the canonical `sigstore` crate is disproportionately heavy (366 crates, tokio/reqwest/aws-lc-rs, not pure Rust) and **does not verify the Rekor SET/inclusion proof**; the purpose-built pure-Rust `attestation-verify` crate is selected instead — same in-binary, self-contained, no-`gh` intent the user confirmed. **Flagged for review**: if the user prefers the literal `sigstore` crate or a hand-rolled verifier, swap the dependency before `/speckit-tasks`.
3. **Semver ordering (HIGH)**: version comparison switches from string `==`/`!=` (`update.rs:21,33`) to `semver` precedence; `rgt update`/`--check` proceed only when published `latest > current`. Explicit `--version <tag>` installs only after verification, with a warning when older; invalid tags → exit 2.
4. **Atomic replace + rollback (MEDIUM)**: replace `AtomicReplace`'s two-step `target → .old → target` rename (`src/updater/atomic.rs:20-43`) with the `self_replace` crate: POSIX single-atomic-rename (no absent-target window), Windows self-replace handled correctly; verification happens before replacement so no rollback is ever needed post-verification.
5. **Hygiene (LOW)**: no `let _ =`-swallowed errors in the update path (FR-008); the §7.1 TOCTOU between checksum-verify and extract is accepted as low-risk (requires local code-exec).

## Technical Context

**Language/Version**: Rust (edition 2021, stable). NOTE: the chosen verifier `attestation-verify` requires MSRV 1.88 (checked in research).

**Primary Dependencies** (all pure Rust, all verified): `semver` (1, MIT OR Apache-2.0) — version precedence; `snap` (BurntSushi, MIT OR Apache-2.0) — snappy-decompress the GitHub `bundle_url` body; `attestation-verify` (0.1, MIT OR Apache-2.0, sans-io) — in-binary Sigstore bundle verification (DSSE signature, Fulcio chain to public-good root, embedded SCT, Rekor SET + Merkle inclusion, GitHub workflow policy); `self_replace` (1.5, Apache-2.0) — cross-platform atomic binary replacement. Existing `ureq`/`serde_json` reused for the attestation fetch.

**Storage**: N/A — no schema/store changes.

**Testing**: `cargo test` — unit tests for semver gating, attestation-verify against a real captured GitHub bundle fixture (research identified a real v0.3 bundle shape), `self_replace` behavior, and `--skip-*` opt-outs; contract tests for `execute_update` refusal paths (missing checksum / missing or failing attestation → error + non-zero); `cargo fmt`/`clippy` per constitution XI.

**Target Platform**: macOS, Linux, Windows (`self_replace` covers the Windows running-`.exe` case; POSIX atomicity required by FR-007).

**Project Type**: CLI tool (Rust single binary) with a self-update subcommand.

**Performance Goals**: update path unchanged order-of-magnitude (network-bound); verification adds ms-scale CPU (RSA/ECDSA + SHA-256) plus one extra HTTP fetch (attestation).

**Constraints**:
- FR-001/FR-003: missing checksums.txt or missing/unverifiable attestation → hard refusal + non-zero exit; relaxations only via explicit `--skip-checksum` / `--skip-attestation` (documented insecure).
- FR-002: checksum mismatch always aborts before install (existing, preserved).
- FR-004/FR-006: semver precedence for auto-update and `--check`; never install an older tag by default.
- FR-005: `--version` installs only after full verification; warns on older tag; invalid tag → exit 2.
- FR-007: POSIX replace is a single atomic rename (no absent-target window); `self_replace` on Windows is best-effort but no longer silent.
- FR-008: every update error names the failing step/file; non-zero exit; no swallowed errors.
- Attestation verification is **in-binary** (no `gh` subprocess); the fetch uses the existing ureq agent with optional `GITHUB_TOKEN` for rate limits.
- **Testability seam**: `execute_update` accepts an injectable release-fetcher (closure/trait defaulting to `github::fetch_*`) so contract tests run against stub releases without live network (Constitution IX determinism).

**Scale/Scope**: rework `src/cli/update.rs` (semver gating, checksum hard-error, attestation fetch+verify, `--skip-*` flags), `src/updater/atomic.rs` (delegate to `self_replace`), new `src/updater/attestation.rs` (fetch + decompress + verify bundle), a release-CI attestation step (`.github/workflows/release.yml`), 4 new deps, tests, README/`--help` updates. No changes outside the update path.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | All four new crates (`semver`, `snap`, `attestation-verify`, `self_replace`) are pure Rust with no runtime engines; verification is in-binary. (Precedent: rusqlite already bundles C SQLite statically.) |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | Unaffected. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | Distribution channels unchanged; `rgt update` integrity improved. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | semver/snap/attestation-verify: MIT OR Apache-2.0; self_replace: Apache-2.0 — all compatible with RGT's Apache-2.0. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Unaffected. |
| IX. Test-Driven Verification | ✅ PASS | Test-first tasks: refusal paths, semver gating, attestation verification against a real captured bundle fixture, replace behavior. |
| X. Documentation Matches Behavior | ✅ PASS | README/`--help` updated for `--skip-checksum`/`--skip-attestation` and verification behavior in the same changeset. |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | New deps justified in `research.md` (sigstore crate rejected: 366 crates, not pure Rust, skips SET/inclusion); `attestation-verify` MSRV 1.88 noted as a constraint; fmt/clippy enforced. |

**Gate Result**: PASS — 11/11 principles satisfied; no amendments required.

## Project Structure

### Documentation (this feature)

```text
specs/020-update-integrity/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (attestation verification stack)
├── data-model.md        # Phase 1 output (update domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (update CLI + verification invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── cli/
│   └── update.rs        # MODIFIED: semver gating; checksum hard-error; attestation
│                        #           fetch+verify; --skip-checksum/--skip-attestation flags
└── updater/
    ├── atomic.rs        # MODIFIED: delegate to self_replace (POSIX atomic, Windows handled)
    ├── attestation.rs   # NEW: fetch bundle via /attestations/sha256:<digest>, snap-decompress,
    │                    #      verify via attestation-verify against the public-good trust root
    └── github.rs        # MODIFIED (minor): expose release digest helper + attestation fetch URL

.github/
└── workflows/
    └── release.yml      # MODIFIED: attest-build-provenance step on release artifacts

tests/
├── unit/test_update_semver.rs       # NEW: semver gating + --version behavior
├── unit/test_update_attestation.rs  # NEW: bundle verification vs captured fixture
└── contract/test_update_contract.rs # MODIFIED: refusal paths + --skip-* opt-outs
```

**Structure Decision**: Single-project layout (existing Rust CLI). A dedicated `src/updater/attestation.rs` owns the fetch/decompress/verify pipeline so the verification logic is unit-testable in isolation with a captured bundle fixture (no live network in tests), mirroring the existing `checksum.rs`/`github.rs` module split.

## Complexity Tracking

None — no constitution violations; the four new dependencies are justified under Principle XI in the Constitution Check above (see `research.md` for the rejected alternatives, incl. the `sigstore` crate and the hand-rolled verifier).
