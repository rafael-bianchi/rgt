# Quickstart: Validating Update Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-28

Runnable guide to prove `rgt update` integrity end-to-end. Invariants in [contracts/update-behavior.md](./contracts/update-behavior.md) and [contracts/verification-invariants.md](./contracts/verification-invariants.md); entities in [data-model.md](./data-model.md).

## Prerequisites

- Rust stable toolchain (MSRV 1.88 for `attestation-verify`).
- Build in debug: `cargo build`.
- Network access to api.github.com for manual scenarios 2–4.

## Automated validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```

Expected:
- `test_update_semver` (new): `1.10.0` vs latest `v1.9.9` → "up to date" (no downgrade); `v1.10.1` → update proceeds; invalid tag → exit 2; `--version` older-tag warning path.
- `test_update_attestation` (new): the captured bundle fixture verifies against the embedded public-good trust root with the expected digest + policy; a tampered digest fails.
- `test_update_contract` (extended): stub release missing `checksums.txt` → refusal + exit 1 (unless `--skip-checksum`); missing/unverifiable attestation → refusal + exit 1 (unless `--skip-attestation`); checksum mismatch → abort.
- `test_update_replace` (new): POSIX replace is a single atomic rename with no absent-target window.

## Manual scenario 1 — version-gate behavior (no network)

```bash
./target/debug/rgt update --check
```
Expected: compares the installed version against the published latest using semver precedence; "up to date" when not strictly greater; no download.

## Manual scenario 2 — a release missing `checksums.txt` is refused

Emulate by pointing `--version` at a tag whose release has no checksums (or by a stub in tests):
```bash
./target/debug/rgt update --version v0.1.0 --yes
```
Expected: `Error: release has no checksums.txt asset — refusing to install an unverified binary (use --skip-checksum to override)`, exit 1, nothing installed. Re-run with `--skip-checksum` → proceeds (insecure override).

## Manual scenario 3 — attestation missing/unverifiable is refused

Against a release with a checksum but no (or invalid) attestation, or with `GITHUB_TOKEN`/network blocked during verification:
```bash
./target/debug/rgt update --yes
```
Expected: refusal naming the attestation step, exit 1, nothing installed. `--skip-attestation` → proceeds (insecure override).

## Manual scenario 4 — genuine update still works

```bash
./target/debug/rgt update
```
Expected: download → checksum verified → attestation verified → replace → `Successfully updated to <tag>`; the running binary path always resolves to old-or-new (no missing window).

## Definition of done (link to contracts)

- All automated tests pass (this page, above).
- Manual scenarios 1–4 produce the stated outcomes.
- Every invariant in `contracts/update-behavior.md` and `contracts/verification-invariants.md` has at least one automated test.
