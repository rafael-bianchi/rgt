# Tasks: Expand Agent Hook Coverage to Match RTK

**Input**: Design documents from `specs/018-expand-agent-hooks/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Test tasks ARE included — FR-008 and constitution IX require test coverage for each new integration and all fail-open paths.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story. US1/US2/US3 share the Foundational phase (agent registry, `rgt hook --agent`, normalizers, glue module); US4 depends on US1 (documents the accepted names).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Establish the current baseline before changes (SC-003 requires byte-identical behavior for the existing 4 agents)

- [X] T001 Record the exact current artifact output for the 4 existing agents in `src/hooks/installer.rs` (Claude Code `settings.json` hooks block, Cursor `hooks.json`, Codex `AGENTS.md` section, Windsurf `.windsurfrules`) as the byte-identical baseline for SC-003
- [X] T002 Confirm `rgt hook pre/post` default stdin format and call path: `Hook { event, agent }` in `src/main.rs`, `parse_hook_payload` in `src/hooks/parser.rs`, `handle_passive_hook_event` in `src/hooks/mod.rs`
- [X] T003 Record current test layout (`tests/unit/`, `tests/contract/`) and confirm `cargo test` passes on the untouched tree

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure required by US1/US2/US3 — the `--agent` CLI flag, the 13-agent registry, per-agent normalizers, and embedded glue templates

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Add optional `--agent <name>` argument to the `Hook` subcommand in `src/main.rs` and pass it through to `handle_passive_hook_event`; when omitted, behavior MUST stay identical to today (default Claude Code format)
- [X] T005 Implement the 13-agent registry inside `src/hooks/installer.rs`: canonical names (`claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`), aliases (`claude`↔`claude-code`, `cline`↔`roo-code`, `kilo`↔`kilocode`), tier (`hook`/`plugin`/`rules`), input format, config path, and detection trigger per `data-model.md`
- [X] T006 [P] Create `src/hooks/glue/mod.rs` with embedded const-string templates: TS plugin (OpenCode), TS extension (Pi), Python plugin (Hermes), rules files (Cline, Antigravity, Kilo), and Vibe system prompt — each ONLY invokes `rgt hook post --agent <name>` (thin delegates, constitution VIII; post-only: provenance records the post-tool file state)
- [X] T007 Add per-agent stdin normalizers to `src/hooks/parser.rs` mapping native event JSON → `{path, content}`: `copilot` (dual snake_case / camelCase with JSON-stringified `toolArgs`), `gemini` (`run_shell_command`), `vibe` (`pre_tool`, `tool_input.command`), `opencode`, `pi`, `hermes` per `contracts/agent-hooks.md` dialect table
- [X] T008 Add disk-read content fallback in `handle_passive_hook_event` in `src/hooks/mod.rs`: when `content` is absent but `path` resolves to an existing file, read the file from disk as content
- [X] T009 [P] Unit-test every normalizer in `tests/unit/hook_parsers.rs`: each dialect parses a representative event, unknown/malformed JSON yields a no-op (empty result), and no normalizer ever returns an error that propagates

**Checkpoint**: `cargo test` passes with the new flag, registry, normalizers, and glue module; `rgt hook post` without `--agent` still matches pre-change behavior

---

## Phase 3: User Story 1 - Every RTK-Supported Agent Can Initialize RGT (Priority: P1) 🎯 MVP

**Goal**: `rgt init --agent <name>` accepts all 13 RTK agents (US1, FR-001/FR-002/FR-006) and writes the agent-native hook config.

**Independent Test**: `rgt init --agent <name> --force` succeeds for each of the 13 names and writes the expected artifact (quickstart Scenarios 1, 2, 6, 7); existing 4 configs are byte-identical (SC-003).

### Implementation for User Story 1

- [X] T010 [US1] Add Copilot hook-config writer in `src/hooks/installer.rs`: VS Code Copilot Chat user settings (`github.copilot.chat.hooks`) invoking `rgt hook post --agent copilot`; detect both VS Code Chat and Copilot CLI
- [X] T011 [US1] Add Gemini hooks writer in `src/hooks/installer.rs`: `~/.gemini/hooks.toml` entry invoking `rgt hook post --agent gemini`
- [X] T012 [US1] Add Mistral Vibe writer in `src/hooks/installer.rs`: `~/.vibe/hooks.toml` `pre_tool` entry (`match="bash"`, `strict=false`) invoking `rgt hook post --agent vibe`, plus `~/.vibe/prompts/rgt.md`
- [X] T013 [US1] Add OpenCode TS plugin writer in `src/hooks/installer.rs`: writes the embedded `rgt.ts` glue to `~/.config/opencode/plugins/` (project scope: `.opencode/plugin/`)
- [X] T014 [US1] Add Pi TS extension writer in `src/hooks/installer.rs`: writes the embedded `rgt.ts` glue to `.pi/extensions/` (project) or `~/.pi/agent/extensions/` (`--global`)
- [X] T015 [US1] Add Hermes Python plugin writer in `src/hooks/installer.rs`: writes the embedded Python glue under `~/.hermes/plugins/rgt/` and enables it via `plugins.enabled` in the Hermes config
- [X] T016 [US1] Add Cline/Roo Code rules writer in `src/hooks/installer.rs`: appends the RGT section to `.clinerules` (project)
- [X] T017 [US1] Add Antigravity rules writer in `src/hooks/installer.rs`: writes the embedded rules to `.agents/rules/antigravity-rgt-rules.md` (project)
- [X] T018 [US1] Add Kilo rules writer in `src/hooks/installer.rs`: writes the embedded rules to `.kilocode/rules/rgt-rules.md` (project)
- [X] T019 [US1] Implement `--agent` validation in `src/hooks/installer.rs` (or `src/cli/init.rs`): unknown names exit code 2 with all valid `--agent` values listed (13 canonical + aliases `claude`, `roo-code`, `kilo` — 16 spellings) (FR-001); aliases resolve to canonical names
- [X] T020 [US1] Enforce idempotency and `--force` semantics across all new writers in `src/hooks/installer.rs`: re-runs produce identical content, RGT-marked sections replaced in place, user-authored content never clobbered without `--force` (FR-006)
- [X] T021 [P] [US1] Contract-test every new artifact in `tests/contract/hook_installer.rs`: exact file content written per agent, idempotent re-run, `--force` overwrite, and byte-identical output for the existing 4 agents (SC-003)

**Checkpoint**: 13 `--agent` names accepted; all 13 artifacts written; existing 4 configs byte-identical

---

## Phase 4: User Story 2 - One-Command Setup Configures Every Installed Agent (Priority: P1)

**Goal**: `rgt init` without `--agent` detects and configures every installed supported agent in one run (US2, FR-003).

**Independent Test**: Emulate 2+ agent config dirs, run `rgt init`, confirm all detected agents configured and listed; zero agents installed → database init succeeds with "no hook configs written" (quickstart Scenario 3).

### Implementation for User Story 2

- [X] T022 [US2] Add per-agent detection triggers in `src/hooks/installer.rs` (e.g., `~/.vibe/hooks.toml`, `.kilocode/rules/`, `~/.config/opencode/`, `.clinerules`, VS Code settings for Copilot, `~/.gemini/hooks.toml`, `~/.hermes/plugins/`, `.pi/extensions/`, `.agents/rules/`) per `data-model.md` detection_trigger
- [X] T023 [US2] Extend the no-`--agent` loop in `src/hooks/installer.rs` to configure every detected agent (all 13, including `plugin` and `rules` tiers) and list each configured entry in `execute_init` output in `src/cli/init.rs`; when nothing is detected, still succeed with the "No new AI coding tool hook configs written" message (US2 AC2)
- [X] T024 [P] [US2] Unit-test auto-detection in `tests/unit/` (e.g., `tests/unit/detection.rs`): each trigger path → detected; no paths → empty result without error; detection respects `--global` vs project scope
- [X] T024b [P] [US2] Add cross-platform path-resolution unit tests in `tests/unit/` (e.g., `tests/unit/paths.rs`): assert Copilot user-settings path resolves to `%APPDATA%` on Windows and `~/.config` on Linux/macOS, and each of the 13 `config_path` targets resolves correctly per `std::env::consts::OS` (constitution cross-platform parity; contracts `agent-hooks.md` L92)

**Checkpoint**: Single `rgt init` configures all installed supported agents; empty-detection path is non-erroring

---

## Phase 5: User Story 3 - New Hooks Never Block the Agent (Priority: P2)

**Goal**: Every hook (existing + new) fails open — missing binary, malformed JSON, or absent runtime never blocks or alters the agent's tool call (US3, FR-005).

**Independent Test**: Pipe malformed JSON and empty payloads to `rgt hook post --agent <each>` and assert exit 0 with no stdout; verify plugin glue no-ops when the RGT binary/runtime is missing (quickstart Scenario 4).

### Implementation for User Story 3

- [X] T025 [US3] Audit `handle_passive_hook_event` in `src/hooks/mod.rs` and the new normalizers in `src/hooks/parser.rs`: every error path returns `Ok(())` with empty stdout (fail-open), and no path ever writes to stdout on failure
- [X] T026 [US3] Add fail-open guards to the embedded glue in `src/hooks/glue/mod.rs`: TS plugins (OpenCode, Pi) and the Python plugin (Hermes) skip registration/return no-op when `rgt` is missing from PATH or the runtime is absent (mirrors RTK's contract, quickstart Scenario 6)
- [X] T027 [P] [US3] Unit-test all fail-open paths in `tests/unit/hook_parsers.rs` and `tests/contract/hook_installer.rs`: malformed JSON, empty payload, non-existent path, missing binary/runtime → exit 0, no output, command unmodified (SC-004)

**Checkpoint**: 100% of hook failure paths verified non-blocking by automated tests

---

## Phase 6: User Story 4 - Documentation Lists All Supported Agents (Priority: P3)

**Goal**: The documented supported-agent list matches the `--agent` accept list exactly, including aliases (US4, FR-001, SC-005).

**Independent Test**: `rgt init --help` and README/AGENTS.md list the same 16 accepted values (13 canonical + aliases); every documented name is accepted by the CLI (quickstart Scenario 8).

### Implementation for User Story 4

- [X] T028 [US4] Update the `Init` subcommand help text in `src/main.rs`: document all 13 agents (13 canonical names + 3 aliases `claude`, `roo-code`, `kilo`), tiers, and `--agent` usage
- [X] T029 [P] [US4] Update README.md: supported-agents table (13 agents), `rgt init --agent` examples, and a note that hooks are provenance-capture only (no command rewriting)
- [X] T030 [P] [US4] Update AGENTS.md RGT Integration section to list the full supported-agent set
- [X] T031 [P] [US4] Update the module doc comment in `src/hooks/installer.rs` to reflect the 13-agent coverage

**Checkpoint**: Documented list and `--agent` accept list match exactly (SC-005)

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Full validation, quality gates, and end-to-end verification

- [X] T032 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and `cargo test` on the completed tree (constitution XI / IX)
- [X] T033 Run quickstart.md validation scenarios 1-8 end-to-end (`rgt init --agent` matrix, unknown-agent rejection, auto-detection, fail-open pipes, plugin artifacts, rules files, doc parity)
- [X] T034 Re-verify SC-003: diff the Claude Code/Cursor/Codex/Windsurf artifacts produced by this build against the Phase 1 baseline (T001) — must be byte-identical

**Checkpoint**: All CI-quality gates green; quickstart scenarios pass; no regression on existing integrations

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — establishes baseline
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS US1, US2, US3
- **US1 (Phase 3)**: Depends on Foundational
- **US2 (Phase 4)**: Depends on Foundational (registry + detection triggers). Independent of US1 implementation (different writer code in same file — apply sequentially to avoid edit conflicts)
- **US3 (Phase 5)**: Depends on Foundational (normalizers + glue). Independent of US1/US2
- **US4 (Phase 6)**: Depends on US1 (documents the accepted names)
- **Polish (Phase 7)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependency on other stories (MVP)
- **US2 (P1)**: Can start after Foundational — needs detection triggers from the registry (T005)
- **US3 (P2)**: Can start after Foundational — needs normalizers (T007) and glue (T006)
- **US4 (P3)**: Needs US1 complete (documents the accepted agent names)

### Within Each User Story

- Registry (T005) before writers (T010-T018)
- Writers before validation (T019, T020) and contract tests (T021)
- Fail-open audit (T025) after normalizers; glue guards (T026) after glue module

### Parallel Opportunities

- T006 (glue module) runs in parallel with T005 (registry) — different files
- T009 (normalizer tests) can run once T007 lands — different file
- T021, T024, T027 are [P] (separate test files, no cross-deps)
- T029, T030, T031 (docs) run in parallel — different files
- US2/US3 writers share `src/hooks/installer.rs` with US1 writers — apply sequentially to avoid edit conflicts; no longer marked [P]

---

## Parallel Example: US1 + US2 + US3

```bash
# Developer A: US1 (agent writers in src/hooks/installer.rs)
Tasks: T010, T011, T012, T013, T014, T015, T016, T017, T018, T019, T020

