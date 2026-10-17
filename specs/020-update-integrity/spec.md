# Feature Specification: Update Integrity

**Feature Branch**: `feat/020-update-integrity`

**Created**: 2026-08-28

**Status**: Draft

**Input**: User description: "rgt update must never install an unverified binary, must never silently install an older version, must verify the binary/checksum against a trust anchor independent of the release's own assets, and must never leave the running binary missing or broken mid-update. Findings §7.1 (checksum-bypass CRITICAL, attestation HIGH, semver HIGH, atomic-replace MEDIUM, swallowed-errors LOW)."

Source context: `docs/findings-hooks-and-value-extraction.md` §7.1 (plan: `docs/spec-plan-from-findings.md`, spec #2, P0).

## Clarifications

### Session 2026-08-28

- Q: Which independent signature/attestation mechanism should 020 adopt? → A: GitHub Artifact Attestations — releases are already built in GitHub Actions; attest builds via OIDC at build time, verify at update time (in-binary verifier preferred). Lowest key-management burden; the trust anchor is external to the release's own assets.
- Q: How should `rgt update` verify the attestation at update time? → A: In-binary verifier (sigstore crate) — no `gh` dependency, self-contained single binary, unit-testable with real attestation fixtures; the larger compile-time dependency tree is accepted.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - `rgt update` Refuses Anything It Cannot Verify (Priority: P1)

A developer runs `rgt update`. Before the downloaded binary is installed, the update must be verified against BOTH the release's `checksums.txt` (SHA-256) AND a GitHub Artifact Attestation — a trust anchor independent of the release's own assets. If the release is missing `checksums.txt`, the attestation is missing, or either verification fails, the update is **refused with a clear error and a non-zero exit** — never silently skipped. The only way to relax this is an explicit, documented, insecure opt-out flag.

**Why this priority**: Findings §7.1 is CRITICAL: today a release missing `checksums.txt` falls through `if let Some(...)` with no `else` (`src/cli/update.rs:82-94`) and installs a completely unverified binary. This is a supply-chain hole — an accidental or compromised release installs arbitrary code. "Secure by default, explicit opt-out" is the required posture (curl `-k` precedent).

**Independent Test**: Contract tests run `execute_update` against a stub release that has no `checksums.txt` asset and assert the update is refused (error + non-zero exit) unless `--skip-checksum`; the same for a missing/unverifiable attestation unless `--skip-attestation`.

**Acceptance Scenarios**:

1. **Given** a release with no `checksums.txt` asset, **When** `rgt update` runs, **Then** it prints a clear error naming the missing asset and exits non-zero, installing nothing.
2. **Given** the same release with `--skip-checksum`, **When** `rgt update` runs, **Then** it proceeds (documented as insecure) only if the remaining verification (attestation) passes.
3. **Given** a release whose attestation is missing or fails verification, **When** `rgt update` runs, **Then** it is refused with a clear error and non-zero exit unless `--skip-attestation`.
4. **Given** a checksum mismatch, **When** `rgt update` runs, **Then** the install is aborted, the current binary is left intact, and the exit is non-zero.

---

### User Story 2 - `rgt update` Never Silently Installs an Older Version (Priority: P1)

A developer runs `rgt update` (or `rgt update --check`). RGT compares the published release against the installed version using semantic-version ordering, not string equality. If the release is not strictly newer, it reports "up to date" and installs nothing. An older tag republished as "latest" is never treated as an update.

**Why this priority**: Today `src/cli/update.rs:21/:33` compares with `!=`/`==` on the version string, and `/releases/latest` is time-sorted (GitHub's own docs), so a republished older tag would be reported and installed as an "update" — a silent downgrade, potentially to a vulnerable build. This is a HIGH correctness/security gap confirmed in §7.1.

**Independent Test**: Contract tests with a fake latest tag that is string-greater but semver-smaller (e.g. current `1.10.0` vs latest `v1.9.9`) assert "up to date", no download, exit 0; and a genuine newer tag (semver-greater) proceeds.

**Acceptance Scenarios**:

1. **Given** the installed version is `1.10.0` and the published tag is `v1.9.9` (string-ordered as newer), **When** `rgt update` runs, **Then** it reports "up to date" and installs nothing.
2. **Given** the published tag `v1.10.1`, **When** `rgt update` runs, **Then** it proceeds to update.
3. **Given** a tag that is not valid semver (e.g. `v1.x`), **When** `rgt update` runs, **Then** it errors clearly (invalid input) and exits 2, installing nothing.
4. **Given** an explicit `--version <older-tag>`, **When** `rgt update` runs, **Then** it warns that the requested tag is older than the installed version and proceeds only after verification (explicit user request), never by default.

---

### User Story 3 - `rgt update` Never Leaves the Binary Missing or Broken (Priority: P2)

A developer updates RGT. The replacement of the running binary is atomic on POSIX — a single rename — so there is never a moment where no `rgt` binary exists at the target path. If the replacement fails, the previous binary is restored or recoverable, and error paths report what failed instead of swallowing it.

**Why this priority**: `AtomicReplace::replace` (`src/updater/atomic.rs:20-43`) today renames `target → target.old` and then `tmp → target`; a crash or failure between the two renames leaves the binary missing, and the backup-cleanup errors are discarded with `let _ =`. Not CRITICAL, but a reliability gap in the update path (MEDIUM per §7.1).

**Independent Test**: Unit tests assert that on POSIX the replace is a single `rename(tmp, target)` with no `target → old` intermediate step, that a forced failure leaves the target intact or restores it, and that cleanup errors are reported rather than swallowed.

**Acceptance Scenarios**:

1. **Given** an existing target binary, **When** the replace runs on POSIX, **Then** the target path always resolves to either the old or the new binary (no missing window), observable by an external watcher.
2. **Given** the replace fails (e.g. filesystem error or permission denied), **When** the failure is handled, **Then** a clear error is reported and the old binary at the target path is left intact — the replace is a single atomic rename on POSIX, so there is no "moved aside" intermediate state to restore from.
3. **Given** a cleanup/backup-removal failure, **When** the update finishes, **Then** a clear message names the leftover file rather than silently ignoring it.

---

### Edge Cases

- What happens when the release has no binary asset for the platform's target triple?
- What happens when `checksums.txt` exists but contains no entry for the selected asset?
- What happens when the attestation was issued for a different repository or digest?
- What happens when verification can't reach the network (offline) — is that a hard refusal or a warning?
- What happens when the current binary is a locally-built dev build whose version isn't a release tag (semver parse of `CARGO_PKG_VERSION`)?
- What happens when two `rgt update` runs race the same target binary?
- What happens if the process is killed between the download/verify and the replace?
- What happens when the requested `--version` tag doesn't exist or isn't a valid release?
- What happens on Windows, where a running `.exe` cannot be renamed (ERROR_SHARING_VIOLATION)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt update` MUST refuse to install when the release has no `checksums.txt` asset, printing a clear error and exiting non-zero — unless the explicit `--skip-checksum` flag is passed. Missing checksums are NEVER a silent default skip.
- **FR-002**: When a checksum is present, verification MUST run before extraction/installation, and a mismatch MUST abort with a non-zero exit (never install on mismatch).
- **FR-003**: `rgt update` MUST verify the release's GitHub Artifact Attestation (independent of the release's own assets) before installing; a missing or unverifiable attestation MUST refuse the update with a clear error and non-zero exit unless the explicit `--skip-attestation` flag is passed. The attested digest is the SHA-256 of the downloaded archive (the artifact the attestation covers), not the extracted binary.
- **FR-004**: `rgt update` (no `--version`) MUST compare versions with semantic-version ordering and proceed ONLY when the published version is strictly greater than the installed version; equal or older is "up to date" (no install).
- **FR-005**: `--version <tag>` MUST install only after the same verification (FR-001–FR-003) passes, MUST warn when the requested tag is older than the installed version, and MUST reject invalid/nonexistent tags with a clear error (invalid input → exit 2). The `--skip-checksum`/`--skip-attestation` opt-outs apply to `--version` installs identically.
- **FR-006**: `--check` MUST use the same semantic-version comparison (FR-004) for its report.
- **FR-007**: On POSIX, binary replacement MUST be a single atomic rename with no intermediate window where the target path is absent; a failed replace leaves the previous binary intact at the target path (there is no "moved aside" intermediate state on POSIX), and the previous binary remains recoverable on success.
- **FR-008**: All update error paths MUST report what failed and, where applicable, which file, with a non-zero exit; no `let _ =`-style swallowed errors in the update path.
- **FR-009**: The insecure opt-out flags (`--skip-checksum`, `--skip-attestation`) MUST be explicit, documented as insecure, and never the default.

### Key Entities *(include if feature involves data)*

- **Release**: A published GitHub release with `tag_name` and an `assets` list. Tag must be valid semver for auto-update comparison.
- **ReleaseAsset**: A downloadable artifact (binary archive) with a name and download URL.
- **ChecksumsFile**: The release's `checksums.txt` asset; the SHA-256 manifest used for integrity verification.
- **Attestation**: A GitHub Artifact Attestation binding the build to the repository via OIDC — the independent trust anchor.
- **SemverVersion**: The parsed semantic version used for ordering decisions (strictly-greater comparison).
- **AtomicReplace**: The binary-replacement operation; on POSIX a single rename with rollback, no absent-target window.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 0 unverified installs: every auto-update in the contract test suite either passes BOTH checksum and attestation verification or is refused with a non-zero exit (opt-out flags excluded by test design).
- **SC-002**: 0 silent downgrades: an older tag (semver-smaller) is never installed by `rgt update` without an explicit `--version` request and warning.
- **SC-003**: 100% of POSIX replace operations complete with the target path always resolving to the old or new binary (no absent window), asserted by unit tests.
- **SC-004**: 100% of failed verifications leave the current binary intact and functional (asserted by contract tests).
- **SC-005**: 100% of update error messages name the failing step and, where applicable, the file; no error path is silent.

## Assumptions

- Scope is the §7.1 finding family: checksum-bypass (CRITICAL), independent attestation verification (HIGH), semver ordering (HIGH), atomic replace + rollback (MEDIUM), swallowed-errors/TOCTOU hygiene (LOW). Nothing else in the update path is changed.
- Signature mechanism: **GitHub Artifact Attestations** (user-confirmed 2026-08-28). Every release is attested at build time via `actions/attest-build-provenance`; verification at update time is **in-binary via the sigstore crate** (user-confirmed 2026-08-28), with no `gh` subprocess dependency. If the verifier cannot reach the required infrastructure, the update is refused (FR-003) — offline installs require the explicit `--skip-attestation` opt-out.
- Explicit opt-out flags `--skip-checksum` and `--skip-attestation` are the only relaxations; both documented as insecure (curl `-k` precedent from §7.1).
- POSIX atomicity is required (FR-007); Windows behavior (running-`.exe` rename) remains best-effort and is flagged as a known limitation, not silently hidden.
- No changes to hook configuration, extraction, storage, or the rest of the codebase; `src/updater/*`, `src/cli/update.rs`, and the update CLI flags are the only touched surfaces.
- Out of scope: signature/attestation for Homebrew/cargo-install distribution channels (RGT's own `rgt update` path only), and the release-CI attestation wiring is part of this feature's implementation but is exercised in CI, not in tests.
