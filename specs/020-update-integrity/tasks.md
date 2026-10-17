# Tasks: Update Integrity

**Input**: Design documents from `/specs/020-update-integrity/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed user-facing behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- New modules: `src/updater/version.rs`, `src/updater/attestation.rs`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the four new dependencies

- [x] T001 Add `semver` = "1", `snap` = "1", `attestation-verify` (pin `=0.1`), and `self_replace` = "1.5" to `[dependencies]` in `Cargo.toml`; run `cargo build` to confirm they compile on the current stable toolchain (note MSRV 1.88 for `attestation-verify`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The semver gating helper, the attestation fetch/decompress/verify module, and the atomic-replace rewrite that every user story builds on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T002 [P] Unit tests in `tests/unit/test_update_semver.rs`: (a) `latest_is_newer("1.10.0", "1.9.9")` → true; (b) `latest_is_newer("1.9.9", "1.10.0")` → false (string-greater but semver-smaller — the §7.1 downgrade case); (c) pre-release ordering `1.0.0-alpha.1 < 1.0.0`; (d) invalid input → error
- [x] T003 [P] Unit tests in `tests/unit/test_update_attestation.rs`: capture a real Sigstore v0.3 bundle from a public repo's `/attestations/sha256:<digest>` endpoint, snappy-decompress, commit as `tests/fixtures/attestation_bundle.json` (record the artifact SHA-256 and the source repo), then assert `verify_bundle` succeeds against that digest + a policy matching the fixture's repo, and FAILS against a tampered digest
- [x] T004 [P] Unit tests in `src/updater/atomic.rs` (extend the existing `#[cfg(test)]` module): (a) a POSIX watcher-style test that the target path always resolves to old-or-new during replace (no absent window); (b) a failure path reports an error instead of swallowing it

### Implementation for Foundational Primitives

- [x] T005 Implement `src/updater/version.rs` with `parse_version(&str) -> Result<semver::Version, String>` and `latest_is_newer(latest: &str, current: &str) -> Result<bool, String>` using `semver` `Ord` precedence; register `pub mod version;` in `src/updater/mod.rs`
- [x] T006 Implement `src/updater/attestation.rs` with `fetch_bundle(subject_digest, token) -> Result<String, String>` (ureq `GET /repos/rafael-bianchi/rgt/attestations/sha256:<digest>`, follow `attestations[0].bundle_url`), `decompress_bundle(&[u8]) -> Result<String, String>` (snap), and `verify_bundle(bundle_json, artifact_sha256, policy) -> Result<(), String>` via `attestation-verify` against the embedded public-good trust root; register `pub mod attestation;` in `src/updater/mod.rs`
- [x] T007 Rewrite `AtomicReplace::replace` in `src/updater/atomic.rs` to delegate to `self_replace::self_replace` (POSIX single atomic rename, Windows handled); remove the two-step `target → .old` rename; propagate errors instead of `let _ =`
- [x] T022 Introduce an injectable release-fetcher seam in `src/cli/update.rs`: `execute_update` accepts a release-fetcher (closure or trait) defaulting to the real `github::fetch_*` calls, so contract tests can supply stub releases and verification outcomes without network — required for deterministic US1/US2/US3 contract tests (Constitution IX)

**Checkpoint**: `cargo test test_update_semver test_update_attestation` and the atomic unit tests pass; `src/updater/{version,attestation,atomic}.rs` compile; the fetcher seam (T022) is wired through `execute_update`

---

## Phase 3: User Story 1 - `rgt update` Refuses Anything It Cannot Verify (Priority: P1) 🎯 MVP

**Goal**: Missing `checksums.txt` or a missing/unverifiable attestation is a hard refusal with a non-zero exit; the only relaxations are explicit `--skip-checksum` / `--skip-attestation` (FR-001/FR-002/FR-003)

