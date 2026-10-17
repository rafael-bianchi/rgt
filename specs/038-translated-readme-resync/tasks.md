# Tasks: Translated README Resync

**Input**: Design documents from `/specs/038-translated-readme-resync/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/readme-parity-contract.md, quickstart.md

**Tests**: Included — the spec requires a structural parity check (Constitution IX).

**Organization**: Single user story; the six files are independent, parallel edits.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the parity gap and create the verification tool.

- [ ] T001 Confirm the gap: grep all 6 translations for `doctor`, `gc`, `number-format`, `attestation` (expect 0 hits); confirm the English sections listed in research.md §1 exist in README.md
- [ ] T002 Create `scripts/check-readme-parity.sh`: per language, assert localized Doctor/Update/`gc`/`--number-format`/non-text-source content is present; assert the language bar in all 7 files links all 7 variants; assert balanced fences and consistent table columns; offline + deterministic

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: A reference mapping of the exact English content to add, so the six edits are consistent.

**⚠️ CRITICAL**: No user-story work can begin until this phase is complete.

- [ ] T003 Extract the exact English content to add per translation from `README.md`: the `Doctor` section, the `Update` section, the `gc` line + expanded Operations prose in `Provenance`, the `--number-format` additions to `Init & Hooks`/`record`, the config-preservation + absolute-path paragraphs, and the non-text-source callout (capture the exact strings for faithful translation)

**Checkpoint**: The reference content is identified; the parity script runs and (correctly) fails on the current files.

---

## Phase 3: User Story 1 - Non-English Users Can Discover the Full Toolset (Priority: P1) 🎯 MVP

**Goal**: All six translations carry the full feature set; the parity check passes.

**Independent Test**: `scripts/check-readme-parity.sh` exits 0.

### Tests for User Story 1

> **NOTE: The parity script (T002) is the test; it must fail before the edits and pass after.**

### Implementation for User Story 1

- [ ] T004 [P] [US1] Reconcile `README_es.md`: add Doctor, Update (checksums+attestation+semver), `gc`, `--number-format`, non-text-source callout, config-preservation + absolute-path prose in Spanish, preserving existing content and localized anchors
- [ ] T005 [P] [US1] Reconcile `README_fr.md` (same additions in French)
- [ ] T006 [P] [US1] Reconcile `README_ja.md` (same additions in Japanese)
- [ ] T007 [P] [US1] Reconcile `README_ko.md` (same additions in Korean)
- [ ] T008 [P] [US1] Reconcile `README_pt.md` (same additions in Portuguese)
- [ ] T009 [P] [US1] Reconcile `README_zh.md` (same additions in Chinese)

**Checkpoint**: `scripts/check-readme-parity.sh` passes for all six.

---

## Phase 4: Polish & Cross-Cutting Concerns

**Purpose**: Final validation.

- [ ] T010 Run `git diff --stat -- src/` (assert empty), `cargo test` (regression), and `scripts/check-readme-parity.sh`; confirm each file's TOC anchors match its headings and the language bar links all 7 variants

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS US1.
- **User Story 1 (Phase 3)**: Depends on Foundational (T003).
- **Polish (Phase 4)**: Depends on US1.

### User Story Dependencies

- **US1 (P1)**: single story; the six per-file tasks are independent and parallelizable ([P]).

### Within User Story 1

- Reference content extracted first (T003); then each file reconciled.

### Parallel Opportunities

- T004–T009 touch six distinct files — fully parallel.

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001–T002)
2. Phase 2: Foundational (T003)
3. Phase 3: US1 (T004–T009)
4. **STOP and VALIDATE**: `scripts/check-readme-parity.sh` exit 0

### Incremental Delivery

1. Foundational → parity script + reference content
2. US1 → six translations reconciled
3. Polish → structure/regression validation

---

## Notes

- Preserve existing translations (FR-002) — do not gratuitously re-translate.
- Localized anchors must match each file's own headings (FR-003).
- No Intel-Mac claims (spec 035's decision); no `src/` changes.
