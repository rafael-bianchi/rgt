# Data Model: Update Integrity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-28

Domain model for the `rgt update` path. Existing `GitHubRelease`/`GitHubReleaseAsset`/`Platform`/`ChecksumEntry` entities are unchanged; the entities below are new or carry this feature's guarantees.

## Entities

### SemverVersion

The parsed semantic version used for ordering decisions.

| Attribute | Type | Notes |
|---|---|---|
| `raw` | `String` | Source string (from `CARGO_PKG_VERSION` or release `tag_name`, leading `v` stripped). |
| `parsed` | `semver::Version` | Parse result; invalid input is a clear error (invalid input → exit 2). |

Rules:
- Auto-update proceeds only when `latest.parsed > current.parsed` (SemVer 2.0 precedence, `Ord`).
- `--check` reports "up to date" when `latest <= current`.
- Pre-release ordering applies (`1.0.0-alpha.1 < 1.0.0`).

### UpdateDecision

The gating decision for an update attempt.

| Attribute | Type | Notes |
|---|---|---|
| `kind` | `DecisionKind` | `Update` (proceed) or `UpToDate` (no install) or `InvalidInput` (exit 2). |
| `requested_tag` | `Option<String>` | Non-`None` only for explicit `--version`. |

Transitions (no `--version`): fetch latest → semver-compare → `Update` iff `latest > current`; else `UpToDate`.
Transitions (`--version`): validate tag (exists + valid semver) → `Update` (after verification), with an "older than installed" warning when `requested < current`.

### ChecksumsFile

The release's `checksums.txt` asset (existing entity). New invariant: its **absence** is a hard refusal unless `--skip-checksum` (FR-001); a missing entry for the selected asset is likewise an error (never silently skipped).

### AttestationBundle

The GitHub Artifact Attestation for the downloaded artifact, fetched and verified before install.

| Attribute | Type | Notes |
|---|---|---|
| `subject_digest` | `String` | `sha256:<hex>` of the downloaded archive (the digest the attestation must cover). |
| `bundle_json` | `String` | Snappy-decompressed Sigstore v0.3 bundle from `attestations[0].bundle_url`. |
| `policy` | `GithubPolicy` | Source repo (`rafael-bianchi/rgt`), workflow, ref, commit — what the bundle must prove. |

Lifecycle:
1. `Fetched` — `GET /repos/{owner}/{repo}/attestations/sha256:<digest>`; requires the request to succeed (network failure → refusal, FR-003).
2. `Decompressed` — bundle_url body snappy-decoded.
3. `Verified` — `attestation-verify` validates DSSE signature, Fulcio chain to public-good root, embedded SCT, Rekor SET + Merkle inclusion, and the `GithubPolicy`; subject digest must equal the downloaded file's digest.
4. `Missing`/`Unverifiable` → hard refusal unless `--skip-attestation` (FR-003).

### ReplaceOperation

The atomic binary replacement (delegates to `self_replace`).

| Attribute | Type | Notes |
|---|---|---|
| `target` | `PathBuf` | `std::env::current_exe()`. |
| `new_binary` | `PathBuf` | Extracted, verified binary. |

Invariants:
- POSIX: single atomic rename; the target path always resolves to the old or new binary (no absent window) — asserted by tests.
- Windows: `self_replace`'s move-aside + delete-glue strategy (best-effort; no longer silent).
- Runs only after checksum + attestation verification pass (FR-002/FR-003), so no post-verification rollback is needed.

## Validation rules (from FR-001..FR-009)

- **Secure by default**: both checksum and attestation verification MUST pass before install; each refusal is a hard error with a non-zero exit. The only relaxations are the explicit `--skip-checksum` and `--skip-attestation` flags, both documented as insecure (FR-001/FR-003/FR-009).
- **No silent downgrade**: string-equal/older tags are never installed by auto-update; only semver-strictly-greater (FR-004).
- **Explicit `--version`**: installs only after full verification, warns when older, rejects invalid tags with exit 2 (FR-005).
- **No swallowed errors**: every update error names the failing step and, where applicable, the file; non-zero exit (FR-008).
- **Ordering**: download → checksum verify → attestation verify → extract → replace. Any verification failure aborts before extraction/replacement (FR-002/FR-003, SC-004).

## Out of scope

Homebrew/cargo-install channel attestation, store/graph changes, hook/installer changes, and the release-CI attestation wiring (exercised in CI, not in unit tests).
