# Tasks: Init Observability

**Input**: Design documents from `/specs/028-init-observability/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/doctor-invariants.md, quickstart.md

**Tests**: Included — spec "Independent Test" per story + Constitution IX require coverage.

**Organization**: Tasks grouped by user story (US1–US5).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm baseline before any changes.

- [x] T001 Verify baseline: run `cargo build` and `cargo test --test test_hook_installer_contract --test test_hook_parsers` and confirm they pass; confirm no new dependencies (uses `std::env::current_exe()`, per research.md §2).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types and helpers every story depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T002 [P] Add `Diagnostic { name, status, message }` and `DiagnosticStatus { Healthy, Warning, Error }` types plus an `execute_doctor` skeleton returning `Vec<Diagnostic>` in new `src/cli/doctor.rs`
- [x] T003 [P] Add `list_hook_artifacts(home, config_dir) -> Vec<(agent, PathBuf)>` helper (existence + RGT-marker check) in `src/hooks/installer.rs`
- [x] T004 Register new test target `test_doctor_contract` (path `tests/contract/test_doctor_contract.rs`) in `Cargo.toml`

**Checkpoint**: Types/helpers compile; new test target is wired.

---

## Phase 3: User Story 1 - Diagnose a Silent Capture Failure (Priority: P1) 🎯 MVP

**Goal**: `rgt doctor` distinguishes errors (not initialized → exit 1) from warnings ("hooks present, zero captures" → PATH hint, exit 0).

**Independent Test**: `cargo test --test test_doctor_contract` — asserts the exit-code contract.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T005 [US1] Contract tests for the doctor exit-code/diagnostics truth table in `tests/contract/test_doctor_contract.rs` (not initialized → error/exit 1; hooks+values → healthy/0; hooks+zero values → PATH warning/0; no hooks → warning/0)

### Implementation for User Story 1

- [x] T006 [US1] Implement `execute_doctor` checks (store-initialized, hooks-present, nodes-recorded → `Diagnostic` severity) in `src/cli/doctor.rs`
- [x] T007 [US1] Wire the `Doctor` subcommand in `src/main.rs` (map any `Error` diagnostic to exit code 1, warnings to 0)

**Checkpoint**: `rgt doctor` runs; `cargo test --test test_doctor_contract` green.

---

## Phase 4: User Story 2 - Hooks Reference the CLI by Absolute Path (Priority: P1)

**Goal**: Subprocess-spawning hooks invoke the CLI by absolute path (no PATH dependency).

**Independent Test**: `cargo test --test test_hook_installer_contract` — asserts the written commands reference an absolute path.

### Tests for User Story 2

- [x] T008 [US2] Contract test in `tests/contract/hook_installer.rs` asserting the eight subprocess agents (claude-code, cursor, copilot, gemini, vibe, opencode, pi, hermes) write an absolute path, not the bare `rgt` name

### Implementation for User Story 2

- [x] T009 [US2] Thread the `std::env::current_exe()` path through `configure()` into the `write_claude_code`/`write_cursor`/`write_copilot`/`write_gemini`/`write_vibe` command hooks in `src/hooks/installer.rs` (add a testable path-injection variant of the configure entry point)
- [x] T010 [US2] Convert `OPENCODE_TS_PLUGIN`/`PI_TS_EXTENSION`/`HERMES_PYTHON_PLUGIN` constants into path-injected format functions in `src/hooks/glue/mod.rs` and update their `write_*` callers

**Checkpoint**: `cargo test --test test_hook_installer_contract` green; rules-file agents untouched.

---

## Phase 5: User Story 3 - Hook Matcher Fires Only on Relevant Tools (Priority: P2)

**Goal**: Claude Code matcher scoped to `Read|Edit|Write|Bash` (Bash preserved for 026).

**Independent Test**: `cargo test --test test_hook_installer_contract` — asserts the matcher value.

### Tests for User Story 3

- [x] T011 [US3] Contract test in `tests/contract/hook_installer.rs` asserting the Claude Code `matcher` equals `Read|Edit|Write|Bash` (not `""`)

### Implementation for User Story 3

- [x] T012 [US3] Change `write_claude_code` matcher from `""` to `Read|Edit|Write|Bash` (both PostToolUse and PreToolUse) in `src/hooks/installer.rs`

**Checkpoint**: matcher scoped; Bash-read capture (026) unaffected.

---

## Phase 6: User Story 4 - Init Reminds the User to Restart (Priority: P2)

**Goal**: `rgt init` prints a restart reminder only when it newly configures a hook.

**Independent Test**: `cargo test --test test_hook_installer_contract` — asserts the gating helper.

### Tests for User Story 4

- [x] T013 [US4] Unit test in `tests/contract/hook_installer.rs` for `needs_restart_reminder(&InstallReport)` (true when any outcome is `Configured`, false when all `Skipped`)

### Implementation for User Story 4

- [x] T014 [US4] Extract `needs_restart_reminder` helper and print the restart line from `execute_init` in `src/cli/init.rs`

**Checkpoint**: reminder printed on first configure, omitted on no-op re-run.

---

## Phase 7: User Story 5 - Copilot CLI Dialect Assumption Is Locked In (Priority: P3)

**Goal**: Pin the assumed Copilot CLI dialect shape with a test.

**Independent Test**: `cargo test --test test_hook_parsers` — asserts the dialect behavior for assumed shapes.

### Tests for User Story 5

- [x] T015 [US5] Unit test in `tests/unit/hook_parsers.rs` pinning `copilot_dialect` for the assumed shapes (VS Code Chat snake_case, CLI `toolArgs`-as-JSON-string, `command_dialect` fallback)

**Checkpoint**: assumption locked by a test; no dialect logic change.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation and final validation.

- [x] T016 Document `rgt doctor`, the absolute-path caveat (re-run `rgt init` after a relocated/upgraded binary), and the Copilot CLI assumption in `README.md` and `AGENTS.md` (Constitution X)
- [x] T017 Run quickstart.md scenarios, then `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and full `cargo test`; fix any regressions

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS all user stories.
- **User Stories (Phase 3–7)**: Depend on Foundational.
- **Polish (Phase 8)**: Depends on all desired stories.

