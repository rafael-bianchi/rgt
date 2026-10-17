# Feature Specification: Release Versioning Reconciliation

**Feature Branch**: `feat/036-release-versioning-reconciliation`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Reconcile the release-please state that has desynced from reality. `Cargo.toml`/tags are at 0.5.0 (a manual 'chore: bump version' commit), but `.release-please-manifest.json` still says 0.4.0, `CHANGELOG.md` has only the 0.4.0 entry, and `Formula/rgt.rb` is stuck at 0.4.0. Every feature from specs 019–029 has no changelog entry anywhere."

Source context: `docs/weakpoints-and-growth-opportunities.md` §2 item 3 and §3 item 3 (same root cause, two faces). Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #4, P1.

## Clarifications

### Session 2026-08-31

- Q: The manifest has two keys — `{"rgt":"0.1.0",".":"0.4.0"}`. The `rgt` key has never advanced past 0.1.0 (all release-please releases bumped `.`), and the `rgt` key was introduced then effectively abandoned in favor of `.`. How to reconcile? → A: Set the operative `.` key to 0.5.0 (matching `Cargo.toml`, the v0.5.0 tag, and the released v0.5.0 assets) and remove the stale `rgt` key — it is dead weight that release-please has never used to release and whose lingering 0.1.0 value perpetuates exactly the desync this spec exists to fix. The action (`cd.yml`) runs `release-type: rust` against `target-branch: develop` and has consistently advanced the `.` key (0.2.0 → 0.3.0 → 0.3.1 → 0.3.2 → 0.4.0).
- Q: The v0.5.0 release already exists on GitHub with all assets and checksums (verified). Is the Formula bump's SHA update part of this spec? → A: Yes — the real v0.5.0 `checksums.txt` values are used (not placeholders), since the release is live.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Release State Describes Reality Again (Priority: P1)

A maintainer (or the next automated release run) inspects the version-bearing state of the repository. Today: `Cargo.toml` says 0.5.0, the `v0.5.0` tag exists, and the GitHub release ships v0.5.0 artifacts — but `.release-please-manifest.json` says the root package is 0.4.0 (plus a stale `rgt: 0.1.0` entry), `CHANGELOG.md` documents only 0.4.0, and `Formula/rgt.rb` installs 0.4.0. Every feature shipped since (specs 019–029: update integrity, derive/verify hardening, locale-aware extraction, non-text-source capture, store/graph integrity, change-detection hardening, Bash-read capture, doctor, gc) is invisible to anyone reading the changelog or installing via Homebrew. This desync also risks release-please emitting a duplicate or mis-versioned release PR on its next run.

**Why this priority**: P1 per the growth plan — one root cause (a manual version bump bypassing release-please), three visible symptoms; fixes the state now and prevents recurrence.

**Independent Test**: A scripted consistency check asserts `Cargo.toml` version == `.release-please-manifest.json`'s `.` key == the `Formula/rgt.rb` `version` == the latest `CHANGELOG.md` heading; and the Homebrew formula's SHAs match the live v0.5.0 `checksums.txt`.

**Acceptance Scenarios**:

1. **Given** the repository at version 0.5.0, **When** the version-bearing files are read, **Then** `Cargo.toml`, `.release-please-manifest.json` (`.` key), `Formula/rgt.rb` (`version`), and the latest `CHANGELOG.md` release heading all agree on 0.5.0.
2. **Given** the reconciled manifest, **When** release-please's next run computes the next release, **Then** it starts from 0.5.0 and proposes the correct next version based on commits since v0.5.0 — not a stale 0.4.0 base (verified by manifest contents; the actual run happens in CI).
3. **Given** the v0.5.0 GitHub release, **When** the Homebrew formula is read, **Then** its per-arch SHAs equal the corresponding hashes in the release's `checksums.txt` (verified for the arches the formula supports: aarch64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-gnu).
4. **Given** the backfilled changelog, **When** a user reads `CHANGELOG.md`, **Then** the 0.5.0 entry documents the spec 019–029 features (update integrity, derive/verify hardening, store/graph integrity, locale-aware extraction, non-text-source capture, change-detection hardening, Bash-read capture, doctor, gc).

---

### Edge Cases

- The formula's `on_macos` block only covers `Hardware::CPU.arm?` (no Intel branch) — the v0.5.0 release *does* include `x86_64-apple-darwin.tar.gz`, but wiring the Intel branch is spec 035's decision, NOT this one. This spec updates the existing branches' SHAs only.
- The `rgt` manifest key removal: if release-please were somehow still reading it, a stale 0.1.0 could trigger a phantom "rgt" package release. Removing it eliminates that possibility; `.` is the key the action has actually maintained.
- `Cargo.lock` already carries 0.5.0 (synced in `ea948ae`), so no lockfile change is needed — verify, don't assume.
- Future bumps: the version must only move via release-please's own flow; this is a process note, not a code change.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `.release-please-manifest.json` MUST record the root package (`.`) at `0.5.0`, and MUST NOT contain the stale `rgt` key.
- **FR-002**: `CHANGELOG.md` MUST gain a `0.5.0` entry (matching the release-please style already used for 0.4.0) that documents the features shipped by specs 019–029.
- **FR-003**: `Formula/rgt.rb` MUST declare `version "0.5.0"` and update the sha256 for each supported arch to the corresponding value from the live v0.5.0 `checksums.txt`.
- **FR-004**: The reconciliation MUST NOT add the Intel-Mac branch to the formula (that decision belongs to spec 035) and MUST NOT change any production Rust code.
- **FR-005**: The version-bearing files MUST be consistent with each other and with the released v0.5.0 tag/assets (verified by the acceptance-test script).

### Key Entities *(include if feature involves data)*

- **VersionState**: the set of version-bearing artifacts — `Cargo.toml`, `.release-please-manifest.json`, `CHANGELOG.md`, `Formula/rgt.rb`, git tag `v0.5.0`, GitHub release assets + `checksums.txt`.
- **ChangelogEntry**: one release heading + bullet summary in release-please's conventional format.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A single source of truth check (script) passes: `Cargo.toml` == manifest `.` key == formula `version` == latest changelog heading == `0.5.0` (verified by running it).
- **SC-002**: The formula's three SHA values exactly match the v0.5.0 release `checksums.txt` for aarch64-apple-darwin, x86_64-unknown-linux-musl, and aarch64-unknown-linux-gnu (verified by diffing).
- **SC-003**: `CHANGELOG.md`'s 0.5.0 entry mentions `doctor`, `gc`, `--number-format`, and the update/verification hardening (grep-verified).
- **SC-004**: No production source code changes (`git diff --stat -- src/` empty); only manifest/changelog/formula/docs files change.

## Assumptions

- The v0.5.0 GitHub release is authoritative and its `checksums.txt` is the source of truth for the formula SHAs (verified live in this session).
- The `.` manifest key is the operative one release-please uses (proven by the release history 0.2.0→0.4.0 all advancing `.`); the `rgt` key is stale and safe to remove.
- The changelog entry style mirrors the existing 0.4.0 entry (release-please conventional format), since release-please will regenerate/append in that style.
- Version bumps going forward happen exclusively through release-please (process note for CONTRIBUTING/docs, not a code gate).

## Out of scope

- The Intel-Mac decision (spec 035 — formula Intel branch, release matrix) — explicitly deferred.
- Spec 034's crates.io publishing (a different distribution state) — separate spec.
- Any production-code change, CI behavior change, or docs beyond the changelog/manifest/formula reconciliation.
