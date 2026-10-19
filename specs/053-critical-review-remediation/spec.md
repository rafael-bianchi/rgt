# Feature Specification: Pass-3 Critical Review Remediation

**Feature Branch**: `fix/053-critical-review-remediation`

**Created**: 2026-10-18

**Status**: Draft

**PriorityOrder**: Complete Pass-3 Critical Review Remediation (Spec 053)

**Input**: Complete findings from the Pass-3 Critical Review covering the `sync_staleness` + `record_file_batch` state-machine bug, `rgt graph --format ttl` in uninitialized directories, line-shift reconciliation & stale node resurrection, `u64`/`i64` SQLite `dev`/`ino` conversions, `resolve_auto_format` & CSV extraction false positives, `unwrap_line_numbered_content` colon/CRLF handling, project-scoped `rgt init --agent copilot` global mutation, passive hook path normalization & tool qualification, Mistral Vibe `post_tool` alignment, MCP `unit` vs `result_unit` separation, Turtle stdout deadline enforcement, updater/installer version tag normalization, and CI/CD & E2E test harness hardening.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Provenance Store State-Machine, Line-Shift Reconciliation & Turtle Uninitialized Export (Priority: P1)

When a tracked file is modified on disk, inspected via a read/verification command (`status`, `query`, `derive`, `verify`, `graph`, `gc`), and then re-recorded via `rgt record` or a passive hook, RGT must still reconcile and record the new values, un-stale unchanged values, preserve line-shift reconciliation across multiple edits without `ON CONFLICT(id)` collisions, safely convert 64-bit `dev`/`ino` values in SQLite, avoid TOCTOU false invalidations during `sync_staleness`, and allow `rgt graph --format ttl` to succeed in uninitialized directories.

**Why this priority**: Fixes a critical state-machine bug where running `rgt status` after editing a file permanently prevents `rgt record` from inserting new values or un-staling existing nodes (and crashes passive hooks with a foreign-key violation), and fixes `rgt graph --format ttl` failing with exit code 1 in uninitialized directories.

**Independent Test**:
1. Record `data.csv` (`a,100`), edit it to `a,100\nb,200`, run `rgt status` (marking `100` stale), and then run `rgt record data.csv` (or `rgt hook post`); verify `100` is restored to fresh (`is_stale = false`), `200` is inserted into `tracked_nodes`, and passive hook capture succeeds without a foreign-key error.
2. Run `rgt graph --format ttl` in an empty temporary directory without `.rgt/store.db`; verify it exits `0` with a valid Turtle prefix document and does not create `.rgt/store.db`.
3. Shift a line containing `100` from line 1 to line 5, then edit the file again to add a new `100` at line 1 while keeping `100` at line 5; verify both `100` nodes exist distinctly in `tracked_nodes` without clobbering the shifted node.
4. Store a document with `dev = u64::MAX` and `ino = u64::MAX`; verify `get_source_document_by_dev_ino` and `sync_staleness` succeed without `FromSqlConversionFailure`.

**Acceptance Scenarios**:

1. **Given** a tracked file modified on disk after `sync_staleness` has marked its nodes stale, **When** `record_file_batch` runs, **Then** it detects either a hash mismatch or stale existing roots for that document, reconciles unchanged values (clearing `is_stale = false` and `stale_reason = None`), inserts new values, and never skips insertion or triggers a `capture_associations` foreign-key failure.
2. **Given** an uninitialized directory without `.rgt/store.db`, **When** `rgt graph --format ttl` (or MCP `export_graph` with `format: "ttl"`) runs, **Then** it renders a valid empty Turtle document using a valid 32-hex placeholder token and writes stdout via `write_bytes_before_deadline`.
3. **Given** a root node whose `line_number` shifted in an earlier edit, **When** the file is edited again or a new identical value is inserted at the shifted node's original line number, **Then** Pass 1/Pass 2 reconciliation matches the shifted node at its current `line_number` and generates a collision-free root ID for the new node so `ON CONFLICT(id)` never clobbers the shifted node.
4. **Given** `dev` or `ino` values exceeding `i64::MAX`, **When** `get_source_document_by_dev_ino` or `evaluate_and_invalidate_all_with_deadline` reads `source_documents`, **Then** it reads `Option<i64>` and converts bit-identically to `u64` without failing.
5. **Given** `evaluate_and_invalidate_all_with_deadline`, **When** applying `DetectionResult::Changed` inside the `IMMEDIATE` transaction, **Then** it verifies the row's current `blake3_hash` in SQLite still matches the pre-transaction `stored_hash` before invalidating, and `build_status_summary` computes `active_nodes` via saturating/filtered counting without risking integer underflow.