### User Story Dependencies

- **US1 (P1)**: after Foundational (T002/T003/T004); no other-story deps.
- **US2 (P1)**: after Foundational; independent of US1 (both touch `installer.rs` — see file-coordination note).
- **US3 (P2)**: after US2 (same file `installer.rs`).
- **US4 (P2)**: after Foundational; independent (edits `init.rs`).
- **US5 (P3)**: after Foundational; independent (edits `hook_parsers.rs`).

### Within Each User Story

- Tests written first (fail), then implementation.
- Types/helpers before the code that consumes them.

### Parallel Opportunities

- T002/T003 (different files) in parallel.
- US1 (T005–T007) can proceed in parallel with US2 (T008–T010) once Foundational is done — they touch `doctor.rs`/`main.rs` vs `installer.rs`/`glue/mod.rs`.
- US4 (T013–T014) and US5 (T015) are independent and can run in parallel with US1/US2/US3.

### File-Coordination Notes

- `src/hooks/installer.rs` is edited by T003, T009, T012 — keep them sequential in that order.
- `tests/contract/hook_installer.rs` gains test fns from T008, T011, T013 — sequential to avoid edit conflicts.
- `src/hooks/glue/mod.rs` (T010) is distinct from `installer.rs`, but T010's callers live in `installer.rs` (T009) — run T009 before T010.

---

## Parallel Example: US1 + US2 (after Foundational)

```bash
# Doctor command path (touches src/cli/doctor.rs + src/main.rs):
Task: "Contract tests for doctor exit codes in tests/contract/test_doctor_contract.rs"

# Absolute-path path (touches src/hooks/installer.rs + src/hooks/glue/mod.rs):
Task: "Contract test asserting subprocess hooks use absolute paths in tests/contract/hook_installer.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1: Setup (T001)
2. Phase 2: Foundational (T002–T004) — CRITICAL
3. Phase 3: US1 doctor (T005–T007)
4. **STOP and VALIDATE**: `cargo test --test test_doctor_contract` — silent capture failure is now diagnosable

### Incremental Delivery

1. Foundational → compile
2. US1 → `rgt doctor` (MVP)
3. US2 → absolute-path hooks (prevention)
4. US3 → matcher scoped
5. US4 → restart reminder
6. US5 → Copilot dialect pinned
7. Polish → docs + fmt/clippy/full test

---

## Notes

- `installer.rs`/`hook_installer.rs` are the two hottest files — keep their tasks sequential.
- Absolute path is only for the eight subprocess agents; the five instructions-only agents (codex, windsurf, cline, antigravity, kilocode) are instructions, not spawns.
- US5 is test-only (no dialect logic change), per FR-006.
- Commit after each task or logical group; stop at checkpoints to validate independently.
