# Tasks: Installer Config Preservation

**Input**: Design documents from `/specs/019-installer-config-preservation/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/, quickstart.md

**Tests**: Included. The project constitution (Principle IX) mandates test coverage for all new/changed user-facing behavior, so each story has test-first tasks written to fail before implementation.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- Single Rust project: `src/`, `tests/` at repository root
- New editor module: `src/hooks/editor.rs` (registered in `src/hooks/mod.rs`)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the two new dependencies and the editor module skeleton

- [x] T001 Add `jsonc-parser` (features = ["cst"]) and `toml_edit` to `[dependencies]` in `Cargo.toml`; run `cargo build` to update `Cargo.lock`
- [x] T002 [P] Create `src/hooks/editor.rs` with the `ConfigEditError` type (carries `path`, `reason`, optional `backup_path`) and a module doc describing the safe-edit guarantees; register `pub mod editor;` in `src/hooks/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The format-preserving editor primitives, backup helper, and install-report types that EVERY user story depends on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational Primitives (write FIRST, expect FAIL) ⚠️

- [x] T003 [P] Unit tests in `tests/unit/hook_editors.rs` asserting: (a) JSONC merge preserves unrelated content byte-for-byte incl. comments/trailing commas; (b) genuinely invalid JSON → `Err`, including a JSONC file that ALSO contains invalid JSON elsewhere; (c) existing `rgt hook` entries are replaced, not duplicated; (d) TOML array ensure preserves comments/formatting and never emits a duplicate `[plugins]`; (e) malformed TOML → `Err`; (f) `text_replace_block` preserves content below the end marker and errors on a legacy block (open marker, no end marker); (g) an RGT marker appearing mid-line or inside a comment is NOT treated as an RGT block; (h) an unexpected `hooks` shape (e.g. object instead of array) → `Err` (unmergeable)

### Implementation for Foundational Primitives

- [x] T004 Implement `parse_jsonc_tolerant(text) -> Result<...>` in `src/hooks/editor.rs` using `jsonc-parser` `parse_to_serde_value`/`parse_to_ast` with comments + trailing commas allowed; return `ConfigEditError` for genuinely unparsable input
- [x] T005 Implement `jsonc_merge_hook_entries(text, sections)` in `src/hooks/editor.rs`: parse tolerant, locate each target section's array span via AST `Ranged` offsets, replace existing entries whose `command` starts with `rgt hook` and insert fresh ones, splice the original text so untouched bytes are verbatim (byte-for-byte, per research.md §1)
- [x] T006 Implement `toml_ensure_array_value(doc, path, value)` and `toml_ensure_table_entry(doc, path, key, value)` in `src/hooks/editor.rs` using `toml_edit` (`or_insert` so `[plugins]` is never duplicated; `Array::push` only when absent)
- [x] T007 Implement `text_replace_block(text, open_marker, end_marker, block)` in `src/hooks/editor.rs`: paired-marker replacement preserving content outside the markers; legacy block (open marker, no end marker) → `ConfigEditError`; absent marker → append block at end
- [x] T008 Implement `backup_before_write(path) -> io::Result<Option<PathBuf>>` in `src/hooks/editor.rs`: copies an existing file to `<path>.rgt.bak` before any modification; no-op when the file is absent; never auto-deletes backups
- [x] T009 Define `InstallReport { outcomes: Vec<AgentOutcome> }` and `AgentOutcome::{Configured{agent, artifact}, Skipped{agent}, Failed{agent, reason}}` in `src/hooks/installer.rs`

**Checkpoint**: `cargo test test_hook_editors` passes; `InstallReport` compiles

---

## Phase 3: User Story 1 - Existing User Configuration Is Never Destroyed (Priority: P1) 🎯 MVP

**Goal**: JSON-family (Claude Code, Cursor, Copilot Chat) and TOML (Hermes, Gemini, Vibe) writers merge RGT's entries additively; every non-RGT entry preserved byte-for-byte (FR-001/FR-002, SC-001)

