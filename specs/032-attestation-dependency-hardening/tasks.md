# Tasks: Attestation Dependency Hardening

**Input**: Design documents from `/specs/032-attestation-dependency-hardening/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/attestation-hardening-contract.md, quickstart.md

**Tests**: Included — the capture script's deterministic parts are unit-tested (Constitution IX).

**Organization**: Single user story; three deliverables (pin, docs, script).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the current state.

- [ ] T001 Confirm: `Cargo.toml` has the caret `attestation-verify = "0.1.0"`; `cargo search attestation-verify` shows 0.1.0 as the only version; the GitHub API returns zero attestations for v0.5.0 assets (no real attested release yet); the policy constants (owner/repo ids, workflow path) match the live API

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The capture script + unit test (the testable core).

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T002 Create `scripts/capture-rgt-attestation-fixture.sh` (executable): args `TAG [ASSET]`; downloads the released archive, computes its SHA-256, queries `gh api "/repos/rafael-bianchi/rgt/attestations/sha256:<hex>" --jq '.attestations[0].bundle_url'`, downloads + `snap`-decompresses the bundle, writes `tests/fixtures/rgt/rgt-bundle.json` + `artifact.sha256`; exits non-zero with a clear "no attestation" message (never writes a corrupt fixture) when the release has no attestation
- [ ] T003 Add a unit test covering the capture logic's deterministic parts in `tests/unit/test_update_attestation.rs` (digest/URL construction; snappy round-trip via the existing `decompress_bundle`)

**Checkpoint**: The script runs against today's v0.5.0 and fails cleanly (no attestation); the unit test passes.

---

## Phase 3: User Story 1 - The Update Trust Guarantee Is Protected From Silent Dependency Drift (Priority: P2) 🎯 MVP

**Goal**: Exact pin + documented risk acceptance + one-command fixture capture.

**Independent Test**: `grep` the pin, `grep` SECURITY.md, run the script against v0.5.0.

### Tests for User Story 1

> **NOTE: T003 is the test; the script's clean-failure path is verified by running it (SC-003).**

### Implementation for User Story 1

- [ ] T004 [US1] Change `Cargo.toml` to `attestation-verify = "=0.1.0"` (exact pin) and confirm `cargo build`/`cargo test` still pass (Cargo.lock already resolves 0.1.0, so no lockfile churn)
- [ ] T005 [US1] Create `SECURITY.md` documenting: the dependency's immaturity/zero adoption and security-critical role; that `rgt_policy()` has never been exercised against a real signed `rafael-bianchi/rgt` release; the fail-closed rationale (a verification failure refuses the install, so a dependency bug surfaces as a refusal, never silent acceptance); and the remediation trigger (run the capture script against the first attested release, then the gated self-test runs for real)
- [ ] T006 [US1] Update the `eprintln!` skip message in `verifies_real_rgt_bundle_when_fixture_present` (`tests/unit/test_update_attestation.rs`) to reference `SECURITY.md` and the capture script (still a skip, not a hard failure)

**Checkpoint**: Pin greps exactly; SECURITY.md is complete; the gated test still skips with the updated message.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T007 Run the full contract: grep pin, grep SECURITY.md terms, run the capture script against v0.5.0 (clean failure, no fixture written), `cargo test --test test_update_attestation`, `cargo fmt --all -- --check`, `cargo clippy --all-targets`, full `cargo test`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T002/T003).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story.

### Within User Story 1

- T004/T005/T006 touch distinct files — parallel after T002/T003.

### Parallel Opportunities

- T004 (Cargo.toml), T005 (SECURITY.md), T006 (test message) — independent.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002–T003)
3. Phase 3: US1 (T004–T006)
4. **STOP and VALIDATE**: pin + SECURITY.md + clean-failure script + green suite

### Incremental Delivery

1. Foundational → capture script + unit test
2. US1 → pin, SECURITY.md, updated skip message
3. Polish → full contract validation

---

## Notes

- No updater or policy behavior change — hardening/docs/tooling only.
- Un-skipping the fixture test is genuinely gated on a real attested release (none exists — verified this session); the script makes it a one-command step when one ships.
- The self-skip stays a skip: a hard failure would break every PR until a release exists.