---

### User Story 2 - Number & Date Extraction Precision & Line-Number Unwrapping Hardening (Priority: P1)

When RGT extracts numbers and dates from files or unwraps line-numbered tool outputs, it must not let CSV cells (`Alice,10,20`), arrays (`[1,2]`), or coordinates (`(0,0)`) poison `resolve_auto_format` into `NumberFormat::Eu`, must split 2-column and spaced-text CSV rows accurately in `Us` mode, must not strip `<digits>:` keys from plain YAML/config/data files in `unwrap_line_numbered_content`, must tolerate CRLF line endings in `matching_file_snapshot`, and must avoid turning prose parenthetical numbers (`step (1)`) into negative numbers while supporting accounting negatives with `%` (`(12.5%)`).

**Why this priority**: Prevents a single `1,2` or `Alice,10,20` line from corrupting all US decimals across an entire file, fixes CSV integer column merging, and prevents YAML/data files starting with `2024: 100` from failing snapshot verification.

**Independent Test**:
1. Extract values in `Auto` mode from a file containing `"Alice Smith,10,20\nPrice,12.50\nRatio,1.234\n"`; verify format resolves to `Us`, extracting `10`, `20`, `12.5`, and `1.234` (not `1234`).
2. Extract values in `Us` mode from `"10,200\n10,200,300,John Doe\nJohn Doe,12,34\n"`; verify all CSV cells are extracted as separate numbers (`10`, `200`, `10`, `200`, `300`, `12`, `34`).
3. Feed a YAML/data file `"2024: 100\n2025: 200\n"` and a multi-line CRLF file `"1→Metric: 1250\r\n2→Cost: 250\r\n"` through `unwrap_line_numbered_content` and `matching_file_snapshot`; verify both match on disk and extract accurately.

**Acceptance Scenarios**:

1. **Given** `resolve_auto_format`, **When** scanning for prose European decimal tokens (`12,34`), **Then** it only triggers on isolated whitespace-bounded tokens where no US decimal (`\b\d+\.\d+\b`) exists in the file and the line is not a multi-comma CSV/array/tuple line.
2. **Given** a CSV row in `Us` mode such as `10,200`, `10,200,300,John Doe`, or `John Doe,12,34`, **When** `extract_numbers` runs, **Then** unspaced comma-separated cells on CSV lines are split into distinct numbers rather than merged into thousands-grouped integers or rejected as `Malformed`.
3. **Given** `unwrap_line_numbered_content` and `matching_file_snapshot`, **When** inspecting colon-prefixed lines (`^\s*(\d+):\s?(.*)$`), **Then** `is_colon` only unwraps when line numbers are sequential (starting at $\ge 1$ and incrementing by $1$ per line), and `matching_file_snapshot` normalizes CRLF vs LF line endings and wrapper trailing newlines when comparing unwrapped tool output against disk.
4. **Given** parenthesized numbers in `extract_numbers`, **When** a number is enclosed in parentheses (`(12.5%)` or `(1,200.00)` vs prose `step (1)`), **Then** accounting parentheses with optional `%` inside `(...)` are recognized as negative while single-digit prose enumerations (`step (1)`, `item (2)`) are not turned negative unless preceded by accounting/table context or formatted as currency/decimal.