**Independent Test**: Contract tests populate configs with a foreign hook group + unrelated settings, run the affected writer, and assert non-RGT content is byte-identical after install

### Tests for User Story 1 (write FIRST, expect FAIL) ⚠️

- [x] T010 [P] [US1] Contract tests in `tests/contract/hook_installer.rs`: (a) Claude Code `settings.json` with an RTK-style `PostToolUse` group + `keybindings` preserved after `write_claude_code`; (b) Cursor `hooks.json` pre-existing entries preserved; (c) Copilot Chat `github.copilot.chat.hooks` foreign entries preserved; (d) Hermes `config.toml` keeps other `plugins.enabled` entries and one `[plugins]` table

### Implementation for User Story 1

- [x] T011 [US1] Rewrite `write_claude_code` in `src/hooks/installer.rs` to merge via `jsonc_merge_hook_entries` for `hooks.PostToolUse` (`rgt hook post`) and `hooks.PreToolUse` (`rgt hook pre`); remove the `existing["hooks"] = rgt_hooks;` whole-key assignment. Trigger: merge when `--force` OR RGT entries absent (→ `Configured`); when RGT entries present, do nothing (→ `Skipped`). This changes the current `force || !settings_file.exists()` skip so an existing `settings.json` without RGT hooks gets configured on a plain `rgt init` (US1-AS1).
- [x] T012 [US1] Rewrite `write_cursor` in `src/hooks/installer.rs` to merge into the existing `hooks.json` `postToolUse`/`preToolUse` arrays (`{command, matcher}` shape); never rewrite the whole file. Trigger: merge when `--force` OR RGT entries absent (→ `Configured`); RGT entries present → `Skipped` (no write)
- [x] T013 [US1] Rewrite the Copilot path in `src/hooks/installer.rs` (`write_json_key` usage for `github.copilot.chat.hooks`) to merge `PostToolUse` via `jsonc_merge_hook_entries` (`rgt hook post --agent copilot`); preserve foreign groups. Trigger: merge when `--force` OR RGT entries absent (→ `Configured`); RGT entries present → `Skipped` (no write)
- [x] T014 [US1] Replace `enable_hermes_plugin`/`insert_into_enabled` in `src/hooks/installer.rs` with the TOML editor: `doc.entry("plugins").or_insert(table())` then ensure `"rgt"` in `plugins.enabled`. Trigger: edit when `--force` OR `"rgt"` absent from `plugins.enabled` (→ `Configured`); present → `Skipped` (no write)
- [x] T015 [US1] Rewrite `write_gemini` and `write_vibe` in `src/hooks/installer.rs` to use the TOML editor for `hooks.toml` (Gemini `PostToolUse.command` contains `rgt hook post --agent gemini`; Vibe `[[pre_tool]]` entry with `rgt hook post --agent vibe`); if the target table/array exists with a conflicting non-RGT entry, return `ConfigEditError` (cannot duplicate the table) instead of emitting invalid TOML. Trigger: edit when `--force` OR the RGT entry is absent (→ `Configured`); present → `Skipped` (no write)

**Checkpoint**: preservation contract tests (T010) pass; non-RGT content verified byte-for-byte

---

## Phase 4: User Story 2 - Malformed User Configuration Fails Loudly, Never Silently (Priority: P1)

**Goal**: No silent `{}` fallback on unparsable files; per-agent error isolation; backup before destructive writes; non-zero exit; JSONC tolerated (FR-003/FR-004/FR-008/FR-009/FR-010, SC-002/SC-006)

**Independent Test**: Contract tests feed genuinely invalid configs (truncated JSON, invalid TOML) and assert the file is untouched, an error names the file, and the run exits non-zero; JSONC files are merged instead

### Tests for User Story 2 (write FIRST, expect FAIL) ⚠️

