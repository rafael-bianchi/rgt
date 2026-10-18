# Tasks: Spec 052 - Pass-2 Adversarial Critical Review Remediation

**Input**: Feature specification `specs/052-adversarial-review-remediation/spec.md` and plan `specs/052-adversarial-review-remediation/plan.md`.

---

## Phase 1: Live 13-Agent Harness Compatibility, Shell Parsing & Config Editor Hardening [US1]

- [x] T001 [US1] [FR-001] Unwrap Claude Code `cat -n` (`^\s*\d+[→\t]`) and OpenCode `<file>\n00001| ...` line-numbered `Read` payloads, propagate CLI `event_type` fallback to `CapturePhase`, normalize OpenCode `args.filePath` vs `display.path`, allow OpenCode `bash` reads via `command_dialect`, and support Codex CLI `shell`/`local_shell`/`exec_command`/`container.exec` with array `cmd`/`command` and `tool_response.output` in `src/hooks/parser.rs` and `src/hooks/mod.rs`.
- [x] T002 [P] [US1] [FR-002] Unlock `capture.content_complete` for any snapshot-verified agent payload in `src/hooks/parser.rs`, and update `src/hooks/installer.rs`, `src/hooks/paths.rs`, `src/hooks/registration.rs`, and `src/hooks/glue/mod.rs` to retain `PreToolUse`/`PostToolUse` compatibility while supporting Gemini `.gemini/settings.json` (`AfterTool`), Vibe `post_tool` / `.vibe/config.toml`, Hermes `plugin.yaml` + global guidance path, Pi `pi.on("tool_result", ...)`, Windsurf `.windsurf/hooks.json` preference, and distinct Roo Code `.roo/rules/` vs Cline `.clinerules`.
- [x] T003 [P] [US1] [FR-003] Harden `extract_path_from_command` (quoted paths with spaces, env var prefixes, `cd <dir> &&` compounds, `/bin/` paths, `bat`/`sed`, and rejecting `python -c` / `python3 -c` inline code), honor payload `cwd` and `filePath` aliases in `src/hooks/parser.rs` and `src/hooks/mod.rs`, and add `TERM`/`INT` traps to the watchdog subshell in `posix_direct_hook_command` (`src/hooks/installer.rs`).
- [x] T004 [P] [US1] [FR-004] Fix JSONC line-comment splicing and `matcher` comparison/`--force` update in `src/hooks/editor.rs`, preserve existing `.rgt.bak` backups in `backup_before_write`, and exclude `GuidanceOnly` from `hooks_present` in `src/cli/doctor.rs`.

---

## Phase 2: Root & Derived Node Identity Stability & Lineage Order Preservation [US2]

- [x] T005 [US2] [FR-005] Scope intra-line `occurrence` per `(value_kind, canonical_value_repr)` in `extract_values_from_content` (`src/hooks/parser.rs`).
- [x] T006 [US2] [FR-005] Reconcile pure line shifts in `record_file_batch` (`src/store/queries.rs`) by updating `line_number` in place for unchanged values without invalidating root IDs or hijacking adjacent line IDs, canonicalize `file_path` across `(dev, ino)` aliases, and normalize `\` to `/` in `normalize_path_for_project` (`src/store/db.rs`).
- [x] T007 [US2] [FR-006] Use length-delimited hashing including `expression` in `TrackedNode::generate_derived_id` with legacy fallback in `classify_derivation_group` (`src/types/node.rs`, `src/cli/derive.rs`, `src/export/turtle.rs`), reject duplicate parent IDs in `EXPRESSION` derivations (`src/verify/expression.rs`, `src/cli/derive.rs`), and preserve declared parent edge order in `rgt query` (`src/query/mod.rs`).

---

## Phase 3: Value Extraction False-Positive Elimination & Expression AST Validation [US3]

- [x] T008 [US3] [FR-007] Prevent `Auto`/`Us` mode from splitting European decimal tokens (`12,34`) in prose into two numbers, reject IPv4 addresses (`192.168.100.100`) and multi-dot versions (`v1.100.200`) in `Eu`/`Us` modes, support compact ISO-8601 offsets (`[+-]\d{4}`), mask URLs/standalone `HH:MM(:SS)?` times/slash dates/UUIDs, and treat `-` after `)` or `]` as binary subtraction (`+200.0`) in `src/hooks/parser.rs`.
- [x] T009 [P] [US3] [FR-008] Reject semicolons (`;`), string literals (`"`, `'`), comparison/boolean operators, and excessive operator chains (> 256 operators) in `validate_expression` / `validate_parent_variable_references`, and canonicalize `-0.0` to `0.0` in `src/verify/expression.rs`.