---

### User Story 3 - 13-Agent Harness Scope Isolation, Path Normalization & Shell Parser Hardening (Priority: P1)

When `rgt init`, `rgt doctor`, or `rgt hook post` runs across the supported harnesses, project-scoped `rgt init --agent copilot` must not touch global `$HOME` files, passive hooks must normalize in-project absolute paths to project-relative paths and discover `.rgt` from `capture.cwd` or the target file path (including for PDF reads), `cursor` and `copilot` must filter out non-read tool events, Mistral Vibe must install and validate `post_tool` hooks alongside legacy `pre_tool` support, shell command parsing must handle `/bin/bash`/`zsh` arrays, multi-line commands, `python -c` segments, `sed -i`, and output redirections (`>`), and `"roo"` must be included in `ALL_AGENT_NAMES`.

**Why this priority**: Stops project-level `rgt init --agent copilot` and E2E tests from mutating the user's real `~/.config/github-copilot/AGENTS.md`, unifies hook vs CLI file paths in the database, prevents `Write`/`Edit` tool calls in Cursor/Copilot from recording self-authored values, and fixes Mistral Vibe passive hook execution.

**Independent Test**:
1. Run `rgt init --agent copilot` (project scope) and verify `~/.config/github-copilot/AGENTS.md` is untouched.
2. Run `rgt hook post --agent claude-code` from `/tmp` with a payload whose `file_path` is `/abs/path/to/project/data.csv`; verify `.rgt` in `/abs/path/to/project` is found and the source document is stored as `"data.csv"` (matching `rgt record data.csv`).
3. Send a `cursor` or `copilot` `Write`/`Edit` payload to `rgt hook post`; verify it is ignored and records 0 values.
4. Run `rgt init --agent vibe` and verify `.vibe/hooks.toml` (or `.vibe/config.toml`) registers a `post` hook that `rgt doctor` reports as `Active (Healthy)`.

**Acceptance Scenarios**:

1. **Given** `write_copilot` in `src/hooks/installer.rs`, **When** `global == false`, **Then** it writes only project-local `.vscode/settings.json` and `.github/copilot-instructions.md` without modifying `copilot_cli_config_dir(home, config_dir).join("AGENTS.md")`.
2. **Given** `DbStore::normalize_path_for_project` and `handle_passive_hook_event` / `handle_pdf_capture`, **When** an absolute or relative path inside the project is recorded or a hook runs from another working directory, **Then** project-root discovery checks `capture.cwd` and the target file's parent directory, `handle_pdf_capture` resolves relative paths against `capture.cwd`, and `normalize_path_for_project` strips `project_root` from in-project absolute paths and cleans `./` segments.
3. **Given** `normalize_agent_event_with_phase`, **When** `agent` is `cursor` or `copilot` (or any default-dialect agent), **Then** `is_qualifying_read_event` rejects `Write`, `Edit`, `MultiEdit`, `NotebookEdit`, `create_file`, `edit_file`, `write_file`, and `delete_file` tools; `copilot_dialect` extracts response content when `toolArgs` is present; and `opencode_native_read_dialect` treats omitted `truncated` as `false` and falls back to `result.output` when `metadata.display` is absent.
4. **Given** `extract_command_str`, `split_shell_segments`, and `extract_path_from_command`, **When** parsing shell commands, **Then** `/bin/bash`, `/bin/sh`, `zsh`, and `/bin/zsh` command arrays unwrap `-c`/`-lc`, `\n` splits command segments, `python -c` uses `continue` rather than aborting subsequent `&&` segments, `sed -i` and output redirection targets (`> out.txt`) are never treated as read targets, and `grep` in a pipeline (`cmd | grep pattern`) requires a file argument beyond the pattern.
5. **Given** `write_vibe`, `write_gemini`, `registration.rs`, `paths.rs`, and `init.rs`, **When** configuring or inspecting agents, **Then** Vibe installs `post_tool` (`rgt hook post --agent vibe`) while `registration.rs` accepts `post_tool` or `pre_tool`, `write_gemini`/`write_vibe` clean up redundant project `hooks.toml` when migrating to `settings.json`/`config.toml`, `inspect_local_guidance` surfaces broken (`Incomplete`) project guidance before falling back to user scope, `ALL_AGENT_NAMES` includes `"roo"` (`17` valid spellings), and `init.rs` accurately states `.windsurf/hooks.json` is preferred over `.devin/hooks.json`.

