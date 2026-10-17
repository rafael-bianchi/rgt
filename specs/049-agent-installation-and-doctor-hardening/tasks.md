# Tasks: Agent Installation, Guidance, and Doctor Diagnostics Hardening

**Input**: Feature specification `specs/049-agent-installation-and-doctor-hardening/spec.md` and plan `specs/049-agent-installation-and-doctor-hardening/plan.md`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: Parallelizable task.
- **[Story]**: [US1] through [US4] mapping to spec.md.

---

## Phase 1: Setup & Tests First

- [x] T001 [US1] Add contract tests in `tests/contract/hook_installer.rs` verifying that `rgt init -g` writes to user home directories and project `rgt init` writes to project directories for Copilot, Gemini, Vibe, Codex, Cursor, Windsurf, Cline, Antigravity, and Kilo.
- [x] T002 [US2] Add unit tests in `tests/unit/detection.rs` asserting that `detect_installed_agents` does not falsely detect Copilot on bare `settings.json`, Codex on bare `AGENTS.md`, or Antigravity on bare `.agents/`.
- [x] T003 [US3] Add contract test in `tests/contract/test_doctor_contract.rs` asserting that having both user and project registrations for an agent reports `Active` and `rgt doctor` exits 0.
- [x] T004 [US4] Add contract test in `tests/contract/hook_installer.rs` asserting that `rgt init --force` overwrites an existing conflicting rule block.

---

## Phase 2: Implementation

- [x] T005 [US1] In `src/hooks/paths.rs`, add scope-aware path functions for guidance and hook files for all agents.
- [x] T006 [US1] In `src/hooks/installer.rs`, update `write_copilot`, `write_gemini`, `write_vibe`, `write_cursor`, `write_codex`, `write_windsurf`, `write_cline`, `write_antigravity`, and `write_kilocode` to strictly respect `global`.
- [x] T007 [US2] In `src/hooks/installer.rs`, refine `detect_installed_agents` triggers to prevent false positives and add Windsurf user directory detection.
- [x] T008 [US3] In `src/hooks/registration.rs`, update `inspect_surface` so that coexisting project and user scopes resolve to project precedence with `Active` health.
- [x] T009 [US4] In `src/hooks/installer.rs` and `src/hooks/editor.rs`, support `force: bool` in `ensure_json_hooks` and `write_cursor`.

---

## Phase 3: Verification & Parity

- [x] T010 Run the complete test suite (`cargo test </dev/null`) and verify all tests pass.
- [x] T011 Run `./scripts/check-readme-parity.sh`, `./scripts/check-test-presence.sh`, and `./scripts/check-version-state.sh`.