---

## Phase 4: SQLite `IMMEDIATE` Transactions, Atomic `sync_staleness` & Store Hygiene [US4]

- [x] T010 [US4] [FR-009] Switch all write transactions in `src/hooks/mod.rs`, `src/cli/record.rs`, `src/cli/derive.rs`, `src/cli/gc.rs`, and `src/graph/invalidation.rs` to `TransactionBehavior::Immediate`, and verify parent staleness inside the write transaction in `execute_derive_core`.
- [x] T011 [US4] [FR-010] Wrap `evaluate_and_invalidate_all` (`src/graph/invalidation.rs`) in an atomic `IMMEDIATE` transaction with deduplicated downstream invalidation and optional `deadline` checks (`src/cli/graph.rs`), and replace N+1 queries in `list_all_nodes`, `list_stale_nodes`, and `list_nodes_by_source_doc` (`src/store/queries.rs`) with single-query row mapping.
- [x] T012 [P] [US4] [FR-011] Prevent read-only/maintenance CLI commands (`status`, `query`, `graph`, `verify`, `gc`) and read-only MCP tools from creating `.rgt/store.db` on disk in uninitialized directories (`src/store/db.rs`, `src/cli/`, `src/query/mod.rs`, `src/verify/mod.rs`, `src/mcp/mod.rs`).

---

## Phase 5: MCP Server, CLI, Updater & CI/CD Pipeline Hardening [US5]

- [x] T013 [US5] [FR-012] Compute `blake3_hash` from `text.as_bytes()` whenever explicit/stdin content is provided in `execute_record_with_content` (`src/cli/record.rs`), and extend `RgtMcpServer` (`src/mcp/mod.rs`) with optional/string-safe `result`, `unit`, `result_unit`, `export_graph`, and file-anchored project root discovery.
- [x] T014 [P] [US5] [FR-013] Reject unknown `rgt graph --format` values with exit code `2`, propagate `get_child_edges` and `gc` SQL errors, escape Mermaid and DOT labels (`src/cli/graph.rs`, `src/cli/gc.rs`), and send `skipped_malformed` warnings to `stderr` (`src/cli/record.rs`).
- [x] T015 [P] [US5] [FR-014] Validate `--version <tag>` before URL construction in `src/cli/update.rs` and `src/updater/github.rs`, bound HTTP (`64 MiB` archive / `16 MiB` JSON-bundle) and Snappy (`16 MiB`) payload sizes, check all `.attestations` entries against `rgt_policy()` in `src/updater/attestation.rs`, and strip `Authorization` headers on `browser_download_url` redirects.
- [x] T016 [P] [US5] [FR-002] [FR-015] Harden `.github/workflows/cd.yml`, `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/workflows/turtle-performance.yml`, `install.sh`, `scripts/check-test-presence.sh`, `scripts/check-readme-parity.sh`, `docs/compatibility/support-matrix.md`, and `scripts/test-harnesses-e2e.sh`.
- [x] T017 [US5] [SC-001] [SC-002] [SC-003] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`, and rebuild/run `rgt-harness-env` Docker E2E test suite with 0 failures and 0 skips.

---

## Phase 6: Convergence

- [x] T018 Make `result` truly optional in MCP `verify_only` for `DATE_DIFF`/`DURATION_SUM`/`DURATION_AVG` (compute and return the typed Duration instead of passing `""` into `DurationClaim::parse`, which currently fails with an empty-claim error), return a clear "result is required" error for `EXPRESSION`, reject non-string/non-number `result` JSON values (`null`/`bool`/objects) as invalid input, and add a contract test in `tests/contract/test_mcp_contract.rs` (`src/mcp/mod.rs`, `src/verify/mod.rs`) per US5/AC2, FR-012 (partial)

---

## Phase 7: Convergence

- [x] T019 Review and align the `--expression` rejection for non-`EXPRESSION` operations that T018 added to `run_verify_core` (`src/verify/mod.rs`): it now makes `rgt verify` and MCP `verify_only` reject `DATE_DIFF`/`DURATION_*` calls that include `expression`, while `execute_derive_core` (`src/cli/derive.rs`) and MCP `derive_and_verify` still accept them and save the stray expression on the temporal derivation edge (`insert_derivation_edge`). Either apply the same rejection (exit code `2`) in `execute_derive_core` or remove it from `run_verify_core`, and add CLI contract coverage for both `verify` and `derive` in `tests/contract/test_duration_claim_contract.rs`, per plan §3 / FR-008 scope (unrequested)