---

### User Story 4 - MCP `unit` vs `result_unit` Separation, Verify/Derive Exit Codes, Updater Tag Normalization & CI/CD Hardening (Priority: P1)

When RGT runs MCP tools (`verify_only`, `derive_and_verify`, `record_values`), CLI `verify`/`derive`/`graph`/`update`, `install.sh`, or CI/CD workflows, MCP must keep `unit` (display unit) distinct from `result_unit` (claim unit) and tolerate `"result": null`, `verify`/`derive` must assign exit code `1` (`VerificationFailed`) to non-finite evaluation results (`a / 0`) and exit code `2` (`InvalidInput`) to wrong parent value types (`Number` passed to `DATE_DIFF`), `rgt update --version 0.7.0` and `install.sh -v 0.7.0` must normalize version tags to `v0.7.0`, and CI/CD workflows and E2E scripts must be free of tag/archive/home-directory bugs.

**Why this priority**: Fixes MCP false verification failures when `result` (in default seconds) and `unit` (for display) are passed together, fixes `rgt update --version 0.7.0` 404 failures, restores blocked-pipe deadline enforcement on `rgt graph --format ttl`, and fixes CI/CD release tagging and Intel macOS build coverage.

**Independent Test**:
1. Call MCP `derive_and_verify` and `verify_only` on a 10-day `DATE_DIFF` with `result: "864000"` and `unit: "days"` (without `result_unit`); verify the claim is checked in `seconds` (`864000s == 10 days`) and displayed in `days` (`"10 days"`), and verify `"result": null` is treated as `None`.
2. Run `rgt verify` / `rgt derive` with `a / b` where `b = 0` (verify exit code `1`) and `DATE_DIFF` with `Number` parents (verify exit code `2`).
3. Pass `--version 0.7.0` (without `v`) to `rgt update` and `install.sh --dry-run -v 0.7.0`; verify both normalize the tag to `v0.7.0`.

**Acceptance Scenarios**:

