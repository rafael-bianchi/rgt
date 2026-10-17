# Research: Update Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-28

Resolves the plan's Technical Context unknowns via fresh crate/API research (docs.rs, crates.io, sigstore.dev, docs.github.com, live API probes). Consolidates findings §7.1's citations.

## 1. In-binary attestation verification

**Decision**: Add `attestation-verify` (0.1, MIT OR Apache-2.0, MSRV 1.88) as the in-binary Sigstore bundle verifier, composed with the existing `ureq` agent for fetching and the `snap` crate for decompression. **This honors the user-confirmed "in-binary, self-contained, no `gh`" decision (2026-08-28) but replaces the literally-named `sigstore` crate** — see "Alternatives considered" for why.

**Rationale**:
- `attestation-verify` is purpose-built for exactly this task ("Verify GitHub Artifact Attestations (Sigstore bundles) offline — minimal, sans-io, fail-closed"): ~75 unique crates, **all pure Rust**, no HTTP (composes with our ureq), no tokio/reqwest/aws-lc-rs.
- It verifies **more** than the canonical crate: DSSE signature, **Rekor SET + Merkle inclusion + checkpoint**, Fulcio certificate chain to the embedded public-good trust root, embedded SCT, and a `GithubPolicy` (source repo, ref, signer workflow, commit). The `sigstore` crate itself has an open TODO (sigstore-rs#285) NOT verifying the SET/inclusion proof.
- Verification is offline from the bundle (no Rekor query needed), which keeps the update path network-light and deterministic.
- License MIT OR Apache-2.0 — compatible.

**Alternatives considered**:
- **`sigstore` crate (0.14, Apache-2.0)** — the literally-confirmed option. Verified: NOT pure Rust (mandatory `aws-lc-rs`, a BoringSSL fork with C/ASM), hard `tokio`/`reqwest`/`hyper` deps, ~366 unique crates in the minimal verify configuration; and it does not verify the Rekor SET/inclusion proof. Disproportionate for a ureq-based CLI and weaker than `attestation-verify`. (Constitution I precedent — bundled SQLite in rusqlite — makes it *permissible*, but the size/crypto-coverage tradeoff favors `attestation-verify`.)
- **prefix-dev `sigstore-verify` (0.11, BSD-3-Clause)** — clean API but still aws-lc-rs + tokio + reqwest; BSD-3-Clause is compatible but the dep tree is equally heavy.
- **Hand-rolled verifier** (`p256`/`ecdsa` + `x509-cert` + `rustls-webpki` + `sha2`/`base64`, embedded Fulcio root/intermediate) — pure Rust, ~10 crates, and `rustls-webpki` is already in the tree. Rejected as primary: owning security-critical crypto (DSSE PAE, chain building, SCT, Rekor) is high-risk to hand-roll; `attestation-verify` implements the identical algorithm, is tested, and pins it in one reviewed dependency. Kept as fallback if `attestation-verify`'s MSRV 1.88 or audit status is unacceptable.
- **`gh attestation verify` subprocess** — rejected per user decision (no `gh` dependency).

## 2. Fetching the attestation at update time

**Decision**: `GET https://api.github.com/repos/{owner}/{repo}/attestations/sha256:<hex-digest>` (works unauthenticated on public repos; optional `GITHUB_TOKEN` for the 60/hr unauth rate limit), take `attestations[0].bundle_url`, GET it, **snappy-decompress** (the bundle_url body is snappy-compressed Sigstore bundle JSON), and parse as a Sigstore v0.3 bundle.

**Rationale**: Verified empirically — the older `actions/artifact-attestations` endpoints return HTTP 404; the current endpoint is `docs.github.com/en/rest/repos/attestations`. `gh`'s own client follows `bundle_url` and snappy-decodes; the bundle payload is a v0.3 bundle (`mediaType`, `verificationMaterial.{certificate,tlogEntries}`, `dsseEnvelope`) — exactly the shape `attestation-verify` consumes. Requires the `snap` crate for decompression (pure Rust, BurntSushi).

**Alternatives considered**: fetching inline `bundle` (null in practice — must follow `bundle_url`); `predicate_type=provenance` filter to reduce noise.

## 3. Semantic version ordering

**Decision**: Add `semver` (1, MIT OR Apache-2.0). Parse both `CARGO_PKG_VERSION` and the release `tag_name` (strip leading `v`) with `Version::parse`; auto-update proceeds only when `latest > current` via `Ord` (SemVer 2.0 precedence, incl. pre-release ordering). `--check` uses the same comparison.

**Rationale**: The findings doc verified `semver` as "A parser and evaluator for Cargo's flavor of Semantic Versioning" with real comparison operators; `docs.rs/semver` confirms `1.0.0-alpha.1 < 1.0.0` and `cmp_precedence()` ignores build metadata. This replaces the string `==`/`!=` at `update.rs:21,33` and GitHub's time-sorted `/releases/latest` as the decision source for "is this newer?".

**Alternatives considered**: the complementary fix (list all releases, parse every tag, local max) — documented in findings §7.1; rejected for scope (needs a new API call + pagination); the `/releases/latest` endpoint + semver comparison resolves the downgrade bug for the default path.

## 4. Atomic binary replacement

**Decision**: Add `self_replace` (1.5, Apache-2.0, mitsuhiko). Replace `AtomicReplace::replace` (`src/updater/atomic.rs:20-43`) — today `target → target.old`, then `tmp → target`, with errors swallowed via `let _ =`.

**Rationale**: `self_replace::self_replace(path)` on Unix places the new file next to the exe and **atomically renames** (the running process keeps the old inode — no absent-target window, satisfying FR-007 by construction); on Windows it handles the running-`.exe` restriction (move aside, `FILE_FLAG_DELETE_ON_CLOSE`, spawn delete-glue), which hand-rolling cannot do portably. Verification (checksum + attestation) runs **before** replacement, so no rollback step is ever needed post-verification — addressing the MEDIUM finding's "no rollback on failure" concern by removing the failure window. Small (~283 lines), stable since 2024, 11.9M downloads.

**Alternatives considered**: hand-rolled POSIX `rename(tmp, target)` — equivalent on POSIX but leaves Windows broken (the existing limitation the findings flagged as needing a live test); keeping the two-step rename + explicit rollback — rejected, keeps a no-binary window on crash.

## 5. Hygiene (LOW findings)

**Decision**: Remove all `let _ =` error-swallowing in the update path (FR-008) — report cleanup failures by name. The §7.1 TOCTOU between checksum verification and archive extraction is accepted as low-risk (exploiting it requires local code-execution equivalent to already having the target's privileges) and is not engineered against.

**Rationale**: FR-009-style user-recoverable errors; the TOCTOU is explicitly ranked LOW in the findings and would add complexity disproportionate to the threat.