- [x] T016 [P] [US2] Contract tests in `tests/contract/hook_installer.rs`: (a) genuinely invalid JSON/TOML → file byte-identical + `ConfigEditError` naming the file, including a config path that is a directory; (b) JSONC (comments/trailing commas) → merged successfully with comments preserved; (c) malformed file for one agent while others are healthy → healthy agents still configured, failed one reported, overall failure surfaced; (d) a `<path>.rgt.bak` exists when a write was attempted

### Implementation for User Story 2

- [x] T017 [US2] Remove the silent fallbacks in `src/hooks/installer.rs` — `unwrap_or_default()`/`unwrap_or(json!({}))` on read+parse in `write_claude_code` and `write_json_key`, and any `unwrap_or`-style read fallbacks introduced by the rewritten `write_cursor` (T012) and TOML writers (T014/T015) — and propagate `ConfigEditError` (file path + reason) from every writer that reads an existing file
- [x] T018 [US2] Refactor `configure()` in `src/hooks/installer.rs` to collect `AgentOutcome` per targeted agent instead of `?` short-circuit; update `detect_and_configure_hooks`, `detect_and_configure_hooks_in_home`, `detect_and_configure_hooks_with_config` to return `InstallReport`
- [x] T019 [US2] Update `execute_init` in `src/cli/init.rs` to print per-agent outcomes (configured/skipped/failed with backup hint) and return `Err(...)` (exit 1) when any agent `Failed`; the unknown-`--agent` exit-2 path is unchanged
- [x] T020 [US2] Wire `backup_before_write` into every writer in `src/hooks/installer.rs` before modifying an existing file; failure messages must include the backup path when one exists

**Checkpoint**: error-path contract tests (T016) pass; `rgt init` reports partial failure and exits 1

---

## Phase 5: User Story 3 - Re-Init Is Safe: No Duplication, No Truncation (Priority: P1)

**Goal**: `rgt init` without `--force` is an idempotent no-op; `--force` replaces only RGT's delimited block; no whole-file overwrites (Codex, Windsurf); output always valid (FR-005/FR-006/FR-007, SC-003/SC-004/SC-005)

**Independent Test**: Integration tests write an RGT block plus trailing user content, re-run with `--force`, and assert the block refreshes while the trailing content survives; a second non-`--force` run writes nothing

### Tests for User Story 3 (write FIRST, expect FAIL) ⚠️

- [x] T021 [P] [US3] Integration tests in `tests/integration/test_hooks_installer.rs`: (a) re-run without `--force` writes nothing; (b) `--force` on a rules file preserves content below the RGT block; (c) Codex `AGENTS.md` user content preserved on `--force`; (d) Windsurf `.windsurfrules` user content preserved on `--force`; (e) Hermes `config.toml` re-parses as valid TOML after re-init

### Implementation for User Story 3