1. **Given** MCP `verify_only` and `derive_and_verify` (`src/mcp/mod.rs`) and `run_verify_core` (`src/verify/mod.rs`), **When** `result` and `unit` are provided without `result_unit`, **Then** `result` is verified in default `seconds` (unless `result_unit` is explicitly set), `unit` controls the returned `display` unit, `result_unit` without `result` is rejected as `InvalidInput`, and `"result": null` is deserialized as `None`.
2. **Given** `run_verify_core` and `execute_derive_core`, **When** an `EXPRESSION` evaluates to `inf`/`NaN` (e.g. division by zero), **Then** it returns `VerifyDeriveError::VerificationFailed` (exit code `1`), whereas passing non-`Date` parents to `DATE_DIFF` or non-`Duration` parents to `DURATION_SUM`/`DURATION_AVG` returns `VerifyDeriveError::InvalidInput` (exit code `2`), and near-zero float results matching `--result 0` canonicalize `-0.0`/sub-epsilon residuals to `0.0`.
3. **Given** `rgt update --version <tag>` (`src/cli/update.rs`, `src/updater/github.rs`) and `install.sh -v <tag>`, **When** `<tag>` is a valid SemVer without a leading `v` (e.g. `0.7.0`), **Then** it normalizes the release tag to `v0.7.0` before querying GitHub Releases, and `attestation::fetch_bundle` preserves underlying verification error messages.
4. **Given** `.github/workflows/release.yml`, `.github/workflows/cd.yml`, `.github/workflows/ci.yml`, `docker/docker-compose.harnesses.yml`, and `scripts/test-harnesses-e2e.sh`, **When** inspected and executed, **Then** `release.yml` includes `x86_64-apple-darwin` and strips `dev-` prefixes safely when naming archives, `cd.yml` passes `${TAG_NAME}` as the target ref to `git tag -fa latest`, `ci.yml` uses `cargo llvm-cov report --summary-only` without running the test suite twice, and `scripts/test-harnesses-e2e.sh` isolates `HOME`/`XDG_CONFIG_HOME` in a temporary directory and tests all 14 canonical agents plus all 3 aliases (`claude`, `kilo`, `roo`).

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `record_file_batch` (`src/store/queries.rs`) MUST reconcile and re-record extracted values whenever `existing_doc.blake3_hash != hash` OR any existing root node for `existing_doc.id` is currently marked `is_stale = true` (or extracted count exceeds existing active roots), restoring matched stale roots to `is_stale = false, stale_reason = None` and inserting any new values into `tracked_nodes` before recording `capture_associations`.
- **FR-002**: `render_turtle_string` (`src/cli/graph.rs`) MUST use a valid 32-character lowercase hexadecimal token (`"00000000000000000000000000000000"`) when rendering an empty snapshot in an uninitialized directory, and `execute_graph_with_options` MUST write Turtle output via `turtle::write_bytes_before_deadline`.
- **FR-003**: `record_file_batch` (`src/store/queries.rs`) MUST match previously line-shifted root nodes in Pass 1 by `(value_kind, value, line_number)` (or `r.id == expected_id`) and avoid colliding with existing active shifted node IDs when generating IDs for newly inserted nodes at a shifted node's original line.
- **FR-004**: `get_source_document_by_dev_ino` (`src/store/queries.rs`) and `InvalidationCascade::evaluate_and_invalidate_all_with_deadline` (`src/graph/invalidation.rs`) MUST read `dev` and `ino` from SQLite as `Option<i64>` and cast to `u64`, re-verify `stored_hash` inside the `IMMEDIATE` transaction before applying `DetectionResult::Changed`, and use saturating subtraction in `build_status_summary` (`src/cli/status.rs`).
- **FR-005**: `resolve_auto_format` (`src/hooks/parser.rs`) MUST NOT switch a file to `NumberFormat::Eu` when the file contains any unambiguous US decimal (`\b\d+\.\d+\b`) or when the `N,M` match occurs on a multi-comma CSV/array/tuple line; `extract_numbers` in `Us` mode MUST split 2-column and spaced-text CSV integer cells (`10,200`, `10,200,300,John Doe`, `John Doe,12,34`) rather than merging them or rejecting them as `Malformed`.
- **FR-006**: `unwrap_line_numbered_content` (`src/hooks/parser.rs`) MUST require sequential line numbers for `colon_re` (`^\s*(\d+):\s?(.*)$`) so plain YAML/key-value files (`2024: 100\n2025: 200`) are not unwrapped, and `matching_file_snapshot` (`src/hooks/mod.rs`) MUST tolerate CRLF vs LF line endings and wrapper trailing newlines; `extract_numbers` MUST recognize accounting parentheses with `%` (`(12.5%)`) as negative while keeping positive sign on prose single-digit enumerations (`step (1)`).
- **FR-007**: `write_copilot` (`src/hooks/installer.rs`) MUST only write `copilot_cli_config_dir(home, config_dir).join("AGENTS.md")` when `global == true` (or write project-local instructions when `global == false`), never mutating `$HOME` during project-scoped `rgt init --agent copilot`.
- **FR-008**: `DbStore::normalize_path_for_project` (`src/store/db.rs`) MUST strip `project_root` from in-project absolute paths and clean leading `./` segments; `handle_passive_hook_event` and `handle_pdf_capture` (`src/hooks/mod.rs`) MUST resolve relative paths against `capture.cwd` before snapshot checking and discover the project root from `capture.cwd` or the target file path when `"."` is outside the project.
- **FR-009**: `normalize_agent_event_with_phase` (`src/hooks/parser.rs`) MUST enforce `is_qualifying_read_event` for `cursor` and `copilot` (rejecting write/edit tools), extract content in `copilot_dialect` when `toolArgs` is present, and make `opencode_native_read_dialect` default missing `truncated` to `false` and fall back to `result.output` when `metadata.display` is absent.
- **FR-010**: `extract_command_str`, `split_shell_segments`, and `extract_path_from_command` (`src/hooks/parser.rs`) MUST unwrap `/bin/bash`, `/bin/sh`, `zsh`, and `/bin/zsh` `-c`/`-lc` command arrays, split segments on `\n`, `continue` rather than `return None` on `python -c`, reject `sed -i`, ignore shell output redirections (`>`, `>>`), and require a file argument after the pattern for piped `grep`.
- **FR-011**: `write_vibe` (`src/hooks/installer.rs`) and `registration.rs` MUST support `post_tool` (`rgt hook post --agent vibe`) while retaining backward compatibility with `pre_tool` in `editor.rs`/`registration.rs`; `write_gemini` and `write_vibe` MUST remove redundant project `hooks.toml` when writing `settings.json`/`config.toml`; `inspect_local_guidance` MUST surface `Incomplete` project guidance before user-scope fallback; `ALL_AGENT_NAMES` (`src/hooks/paths.rs`) MUST include `"roo"`; and `src/cli/init.rs` MUST state `.windsurf/hooks.json` is preferred over `.devin/hooks.json`.
- **FR-012**: MCP `verify_only` and `derive_and_verify` (`src/mcp/mod.rs`) and `run_verify_core` (`src/verify/mod.rs`) MUST keep `result_unit` (claim unit, defaulting to `"seconds"`) separate from `unit` (display unit), reject `result_unit` when `result` is `None`, and treat `"result": null` as `None`; `verify`/`derive` MUST classify non-finite expression evaluations (`inf`/`NaN`) as `VerificationFailed` (exit 1) and wrong temporal parent kinds as `InvalidInput` (exit 2).
- **FR-013**: `rgt update --version <tag>` (`src/cli/update.rs`, `src/updater/github.rs`), `attestation::fetch_bundle` (`src/updater/attestation.rs`), and `install.sh` MUST normalize valid SemVer tags without a leading `v` (e.g. `0.7.0` $\rightarrow$ `v0.7.0`) when querying GitHub Releases (while falling back to the raw tag if needed for custom tags in tests) and preserve detailed attestation verification error messages.
- **FR-014**: `.github/workflows/release.yml`, `.github/workflows/cd.yml`, `.github/workflows/ci.yml`, `scripts/check-test-presence.sh`, and `scripts/test-harnesses-e2e.sh` MUST include `x86_64-apple-darwin` in `release.yml`, pass `${TAG_NAME}` to `git tag -fa latest` in `cd.yml`, avoid running `cargo llvm-cov` twice in `ci.yml`, isolate `HOME`/`XDG_CONFIG_HOME` in `scripts/test-harnesses-e2e.sh`, and test all 14 canonical agents and all 3 aliases (`claude`, `kilo`, `roo`).

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass with 0 warnings and 0 errors.
- **SC-002**: `cargo test --all` passes 100% of unit, contract, and integration tests, including new regression tests for `FR-001`–`FR-014`.
- **SC-003**: Running `bash scripts/test-harnesses-e2e.sh` (locally and/or in `rgt-harness-env`) passes 100% of checks with `0 failed` and leaves the host `$HOME` completely untouched.
