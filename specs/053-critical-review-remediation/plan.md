# Pass-3 Critical Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate all Critical, High, and Medium findings from the Pass-3 Critical Review across the SQLite provenance store, invalidation cascade, number/date extraction parser, 13-agent hook/plugin integrations, MCP server, self-updater, and CI/CD & E2E scripts.

**Architecture:** Fix the `sync_staleness` + `record_file_batch` state-machine interaction so stale nodes on modified files are properly reconciled, un-staled, or replaced without `ON CONFLICT(id)` clobbering; harden `resolve_auto_format`, CSV cell splitting, line-number unwrapping, and shell command extraction in `src/hooks/parser.rs`; isolate project-scope `rgt init` writes, normalize in-project absolute paths, separate MCP `unit` from `result_unit`, and fix updater/workflow/E2E script defects.

**Tech Stack:** Rust (2021 edition), `rusqlite`, `petgraph`, `chrono`, `regex`, `rmcp`, `ureq`, POSIX sh / Bash, GitHub Actions YAML.

**Spec:** `specs/053-critical-review-remediation/spec.md`

## Global Constraints

- Pure Rust single binary with zero runtime dependencies (Constitution Principle I).
- `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` MUST pass with zero warnings (Constitution Principle XI).
- Every behavioral change MUST be covered by deterministic unit, contract, or integration tests (Constitution Principle IX).
- CLI exit codes MUST follow `0 = success`, `1 = verification/expected failure`, `2 = invalid input` (Constitution Principle XI).

## Review Focus

- Modified file re-recorded after `rgt status` / `sync_staleness` has already updated `source_documents.blake3_hash` and marked roots `is_stale = 1`: `rgt record` and passive hooks must un-stale unchanged roots and insert new roots without foreign-key errors.
- Uninitialized directory running `rgt graph --format ttl`: must succeed with exit code `0` and emit a valid empty Turtle prefix document without creating `.rgt/store.db`.
- US file containing both US decimals (`12.50`, `1.234`) and a line with a space and a two-number comma pair (`Alice Smith,10,20` or `[1,2]`): `Auto` mode must remain `Us`, not flip to `Eu`.
- Project-scoped `rgt init --agent copilot` (`global == false`): must not touch `~/.config/github-copilot/AGENTS.md`.
- MCP `derive_and_verify` / `verify_only` called with `result: "864000"` and `unit: "days"` (no `result_unit`): must verify `864000` in default `seconds` and format display in `days`.

---

### Task 1: Store State-Machine, Line-Shift Reconciliation, `dev`/`ino` Conversions & Turtle Export (`FR-001`–`FR-004`)

**Files:**
- Modify: `src/store/queries.rs`
- Modify: `src/graph/invalidation.rs`
- Modify: `src/cli/graph.rs`
- Modify: `src/cli/status.rs`
- Test: `tests/unit/test_record.rs`, `tests/contract/test_turtle_contract.rs`

**Interfaces:**
- `record_file_batch` in `src/store/queries.rs`: reconciles whenever `existing_doc.blake3_hash != hash` OR `existing_roots.iter().any(|r| r.is_stale)` OR `existing_roots.len() != extracted.len()`, includes stale roots as reconciliation candidates when `existing_doc.blake3_hash == hash` (clearing `is_stale = false, stale_reason = None` via `unmark_node_stale`), matches previously shifted roots in Pass 1 by `r.line_number == ext.line_number && r.value == ext.value`, and disambiguates newly generated root IDs if an active shifted root on another line already holds `expected_id`.
- `render_turtle_string` in `src/cli/graph.rs`: uses `"00000000000000000000000000000000"` as `project_id` when `DbStore::open_existing_in_project_with_timeout` returns `None`, and `execute_graph_with_options` uses `turtle::write_bytes_before_deadline` when writing `ttl` output.

- [ ] **Step 1: Write failing regression tests in `tests/unit/test_record.rs` and `tests/contract/test_turtle_contract.rs`**
- [ ] **Step 2: Run tests to verify they fail**
- [ ] **Step 3: Implement fixes in `src/store/queries.rs`, `src/graph/invalidation.rs`, `src/cli/graph.rs`, and `src/cli/status.rs`**
- [ ] **Step 4: Run tests to verify they pass**

---

### Task 2: Number & Date Extraction Precision & Line-Number Unwrapping Hardening (`FR-005`–`FR-006`)

**Files:**
- Modify: `src/hooks/parser.rs`
- Modify: `src/hooks/mod.rs`
- Test: `tests/unit/test_record.rs`, `tests/unit/hook_parsers.rs`

**Interfaces:**
- `resolve_auto_format(content: &str) -> NumberFormat` in `src/hooks/parser.rs`: checks for unambiguous dot-decimals (`^\d+\.\d+$` where fractional digits $\ne 3$) before the secondary prose `eu_decimal_regex` scan, and restricts the secondary scan to lines without multiple commas or bracketed lists.
- `extract_numbers` in `src/hooks/parser.rs`: splits 2-column and spaced-text CSV integer tokens in `Us` mode when the line is CSV-shaped (contains commas without whitespace around the comma), supports `(12.5%)` accounting negatives, and avoids negating single-digit prose enumerations `(1)`–`(9)`.
- `unwrap_line_numbered_content(raw: &str) -> (String, Vec<u32>)` in `src/hooks/parser.rs` and `matching_file_snapshot` in `src/hooks/mod.rs`: requires sequential line numbers for `colon_re` and normalizes `\r\n` vs `\n` in `matching_file_snapshot`.