- [x] T022 [US3] Add `RGT_END_MARKER` (`<!-- /RGT Integration -->`) to `src/hooks/glue/mod.rs` and append an end-marker line to every block constant (`CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, `VIBE_PROMPT`, `COPILOT_CLI_RULES`) and to `codex_section()` in `src/hooks/installer.rs`
- [x] T023 [US3] Replace `write_text_block`/`truncate_from_marker` usage in `src/hooks/installer.rs` with `text_replace_block` for cline/antigravity/kilocode/copilot-cli and the Vibe `rgt.md` prompt; legacy-block detection surfaces a `ConfigEditError` with a manual-remediation hint
- [x] T024 [US3] Rewrite `write_codex` in `src/hooks/installer.rs` to merge/replace only the RGT section in `AGENTS.md` (paired markers); never whole-file overwrite or blind append on `--force`
- [x] T025 [US3] Rewrite `write_windsurf` in `src/hooks/installer.rs` to use the text-block merge instead of whole-file `fs::write` on `--force`
- [x] T026 [US3] Remove `write_toml_block`, `truncate_from_marker`, `join_base_block`, and `insert_into_enabled` from `src/hooks/installer.rs` if now unused after T014/T015/T023

**Checkpoint**: integration tests (T021) pass; `rgt init --force` never truncates user content

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, test reconciliation, and final validation

- [x] T027 [P] Update `README.md` (Quick Start / `rgt init` section) and `AGENTS.md` to document the preservation guarantees, JSONC tolerance, backups (`.rgt.bak`), and exit codes (0/1/2)
- [x] T028 [P] Reconcile outdated 018-era assertions in `tests/contract/hook_installer.rs` that expected byte-identical installer output; adjust them to the new merge semantics
- [x] T029 [P] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets`, and the full `cargo test` suite; fix all findings
- [x] T030 Run the 5 manual validation scenarios in `specs/019-installer-config-preservation/quickstart.md` and confirm the expected outcomes

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational — MVP
- **User Story 2 (Phase 4)**: Depends on Foundational + US1 (T017/T020 touch writers changed in T011–T013; T018 depends on T009)
- **User Story 3 (Phase 5)**: Depends on Foundational + US1 (T014/T015) + US2 (T018 error model); T023 depends on T022
- **Polish (Phase 6)**: Depends on all user stories

### User Story Dependencies

- **US1 (P1)**: after Foundational — merge/preserve writers
- **US2 (P1)**: after US1 — error propagation + reporting layered on the merged writers
- **US3 (P1)**: after US2 — text-block/re-init behavior; independent file surface (`glue/mod.rs`, rules files) but shares `installer.rs`

### Within Each User Story

- Tests written first, must FAIL before implementation
- Editor primitives before writers (Foundational); writers before reporting (US2); markers before block replacement (US3)
- Checkpoint at the end of each phase validates the story independently

### Parallel Opportunities

- T001 ∥ T002 (Setup, different files)
- T003 test-only tasks run before the Foundational impl tasks
- Story test tasks (T010, T016, T021) each run ahead of their phase's impl and are mutually independent (different test files or non-overlapping fixtures)
- T027/T028/T029 (Polish) can run in parallel

---

## Parallel Example: Foundational Phase

```bash
# Tests for editor primitives first (expect FAIL):
Task: "Unit tests in tests/unit/hook_editors.rs"

# Then implementation (sequential — one module, ordered by dependency):
Task: "parse_jsonc_tolerant in src/hooks/editor.rs"
Task: "jsonc_merge_hook_entries in src/hooks/editor.rs"
Task: "toml_ensure_* in src/hooks/editor.rs"
Task: "text_replace_block in src/hooks/editor.rs"
Task: "backup_before_write in src/hooks/editor.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (deps + editor.rs skeleton)
2. Complete Phase 2: Foundational (editor primitives + unit tests — CRITICAL, blocks everything)
3. Complete Phase 3: User Story 1 (merge/preserve writers)
4. **STOP and VALIDATE**: preservation contract tests green; non-RGT content byte-for-byte preserved
5. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → safe-edit primitives in place
2. US1 → existing config never destroyed (MVP)
3. US2 → malformed configs fail loudly, backups, correct exit codes
4. US3 → `--force` re-init is safe and idempotent
5. Polish → docs + full-suite validation per quickstart.md

---

## Notes

- [P] tasks = different files, no dependencies; the JSON/TOML/text writer tasks within US1/US3 are intentionally NOT parallel because they all edit `src/hooks/installer.rs`
- [Story] label maps task to user story for traceability
- Constitution Principle IX mandates the test-first tasks included here; Principle XI requires `cargo fmt`/`clippy` to pass (T029)
- Commit after each task or logical group
- Stop at each checkpoint to validate the story independently

---

## Phase 7: Convergence

- [x] T031 Attach the config file path to `ConfigEditError` before it surfaces from `ensure_json_hooks`/`ensure_toml`/`ensure_text_block` in `src/hooks/installer.rs`, so `rgt init` failure reasons name the offending file (e.g. `settings.json`), per FR-009 / US2-AC1 (partial)