**Independent Test**: Contract tests drive `execute_update` against stub releases and assert refusal + exit 1 unless the matching `--skip-*` flag is present

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T008 [P] [US1] Contract tests in `tests/contract/test_update_contract.rs` (via the T022 fetcher seam — no live network): (a) stub release with no `checksums.txt` → refusal naming the missing asset + non-zero exit; (b) same with `--skip-checksum` → proceeds; (c) missing/unverifiable attestation → refusal unless `--skip-attestation`; (d) checksum mismatch → abort, nothing installed

### Implementation for User Story 1

- [x] T009 [US1] Add `--skip-checksum` and `--skip-attestation` bool flags to the `Update` command in `src/main.rs` and thread them into `execute_update`
- [x] T010 [US1] In `execute_update` (`src/cli/update.rs`), replace the silent `if let Some(...)` checksum path with a hard error when `find_checksum_asset` returns `None` (unless `--skip-checksum`); keep the mismatch abort; missing checksum entry for the asset is also an error
- [x] T011 [US1] Wire attestation verification into `execute_update` (`src/cli/update.rs`): compute the SHA-256 of the downloaded archive, call `attestation::fetch_bundle`/`decompress_bundle`/`verify_bundle` with the `rafael-bianchi/rgt` workflow policy unless `--skip-attestation`; on any failure return the contract's refusal message + non-zero exit

**Checkpoint**: US1 contract tests pass; a release without `checksums.txt` or without a verifiable attestation is refused

---

## Phase 4: User Story 2 - `rgt update` Never Silently Installs an Older Version (Priority: P1)

**Goal**: semver precedence gating for auto-update and `--check`; `--version` warns on older tags and rejects invalid input (FR-004/FR-005/FR-006)

**Independent Test**: Contract tests assert the string-greater/semver-smaller downgrade case reports "up to date" with no download; a genuine newer tag proceeds; invalid `--version` exits 2

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T012 [P] [US2] Contract tests in `tests/contract/test_update_contract.rs` (via the T022 fetcher seam): (a) current `1.10.0` vs latest tag `v1.9.9` → "up to date", no download, exit 0; (b) latest `v1.10.1` → proceeds; (c) `--version v1.x` (invalid semver) → exit 2; (d) `--version <older>` → warns "older than installed" then proceeds only after verification

### Implementation for User Story 2

- [x] T013 [US2] Replace the string `==`/`!=` compares in `execute_update` and `--check` (`src/cli/update.rs`) with `version::latest_is_newer`; equal or older → "up to date", no download
- [x] T014 [US2] Rework the `--version <tag>` path in `src/cli/update.rs`: validate the tag exists and parses as semver (invalid → clear error, exit 2); warn when requested < current; proceed only after verification — the `--skip-checksum`/`--skip-attestation` opt-outs apply here identically (FR-005)

**Checkpoint**: US2 contract tests pass; no auto-downgrade, `--version` is verified and validated

---

## Phase 5: User Story 3 - `rgt update` Never Leaves the Binary Missing or Broken (Priority: P2)

**Goal**: POSIX replace is a single atomic rename with no absent-target window; failures are reported, never swallowed (FR-007/FR-008)

**Independent Test**: Atomic unit tests assert the no-absent-window property and that a failure surfaces as an error; an audit finds no `let _ =` in the update path

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T015 [P] [US3] Tests in `src/updater/atomic.rs` + `tests/contract/test_update_contract.rs` (via the T022 fetcher seam): (a) replace success leaves target with new content and no `.old`/`.tmp` leftovers; (b) a simulated replace failure returns `Err` (not swallowed); (c) verification-before-replace ordering is asserted at the `execute_update` level (a failed verification never reaches replace)

### Implementation for User Story 3

- [x] T016 [US3] Call the rewritten `AtomicReplace` (`self_replace`) from `execute_update` (`src/cli/update.rs`) after both verifications pass; keep the existing success/failure messaging
- [x] T017 [US3] Audit `src/cli/update.rs` and `src/updater/*.rs` for `let _ =`-swallowed errors and surface each as a named error with the failing step (FR-008); the §7.1 TOCTOU (verify-then-extract) is documented as accepted low-risk in a code comment

**Checkpoint**: US3 tests pass; the update path reports all failures and never leaves the target absent on POSIX

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Release-CI attestation, documentation, and final validation