- [ ] **Step 1: Write failing regression tests in `tests/unit/test_record.rs` and `tests/unit/hook_parsers.rs`**
- [ ] **Step 2: Run tests to verify they fail**
- [ ] **Step 3: Implement fixes in `src/hooks/parser.rs` and `src/hooks/mod.rs`**
- [ ] **Step 4: Run tests to verify they pass**

---

### Task 3: 13-Agent Harness Scope Isolation, Path Normalization & Shell Parser Hardening (`FR-007`–`FR-011`)

**Files:**
- Modify: `src/hooks/installer.rs`
- Modify: `src/hooks/registration.rs`
- Modify: `src/hooks/paths.rs`
- Modify: `src/hooks/editor.rs`
- Modify: `src/hooks/parser.rs`
- Modify: `src/hooks/mod.rs`
- Modify: `src/store/db.rs`
- Modify: `src/cli/init.rs`
- Test: `tests/contract/hook_installer.rs`, `tests/unit/hook_parsers.rs`, `tests/contract/test_doctor_contract.rs`

**Interfaces:**
- `write_copilot` in `src/hooks/installer.rs`: only writes `copilot_cli_config_dir(home, config_dir).join("AGENTS.md")` when `global == true` (or when `copilot_cli_config_dir` is inside a project test fixture? Wait: keep contract test compatibility! Let's check `copilot_writer_creates_cli_rules_file` in `tests/contract/hook_installer.rs` before changing `write_copilot`!).
- `DbStore::normalize_path_for_project` in `src/store/db.rs`: strips `project_root` from in-project absolute paths and cleans `./` prefixes.
- `handle_passive_hook_event` and `handle_pdf_capture` in `src/hooks/mod.rs`: resolve relative paths against `capture.cwd` before `handle_pdf_capture` and discover `project_root` from `capture.cwd` / target file path fallback.
- `is_qualifying_read_event`, `extract_command_str`, `split_shell_segments`, `extract_path_from_command` in `src/hooks/parser.rs`.
- `ALL_AGENT_NAMES` in `src/hooks/paths.rs`, Windsurf notice in `src/cli/init.rs`, and Vibe/Gemini/guidance inspection in `src/hooks/installer.rs` and `src/hooks/registration.rs`.

- [ ] **Step 1: Write failing regression tests in `tests/unit/hook_parsers.rs`, `tests/contract/hook_installer.rs`, and `tests/contract/test_doctor_contract.rs`**
- [ ] **Step 2: Run tests to verify they fail**
- [ ] **Step 3: Implement fixes across `src/hooks/`, `src/store/db.rs`, and `src/cli/init.rs`**
- [ ] **Step 4: Run tests to verify they pass**

---

### Task 4: MCP `unit` vs `result_unit`, Verify/Derive Error Classification, Updater Tag Normalization & CI/CD Hardening (`FR-012`–`FR-014`)

**Files:**
- Modify: `src/mcp/mod.rs`
- Modify: `src/verify/mod.rs`
- Modify: `src/verify/expression.rs`
- Modify: `src/cli/derive.rs`
- Modify: `src/cli/update.rs`
- Modify: `src/updater/github.rs`
- Modify: `src/updater/attestation.rs`
- Modify: `install.sh`
- Modify: `.github/workflows/release.yml`, `.github/workflows/cd.yml`, `.github/workflows/ci.yml`
- Modify: `scripts/check-test-presence.sh`, `scripts/test-harnesses-e2e.sh`
- Test: `tests/contract/test_mcp_contract.rs`, `tests/contract/test_duration_claim_contract.rs`, `tests/contract/test_update_contract.rs`

**Interfaces:**
- `parse_mcp_result` in `src/mcp/mod.rs`: maps `Some(serde_json::Value::Null)` to `Ok(None)` and passes `result_unit.as_deref()` (without falling back to `unit.as_deref()`) as `result_unit` to `run_verify_core` and `execute_derive_core`.
- `run_verify_core` in `src/verify/mod.rs`: rejects `result_unit.is_some() && result.is_none()` with `InvalidInput`, uses `result_unit.unwrap_or("seconds")` for `DurationClaim::parse`, and maps wrong parent kinds in temporal operations to `InvalidInput` (exit 2) and non-finite expression evaluation to `VerificationFailed` (exit 1).
- `fetch_release_by_tag` in `src/updater/github.rs` and `install.sh`: normalizes bare SemVer `0.7.0` to `v0.7.0`.

- [ ] **Step 1: Write failing regression tests in `tests/contract/test_mcp_contract.rs`, `tests/contract/test_duration_claim_contract.rs`, and `tests/contract/test_update_contract.rs`**
- [ ] **Step 2: Run tests to verify they fail**
- [ ] **Step 3: Implement fixes in `src/mcp/mod.rs`, `src/verify/`, `src/cli/derive.rs`, `src/updater/`, `install.sh`, `.github/workflows/`, and `scripts/`**
- [ ] **Step 4: Run full verification suite (`cargo fmt`, `cargo clippy`, `cargo test --all`, `scripts/test-harnesses-e2e.sh`)**
