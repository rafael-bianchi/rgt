# Tasks: Release Versioning Reconciliation

**Input**: Design documents from `/specs/036-release-versioning-reconciliation/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/version-state-contract.md, quickstart.md

**Tests**: Included — the spec requires a runnable consistency check (Constitution IX).

**Organization**: Single user story; mechanical state-file reconciliation.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the audited desync state.

- [ ] T001 Verify the desync: confirm `Cargo.toml`/tag/release are 0.5.0 while `.release-please-manifest.json` (`{"rgt":"0.1.0",".":"0.4.0"}`), `CHANGELOG.md` (only 0.4.0), and `Formula/rgt.rb` (version 0.4.0) are not; fetch the v0.5.0 `checksums.txt` and record the three SHAs (aarch64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-gnu)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The consistency-check script that guards the reconciled state.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T002 Create `scripts/check-version-state.sh`: read version from `Cargo.toml` (`^version =`), manifest `.` key, `Formula/rgt.rb` (`version "..."`), and latest `CHANGELOG.md` `## [X.Y.Z]` heading; assert all four equal AND the manifest has no `rgt` key; if `RGT_CHECKSUMS_URL` is set and reachable, diff the three formula SHAs against the checksums file, else warn and pass; exit non-zero on any mismatch

**Checkpoint**: The script runs and (correctly) fails against the current desynced state.

---

## Phase 3: User Story 1 - Release State Describes Reality Again (Priority: P1) 🎯 MVP

**Goal**: Manifest, changelog, and formula all reflect 0.5.0; the check script passes.

**Independent Test**: `scripts/check-version-state.sh` exits 0 with the real v0.5.0 checksums URL.

### Tests for User Story 1

> **NOTE: Write these FIRST — the check script (T002) is the test; it must fail before the fix and pass after.**

### Implementation for User Story 1

- [ ] T003 [US1] Reconcile `.release-please-manifest.json` to `{".": "0.5.0"}` (remove the stale `rgt` key)
- [ ] T004 [US1] Backfill `CHANGELOG.md` with a `0.5.0` entry in release-please's conventional format (dated 2026-08-30), summarizing the 16 commits `v0.4.0..v0.5.0` and naming `rgt doctor`, `rgt gc`, `--number-format`, update integrity/attestations, derive/verify hardening, store & graph integrity, change-detection hardening, non-text-source capture, and Bash-read capture
- [ ] T005 [US1] Bump `Formula/rgt.rb` to `version "0.5.0"` and update the three sha256 values to the v0.5.0 `checksums.txt` values from T001 (NO Intel-Mac branch — spec 035's decision)

**Checkpoint**: `scripts/check-version-state.sh` passes (exit 0), both offline and with the v0.5.0 checksums URL.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T006 Run `git diff --stat -- src/` (assert empty), `cargo test` (confirm no regression from state changes — should be a no-op), and `scripts/check-version-state.sh` with the live URL; confirm `CHANGELOG.md` 0.5.0 entry grep-matches `doctor`, `gc`, `--number-format`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T002).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story.

### Within User Story 1

- T003/T004/T005 touch three distinct files — can run in parallel after the check script exists.

### Parallel Opportunities

- T003 (manifest), T004 (changelog), T005 (formula) are independent files.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002)
3. Phase 3: US1 (T003–T005)
4. **STOP and VALIDATE**: `scripts/check-version-state.sh` exit 0

### Incremental Delivery

1. Foundational → check script (fails on desync)
2. US1 → manifest + changelog + formula reconciled
3. Polish → offline+online check, no src/ diff, changelog grep

---

## Notes

- No production code changes (`git diff --stat -- src/` empty).
- The check script must be network-independent (Constitution IX): SHA verification only when `RGT_CHECKSUMS_URL` is reachable, else warn+pass.
- Going forward, version bumps happen only through release-please (documented in the spec's assumptions; not a code gate).
