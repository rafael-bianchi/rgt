# Contract: Verification Pipeline & Replace Invariants (Update Integrity)

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-28

Referenced by unit/contract tests for the update path.

## Verification pipeline

Strict order, any failure aborts before the next step and before extraction/replacement:

1. **Download** the platform asset to a temp dir (existing).
2. **Checksum**: find `checksums.txt` in the release assets.
   - Missing → refusal unless `--skip-checksum` (FR-001).
   - Entry missing for the asset → error (never silently skipped).
   - SHA-256 mismatch → refusal (FR-002).
3. **Attestation**: fetch `GET /repos/rafael-bianchi/rgt/attestations/sha256:<digest>` → follow `bundle_url` → snappy-decompress → verify the Sigstore v0.3 bundle.
   - Fetch/decompress/verify failure → refusal unless `--skip-attestation` (FR-003).
   - Verified bundle MUST: chain to the public-good trust root, embed a valid SCT, carry a valid Rekor SET + Merkle inclusion, sign the downloaded file's exact digest, and match the `rafael-bianchi/rgt` workflow policy.
4. **Extract** the archive (existing).
5. **Replace** via `self_replace` (see below).

## Replace invariants (FR-007)

- POSIX: a single atomic rename; the target path always resolves to either the old or the new binary (no absent window). Asserted by a watcher-style unit test (metadata before/after rename of a dummy target).
- Windows: `self_replace`'s documented strategy (move aside + `FILE_FLAG_DELETE_ON_CLOSE` + delete-glue) is used; best-effort, but a failure is reported, not swallowed.
- Replacement runs only after both verifications pass — no rollback is required post-verification (SC-004 holds because install never happens on a failed verification).

## Error handling (FR-008)

- Every failure path returns a non-zero exit and a message naming the failing step and, where applicable, the file/tag.
- No `let _ =`-style swallowing in `src/cli/update.rs` or `src/updater/*`.

## Opt-out flags

- `--skip-checksum` and `--skip-attestation` are the ONLY relaxations; each is printed as insecure in `rgt update --help`.
- Both flags never appear in non-interactive default output; their use is acknowledged in the command output.

## Test fixtures

- A captured real GitHub Actions Sigstore v0.3 bundle (the `bundle_url` payload, snappy-decompressed) is committed under `tests/fixtures/` for `attestation-verify` unit tests — no live network in tests.
- Contract tests drive `execute_update` with a stub release missing `checksums.txt` / missing attestation to assert refusal + exit codes.
