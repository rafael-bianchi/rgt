# Tasks: Spec 051 - Comprehensive Critical Review Remediation & Verification Hardening

**Input**: Feature specification `specs/051-comprehensive-review-remediation/spec.md` and plan `specs/051-comprehensive-review-remediation/plan.md`.

---

## Phase 1: Store, Staleness, Subdirectory Rebasing & Schema Integrity [US1]

- [x] T001 [US1] [FR-002] Implement `record_file_batch` root diffing across path and `(dev, ino)` aliases and skip self-invalidation on `"<stdin>"` in `src/store/queries.rs`.
- [x] T002 [P] [US1] [FR-001] Implement schema v2→v3 migration in `src/store/schema.rs` and pre-DDL `SCHEMA_VERSION` validation in `src/store/db.rs`.
- [x] T003 [US1] [FR-001] Implement `DbStore::find_project_root`, `DbStore::init_in_project` (`src/cli/init.rs`), and `DbStore::normalize_path_for_project` in `src/store/db.rs`.
- [x] T004 [US1] [FR-003] Propagate parent staleness (`any_parent_stale`) to newly created and reused derived nodes in `src/cli/derive.rs` and `src/store/queries.rs`.
- [x] T005 [US1] [FR-001] Enforce `sync_staleness` across `status`, `query`, `derive`, `graph`, `gc`, and MCP `get_status`, and make `delete_obsolete_nodes` iterative in `src/store/queries.rs`.

---

## Phase 2: Value & Date Extraction Hardening [US2]

- [x] T006 [P] [US2] [FR-004] Mask all ISO-8601/RFC-3339 date-shaped spans (including offsets, lowercase `t`/`z`, and invalid calendar dates) in `src/hooks/parser.rs`.
- [x] T007 [US2] [FR-004] Support unspaced CSV rows (`10,200,300`, `10,1.5e10`) while preserving prose millions/billions (`1,000,000`, `$12,345,678`) in `src/hooks/parser.rs`.
- [x] T008 [US2] [FR-004] Add `is_allowed_unit_suffix` and UUID/hex guards in `src/hooks/parser.rs` so `12.5kg` and `99.9ms` extract cleanly while `7e3a9f1`, `0x1a2b`, UUIDs, and `1st` are ignored.
- [x] T009 [US2] [FR-004] Preserve negative signs across currency symbols (`-$1,200.00`, `-€500`), ignore multi-hyphen CLI flags (`--100`), and reject non-finite floats in `src/hooks/parser.rs`.

---

## Phase 3: Verification & Duration Math Hardening [US3]

- [x] T010 [P] [US3] [FR-008] Validate that every bound parent variable (`a`, `b`, ...) appears in `EXPRESSION` via `validate_parent_variable_references` in `src/verify/expression.rs`.
- [x] T011 [US3] [FR-005] Enforce strict relative tolerance (`1e-9 * largest`) and same-sign check in `approx_eq` and return `computed_val` from `verify` in `src/verify/expression.rs`.
- [x] T012 [P] [US3] [FR-008] Preserve non-zero seconds when `days > 0` in `ValueData::Duration::to_string_repr` in `src/types/value.rs`.

---

## Phase 4: 13-Agent Hook/Plugin Protocols, Config Editor & Docker CLI Environment [US4]

- [x] T013 [P] [US4] [FR-009] Support Claude Code nested `tool_response.file.content`, OpenCode `split('\n').count()`, and Hermes post-only plugin hooks in `src/hooks/parser.rs` and `src/hooks/glue/mod.rs`.
- [x] T014 [P] [US4] [FR-009] Validate structural hook entry shape (`shapes_match`) in `merge_array` and preserve Windows double-quoted backslashes in `shell_tokens` in `src/hooks/editor.rs`.
- [x] T015 [P] [US4] [FR-009] Implement scope-aware `rgt init -g`, layered user+project `rgt doctor` resolution, and `.claude/hooks.json.rgt.bak` backup in `src/hooks/installer.rs` and `src/cli/doctor.rs`.
- [x] T016 [P] [US4] [FR-007] Install and verify all 13 agent CLIs (including `mistral-vibe` and `hermes-agent`) in `docker/Dockerfile.harnesses` and verify 58/58 E2E checks in `scripts/test-harnesses-e2e.sh`.

---

## Phase 5: MCP Server, Turtle Export & CI/CD Release Hardening [US5]

- [x] T017 [P] [US5] [FR-006] Implement `src/mcp/mod.rs` (`rgt mcp`) with `execute_record_with_content` for inline content (`src/cli/record.rs`) and `sync_staleness` in `get_status`.
- [x] T018 [P] [US5] [FR-010] Harden `source_location` in `src/export/turtle.rs` and `execute_turtle` in `src/cli/graph.rs` for deleted project-internal paths and `"<stdin>"`.
- [x] T019 [P] [US5] [FR-011] Harden `install.sh` (POSIX compliance, checksum enforcement, atomic `mv -f`), `.github/workflows/cd.yml`, and `.github/workflows/release.yml`.
- [x] T020 [US5] [SC-001] [SC-002] [SC-003] Verify `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`, and Docker E2E harness suite.