# Developer B: US2 (auto-detection in src/hooks/installer.rs + src/cli/init.rs)
Tasks: T022, T023

# Developer C: US3 (fail-open in src/hooks/mod.rs + src/hooks/parser.rs + glue)
Tasks: T025, T026
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T009)
3. Complete Phase 3: US1 (T010-T021) — all 13 `--agent` names accepted with artifacts
4. **STOP and VALIDATE**: run quickstart Scenarios 1, 2, 6, 7; confirm SC-003 byte-identical baseline

### Incremental Delivery

1. Setup → baseline confirmed
2. Foundational → `--agent` flag + registry + normalizers + glue
3. US1 → all 13 agents accept and configure (MVP)
4. US2 → one-command auto-detection
5. US3 → fail-open verified for every hook
6. US4 → docs parity
7. Polish → quickstart end-to-end, CI gates green

### Parallel Team Strategy

With three developers:

1. Team completes Setup + Foundational together
2. Developer A: US1 (writers), Developer B: US2 (detection), Developer C: US3 (fail-open) — coordinate on `src/hooks/installer.rs` edits sequentially
3. US4 (docs) after US1
4. Polish validates the full pipeline

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Existing 4 agent configs MUST stay byte-identical (SC-003) — baseline recorded in T001
- Hooks are provenance-capture only; no command rewriting (clarified scope)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently

---

## Phase 8: Convergence

- [X] T035 Add a Copilot CLI hook-config writer and detection trigger in `src/hooks/installer.rs` (Copilot CLI config path per host platform) so `rgt init` detects and configures Copilot CLI in addition to VS Code Chat, per T010 and research.md Copilot mechanism (partial)
