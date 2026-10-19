# Tasks: Spec 053 - Pass-3 Critical Review Remediation

**Input**: Feature specification `specs/053-critical-review-remediation/spec.md` and plan `specs/053-critical-review-remediation/plan.md`.

---

## Phase 1: Provenance Store State-Machine, Line-Shift Reconciliation & Turtle Uninitialized Export [US1]

- [x] T001 [US1] [FR-001] Fix `record_file_batch` (`src/store/queries.rs`) so files modified and marked stale by `sync_staleness` still reconcile, un-stale matched roots (`is_stale = false, stale_reason = None`), and insert newly added values into `tracked_nodes` without foreign-key errors on `capture_associations`.
- [x] T002 [P] [US1] [FR-002] Fix `render_turtle_string` and `execute_graph_with_options` (`src/cli/graph.rs`) so `rgt graph --format ttl` succeeds in uninitialized directories using a valid 32-hex placeholder token and writes stdout via `turtle::write_bytes_before_deadline`.
- [x] T003 [US1] [FR-003] Fix Pass 1 / Pass 2 line-shift reconciliation in `record_file_batch` (`src/store/queries.rs`) so previously shifted nodes match at their updated `line_number` and new nodes inserted at a shifted node's original line number generate a collision-free ID instead of clobbering the shifted node via `ON CONFLICT(id)`.
- [x] T004 [P] [US1] [FR-004] Read `dev` and `ino` as `Option<i64>` and cast to `u64` in `get_source_document_by_dev_ino` (`src/store/queries.rs`) and `InvalidationCascade::evaluate_and_invalidate_all_with_deadline` (`src/graph/invalidation.rs`), re-verify `stored_hash` inside the `IMMEDIATE` transaction before invalidating changed files, and use saturating/filtered counting in `build_status_summary` (`src/cli/status.rs`).

---

## Phase 2: Number & Date Extraction Precision & Line-Number Unwrapping Hardening [US2]

- [x] T005 [US2] [FR-005] Harden `resolve_auto_format` (`src/hooks/parser.rs`) so US files with dot-decimals or multi-comma/bracketed lines (`Alice Smith,10,20`, `[1,2]`) do not flip to `NumberFormat::Eu`, and update `extract_numbers` in `Us` mode to split 2-column and spaced-text CSV integer cells (`10,200`, `10,200,300,John Doe`, `John Doe,12,34`).
- [x] T006 [P] [US2] [FR-006] Require sequential line numbers for `colon_re` in `unwrap_line_numbered_content` (`src/hooks/parser.rs`) so YAML/key-value files (`2024: 100\n2025: 200`) are not corrupted, normalize CRLF vs LF and wrapper trailing newlines in `matching_file_snapshot` (`src/hooks/mod.rs`), and support `(12.5%)` accounting negatives while keeping prose single-digit enumerations (`step (1)`) positive.

---

## Phase 3: 13-Agent Harness Scope Isolation, Path Normalization & Shell Parser Hardening [US3]

- [x] T007 [US3] [FR-007] [FR-008] Prevent project-scoped `rgt init --agent copilot` from mutating the host user's `$HOME` when `global == false` (`src/hooks/installer.rs`), strip `project_root` from in-project absolute paths and clean `./` in `DbStore::normalize_path_for_project` (`src/store/db.rs`), and resolve `capture.cwd` and file-anchored project root discovery in `handle_passive_hook_event` and `handle_pdf_capture` (`src/hooks/mod.rs`).
- [x] T008 [P] [US3] [FR-009] [FR-010] Enforce `is_qualifying_read_event` for `cursor` and `copilot`, extract response content in `copilot_dialect` when `toolArgs` is present, relax `opencode_native_read_dialect` when `truncated` or `metadata.display` is omitted, and harden `extract_command_str`, `split_shell_segments`, and `extract_path_from_command` (`/bin/bash`/`zsh` arrays, `\n` separators, `python -c` continuation, `sed -i` and `>` rejection, piped `grep`) in `src/hooks/parser.rs`.
- [x] T009 [P] [US3] [FR-011] Update `write_vibe` (`src/hooks/installer.rs`), `editor.rs`, and `registration.rs` to install and validate `post_tool` (`rgt hook post --agent vibe`) while accepting legacy `pre_tool`, clean up duplicate project `hooks.toml` in `write_gemini`/`write_vibe`, surface `Incomplete` project guidance in `inspect_local_guidance`, add `"roo"` to `ALL_AGENT_NAMES` (`src/hooks/paths.rs`), and fix the Windsurf preferred-path notice in `src/cli/init.rs`.

---

## Phase 4: MCP `unit` vs `result_unit`, Verify/Derive Exit Codes, Updater & CI/CD Hardening [US4]

- [x] T010 [US4] [FR-012] Separate `unit` (display unit) from `result_unit` (claim unit) in MCP `verify_only` and `derive_and_verify` (`src/mcp/mod.rs`) and `run_verify_core` (`src/verify/mod.rs`), reject `result_unit` without `result` in `run_verify_core`, accept `"result": null` as `None` in MCP, canonicalize near-zero `-0.0` float residuals in `src/verify/expression.rs`, and classify non-finite expression evaluations as exit code `1` (`VerificationFailed`) and wrong temporal parent kinds as exit code `2` (`InvalidInput`) in `src/verify/mod.rs` and `src/cli/derive.rs`.
- [x] T011 [P] [US4] [FR-013] Normalize bare SemVer version tags (`0.7.0` -> `v0.7.0`) in `src/updater/github.rs` and `install.sh`, and preserve detailed verification error messages in `src/updater/attestation.rs`.
- [x] T012 [P] [US4] [FR-014] Harden `.github/workflows/release.yml` (add `x86_64-apple-darwin`, fix `dev-` archive naming), `.github/workflows/cd.yml` (pass `${TAG_NAME}` to `git tag -fa latest`), `.github/workflows/ci.yml` (`cargo llvm-cov report --summary-only` and base-branch diff), `scripts/check-test-presence.sh`, and `scripts/test-harnesses-e2e.sh` (isolate `HOME`/`XDG_CONFIG_HOME`, test all 14 canonical agents and all 3 aliases).
- [x] T013 [US4] [SC-001] [SC-002] [SC-003] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`, and `bash scripts/test-harnesses-e2e.sh` with 0 failures.