- [x] T018 [P] Add an `actions/attest-build-provenance` step to `.github/workflows/release.yml` attesting every release artifact (the production source of attestations `rgt update` verifies)
- [x] T019 [P] Update `README.md` and the `rgt update` `--help` text to document `--skip-checksum`/`--skip-attestation` (marked insecure) and the verification behavior
- [x] T020 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T021 Run the validation scenarios in `specs/020-update-integrity/quickstart.md` (manual scenarios 1–4 or their automated equivalents)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — add deps first
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: Depends on Foundational (T006 attestation, T007 atomic)
- **US2 (Phase 4)**: Depends on Foundational (T005 version) — independent of US1, but both edit `src/cli/update.rs`, so run sequentially (US1 then US2)
- **US3 (Phase 5)**: Depends on Foundational (T007) — edits the same `execute_update`, run after US2
- **Polish (Phase 6)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: after Foundational — refusal paths
- **US2 (P1)**: after Foundational — version gating (shares `update.rs` with US1)
- **US3 (P2)**: after Foundational — atomic replace + error hygiene

### Within Each User Story

- Tests written first, must FAIL before implementation
- Primitives before wiring (Foundational); flags before refusal logic (US1); gating before `--version` (US2)
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T002 / T003 / T004 (Foundational tests) — different files
- Foundational impl tasks are sequential (T005, T006, T007, T022 are independent files → can run in parallel after their tests exist)
- T008 / T012 / T015 (story tests) — different files
- T018 / T019 / T020 (Polish) — different files

---

## Parallel Example: Foundational Phase

```bash
# Tests first (expect FAIL):
Task: "test_update_semver.rs"
Task: "test_update_attestation.rs (+ tests/fixtures/attestation_bundle.json)"
Task: "atomic.rs unit tests"

# Implementation (independent files, can run together):
Task: "src/updater/version.rs"
Task: "src/updater/attestation.rs"
Task: "src/updater/atomic.rs (self_replace)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (deps)
2. Complete Phase 2: Foundational (version, attestation, atomic + tests — CRITICAL, blocks everything)
3. Complete Phase 3: User Story 1 (refuse unverified installs)
4. **STOP and VALIDATE**: refusal contract tests green; a release without `checksums.txt` or attestation is refused with exit 1
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → verification/gating primitives in place
2. US1 → unverified binaries are refused (MVP)
3. US2 → no silent downgrades, `--version` validated
4. US3 → atomic replace + full error reporting
5. Polish → CI attestation step + docs + full-suite validation

---

## Notes

- [P] tasks = different files, no dependencies; the US1/US2/US3 wiring tasks are intentionally NOT parallel because they all edit `src/cli/update.rs`
- [Story] label maps task to user story for traceability
- The attestation fixture (T003) is captured from a public repo at test-writing time; if the embedded trust root rejects it, adjust the fixture to a freshly-fetched bundle (documented in `research.md` §2)
- MSRV 1.88 (attestation-verify) is a documented constraint; CI runs stable
- Commit after each task or logical group; stop at each checkpoint

---

## Phase 7: Convergence

- [x] T023 Map a "release not found" fetch failure on the `--version` path in `src/cli/update.rs` to `invalid_input` (exit 2), matching the contract for nonexistent tags, per FR-005 (partial) — also fixed the pre-existing `github.rs` 404-detection bug (`contains("status 404")` never matched ureq's `"status code 404"`)
- [ ] T024 After the first release attested by the new CI, capture a real `rafael-bianchi/rgt` attestation bundle as `tests/fixtures/rgt/` and add a policy regression test (adjusting `RGT_SIGNER_WORKFLOW`/ref policy in `src/updater/attestation.rs` if the recorded identity differs), per FR-003 / T018 (partial) — **BLOCKED**: no `rafael-bianchi/rgt` release is attested yet (CI step just shipped). Plumbing prepared: `tests/fixtures/rgt/README.md` (capture procedure) + gated `verifies_real_rgt_bundle_when_fixture_present` test that auto-activates once the fixture is added.
