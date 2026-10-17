# Research: Release Versioning Reconciliation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in the actual repo state and the live GitHub release (fetched this session): `.release-please-manifest.json`, `Cargo.toml`, `CHANGELOG.md`, `Formula/rgt.rb`, git tags, `cd.yml`, and the v0.5.0 `checksums.txt`.

## 1. Current state audit (FR-001..FR-005)

| Artifact | Current | Reality (0.5.0) |
|---|---|---|
| `Cargo.toml` | `0.5.0` | ✓ matches |
| `v0.5.0` tag | exists (pushed to origin, `acc5e18`) | ✓ matches |
| GitHub release v0.5.0 | live, non-draft, all 5 assets + checksums.txt | ✓ matches |
| `.release-please-manifest.json` | `{"rgt":"0.1.0",".":"0.4.0"}` | ✗ `.` must be 0.5.0; `rgt` key stale |
| `CHANGELOG.md` | only 0.4.0 entry | ✗ missing 0.5.0 |
| `Formula/rgt.rb` | version 0.4.0 + 0.4.0 SHAs | ✗ must be 0.5.0 + 0.5.0 SHAs |

**Root cause**: `4fb9a85 chore: bump version to 0.5.0 for release` hand-bumped `Cargo.toml` (and a follow-up synced `Cargo.lock`), bypassing release-please. The three state files never caught up.

## 2. Manifest reconciliation (FR-001)

**Decision**: `.release-please-manifest.json` becomes `{".": "0.5.0"}`.

**Rationale**: The release history proves `.` is the operative key — every release-please release advanced `.` (0.2.0 → 0.3.0 → 0.3.1 → 0.3.2 → 0.4.0), never `rgt`. The `rgt` key entered via `1346742` ("use package name as manifest key") but release-please then ignored it; leaving `{"rgt":"0.1.0",".":"0.5.0"}` would preserve a stale 0.1.0 that could mislead a future run into thinking a second package exists. Remove it.

**Alternatives considered**: Keep `rgt` alongside `.` — rejected: it is dead weight and perpetuates the exact confusion this spec removes.

## 3. Changelog backfill (FR-002, SC-003)

**Decision**: Add a `0.5.0` entry in release-please's conventional format (matching the existing 0.4.0 entry), dated 2026-08-30 (the release date per the release asset `createdAt`). Content summarizes the 16 non-merge commits `v0.4.0..v0.5.0`, grouped as Features/Fixes, explicitly naming `rgt doctor`, `rgt gc`, `--number-format`, update integrity/attestations, derive/verify hardening, store & graph integrity, change-detection hardening, non-text-source capture, and Bash-read capture.

**Rationale**: Constitution X — the changelog must document shipped behavior; users read it to discover features. Grouping follows release-please's own convention so a future release-please run can regenerate consistently.

## 4. Homebrew formula bump (FR-003, SC-002)

**Decision**: `Formula/rgt.rb` `version "0.5.0"`; update the three SHA values to the v0.5.0 `checksums.txt` (verified live):

| Arch | v0.5.0 sha256 |
|---|---|
| aarch64-apple-darwin | `b177700ef4465d34564f0e3932dde55f50b18e2bebc42d715b6a7ff6cc8bfed5` |
| x86_64-unknown-linux-musl | `9b4bcdf56c5bed3eacbbf02741d57e3ccda9e5a1244b5d8321b9b108aec22b6c` |
| aarch64-unknown-linux-gnu | `08d9e5532de93ad44fdfe560a5119c3857999233162eeb998c420a504db0b2e1` |

The formula's `on_macos` block covers only `Hardware::CPU.arm?`; the Intel branch is spec 035's call and is NOT added here (FR-004). The `x86_64-apple-darwin` asset exists in the release but is out of scope for this spec.

**Alternatives considered**: Placeholder SHAs — rejected: the real release exists, so real values are available and correct.

## 5. Verification (SC-001, SC-002, SC-004)

**Decision**: A new `scripts/check-version-state.sh`:
1. Reads the version from `Cargo.toml` (`^version = "..."`), `.release-please-manifest.json` (`.` key), `Formula/rgt.rb` (`version "..."`), and the latest `CHANGELOG.md` heading (`## [X.Y.Z]`); asserts all four equal.
2. Asserts the manifest has no `rgt` key.
3. If `RGT_CHECKSUMS_URL` is set and reachable, diffs the formula's three SHAs against that `checksums.txt`; if unreachable, prints a warning and passes (offline-safe — Constitution IX: `cargo test` must not depend on the network).
4. Exits non-zero on any mismatch.

`git diff --stat -- src/` empty is asserted as a manual check (SC-004), since the script itself cannot know CI state.

**Rationale**: A runnable, deterministic guard (Constitution IX) that catches the next hand-bump desync without adding a network dependency to the test suite.
