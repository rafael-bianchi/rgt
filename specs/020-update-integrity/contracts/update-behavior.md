# Contract: `rgt update` CLI Behavior (Update Integrity)

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-28

## Invariants

1. `rgt update` MUST NOT install unless BOTH the SHA-256 checksum and the GitHub Artifact Attestation verify (or the corresponding explicit `--skip-*` flag is passed).
2. A release missing `checksums.txt` is refused; a checksum mismatch is refused; a missing/unverifiable attestation is refused — each with a clear error and a non-zero exit.
3. Auto-update (no `--version`) proceeds only when the published version is semver-strictly-greater than the installed version; equal/older is "up to date" with no download.
4. `--version <tag>` installs only after the same verification, warns when the tag is older than installed, and rejects invalid/nonexistent tags.
5. All update errors name the failing step and, where applicable, the file; nothing is silently swallowed.

## CLI surface

```text
rgt update [--check] [--yes] [--version <tag>] [--skip-checksum] [--skip-attestation]
```

- `--skip-checksum`: relax FR-001/FR-002 (insecure — documented). Never the default.
- `--skip-attestation`: relax FR-003 (insecure — documented). Never the default.

## Exit codes

| Code | Meaning | When |
|---|---|---|
| 0 | Success | Update applied, or `--check`/up-to-date path, or explicit cancel. |
| 1 | Expected error | Any verification refusal (missing checksum/attestation, mismatch, network failure during verification), replace failure, or other expected update error. |
| 2 | Invalid input | Invalid `--version` tag (nonexistent release or unparseable semver). |

## Output shape

- Refusal (no `--skip-*`): `Error: release has no checksums.txt asset — refusing to install an unverified binary (use --skip-checksum to override)` / `Error: attestation missing or failed verification — refusing to install (use --skip-attestation to override)`, then exit 1.
- Checksum mismatch: `Error: checksum verification FAILED. Aborting update.` (existing), exit 1.
- Up-to-date: `rgt is up to date (v<current>)`, exit 0.
- Older-tag warning (explicit `--version`): printed before the install confirmation.
- Success: `Successfully updated to <tag>` (existing), exit 0.

## Acceptance linkage

- US1/FR-001/FR-003 → invariants 1, 2 (refusal paths).
- US2/FR-004/FR-005/FR-006 → invariants 3, 4 (semver gating + `--version`).
- US3/FR-007/FR-008 → invariant 5 + atomic replace (see `verification-invariants.md`).
