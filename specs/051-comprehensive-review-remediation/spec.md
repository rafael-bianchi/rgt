# Feature Specification: Comprehensive Critical Review Remediation & Verification Hardening

**Feature Branch**: `fix/051-comprehensive-review-remediation`

**Created**: 2026-10-17

**Status**: Implemented & Verified

**PriorityOrder**: Complete Cross-Cutting Remediation (Specs 031–051)

**Input**: Complete findings from the 6-domain Critical Architecture & Implementation Review (`critical_review.md` §§1–6) and the follow-up Post-Fix Verification Audit across Store & Graph Integrity, Value & Date Extraction, Verification & Duration Math, 13-Agent Hook/Plugin Protocols & Config Editors, CLI/Turtle/MCP/Updater/Release Pipelines, and Containerized 13-Harness E2E Testing.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Store Staleness, Subdirectory Path Rebasing & Derived Staleness Propagation (Priority: P1)

When a developer initializes an RGT project, records files from the project root or any nested subdirectory, re-records modified files (by relative path, absolute path, symlink, or `--stdin`), or derives new nodes from existing parents, RGT must maintain exact staleness invariants, preserve `source_doc_id` links across `(dev, ino)` aliases, accumulate consecutive `--stdin` recordings without self-invalidation, and propagate parent staleness to newly derived nodes.

**Why this priority**: Prevents silent provenance corruption where modified files, subdirectory relative paths, or derivations from stale parents appear `VALID`.

**Independent Test**:
1. Run `rgt init` in a nested directory inside a workspace that already has an ancestor `.rgt/store.db`; verify `.rgt/store.db` is created locally in the initialized directory (`DbStore::init_in_project`).
2. From a subdirectory `sub/` of an initialized project (without running `rgt init` in `sub/`), run `rgt record data.csv` and `rgt status`; verify `data.csv` is stored as `sub/data.csv` and is not falsely marked `FILE_DELETED`.
3. Derive a child node from a parent with `is_stale = true`; verify the derived node is stored with `is_stale = true` and `stale_reason = Some("PARENT_NODE_STALE")`.

**Acceptance Scenarios**:

1. **Given** an initialized project root `/repo` and a file `/repo/sub/data.csv`, **When** `rgt record data.csv` or a passive hook executes with working directory `/repo/sub`, **Then** `DbStore::normalize_path_for_project` rebases the stored path to `sub/data.csv` so subsequent `sync_staleness` checks from any directory find the file on disk.
2. **Given** a directory `/repo/workspace/child` whose parent `/repo/workspace` already has `.rgt/store.db`, **When** the user explicitly runs `rgt init` inside `/repo/workspace/child`, **Then** `DbStore::init_in_project` creates `/repo/workspace/child/.rgt/store.db` rather than attaching to the parent store.
3. **Given** a file recorded via one path and re-recorded after modification via an alias with the same `(dev, ino)`, **When** `record_file_batch` runs, **Then** it looks up existing `source_documents` by path or `(dev, ino)`, marks removed/changed root nodes `SOURCE_FILE_MODIFIED`, and cascades `PARENT_NODE_STALE` to all downstream derivations.
4. **Given** consecutive `rgt record --stdin` invocations without a `<file>` path, **When** `record_file_batch` records under `"<stdin>"`, **Then** prior `<stdin>` root nodes are not self-invalidated.
5. **Given** one or more parent nodes where at least one parent has `is_stale = true`, **When** `rgt derive` creates or reuses a derived node, **Then** the derived node is marked `is_stale = true` with `stale_reason = "PARENT_NODE_STALE"`.
6. **Given** a v1 or v2 `.rgt/store.db` upgraded to `SCHEMA_VERSION = 3`, **When** `stamp_or_migrate_schema` runs, **Then** `tracked_nodes` preserves `CHECK(is_stale IN (0, 1))`, `duration_secs INTEGER`, `CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION'))`, and index `idx_tracked_nodes_source_doc`, while skipping table rebuilds on fresh databases that already have v3 columns.

---

### User Story 2 - Value & Date Extraction Precision Without False Positives (Priority: P1)

When RGT extracts numbers and dates from prose, financial reports, CSV files, and technical logs, it must accurately distinguish US/EU thousands-grouped numbers (`1,000,000`, `$12,345,678`) from unspaced CSV rows (`10,200,300`, `10,1.5e10`), extract unit-suffixed numbers (`12.5kg`, `99.9ms`, `100GB`), mask all ISO-8601/RFC-3339 timestamps (including lowercase `t`/`z`, timezone offsets, and invalid calendar dates like `2026-02-30`), and ignore hexadecimal hashes, git SHAs (`7e3a9f1`), UUIDs, and ordinals (`1st`, `2nd`).

**Why this priority**: Extraction false positives (splitting millions or extracting digits from UUIDs/SHAs/dates) and false negatives (dropping CSV cells or scientific notation) directly corrupt root provenance nodes.

**Independent Test**: Run `extract_values_from_content` on prose with `1,000,000` and `$12,345,678`, unspaced CSV `10,200,300` and `10,1.5e10`, unit-suffixed `12.5kg`, timestamps `2026-01-15T10:30:00-03:00` and `2026-02-30`, and identifiers `7e3a9f1`, `0x1a2b`, `96a3adc0-f76e-4faf-81e6-5b0578c2c46d`, `1st`.

**Acceptance Scenarios**:

1. **Given** prose containing `Revenue: 1,000,000 and budget $12,345,678`, **When** parsed in `Us` or `Auto` mode, **Then** RGT extracts `1_000_000.0` and `12_345_678.0` as single numbers.
2. **Given** an unspaced CSV line `10,200,300` or `10,1.5e10`, **When** parsed in `Us` or `Auto` mode, **Then** RGT splits cells at comma boundaries into `[10.0, 200.0, 300.0]` and `[10.0, 1.5e10]`.
3. **Given** tokens with trailing ASCII letters, **When** `extract_numbers` checks the right boundary, **Then** recognized measurement/currency suffixes (`is_allowed_unit_suffix`: `kg`, `ms`, `gb`, `usd`, etc.) are accepted while ordinals (`1st`), hex literals (`0x1a2b`), git SHAs (`7e3a9f1`), and UUIDs (`96a3adc0-f76e-4faf-81e6-5b0578c2c46d`) are rejected.
4. **Given** negative currency values (`-$1,200.00`, `($1,200.00)`, `-€500`) or multi-hyphen CLI flags (`--100`), **When** extracted, **Then** negative currency preserves its minus sign while `--100` extracts as positive `100.0`.
5. **Given** a 309+ digit number that overflows `f64::MAX` or `-0.0`, **When** `normalize_number` runs, **Then** non-finite values return `None` (`skipped_malformed += 1`) and `-0.0` is canonicalized to `+0.0`.

---

### User Story 3 - Strict Arithmetic Verification & Duration String Representation (Priority: P1)

When verifying `EXPRESSION` or Duration derivations, RGT must require every declared parent variable (`a`, `b`, ...) to be referenced in the expression, enforce strict relative floating-point tolerance (`1e-9 * largest`) without accepting opposite signs or 100% relative error on small numbers, persist the exact `computed_val`, and preserve non-zero seconds when formatting multi-day `Duration` values.

**Why this priority**: Prevents bogus provenance edges via unused parents (`a + 0`) and prevents false verification passes on small numbers (`1e-10` vs `2e-10` or `-1e-12`).

**Acceptance Scenarios**:

1. **Given** `--parents id1,id2 --operation EXPRESSION --expression "a + 0"`, **When** `verify` runs, **Then** `validate_parent_variable_references` rejects the expression because parent variable `b` is unused (CLI exit code `2`).
2. **Given** expected vs computed values in `approx_eq`, **When** both are non-zero, **Then** opposite signs are rejected and `diff <= 1e-9 * largest` is required.
3. **Given** `ValueData::Duration(Duration::seconds(86_401))`, **When** `to_string_repr()` is called, **Then** it returns `"1d 1s"` (and `"1d 1h 0m 5s"` for `90_005` seconds), never dropping non-zero seconds when `days > 0`.

---

### User Story 4 - 13-Agent Hook/Plugin Dialect, Config Editor & Containerized CLI Parity (Priority: P1)

Across all 13 supported AI coding harnesses, RGT must accurately parse native and shell-read hook payloads, preserve user settings and Windows paths when editing config files, avoid `init` vs `doctor` shape mismatches, and provide a complete containerized testing environment (`docker/Dockerfile.harnesses`) with all 13 agent CLIs (including `vibe` and `hermes`) installed and verified.

**Acceptance Scenarios**:

1. **Given** Claude Code's nested `PostToolUse` payload (`tool_response.file.content`) or legacy `.claude/hooks.json`, **When** processed, **Then** `parser.rs` extracts the complete snapshot and `write_claude_code` backs up RGT-owned `.claude/hooks.json` to `.claude/hooks.json.rgt.bak` before removing it.
2. **Given** an OpenCode `tool.execute.after` payload for a file with trailing newlines, **When** `opencode_native_read_dialect` validates `totalLines`, **Then** both `text.lines().count()` and `text.split('\n').count()` are accepted.
3. **Given** a Hermes plugin generated by `rgt init --agent hermes`, **When** written to `plugin.py`, **Then** only `post_tool_call` is registered (no redundant `pre_tool_call` no-op invocation).
4. **Given** a JSON hook config where an entry has flat `{"command": "..."}` instead of nested `{"hooks": [{"type": "command", "command": "..."}]}`, **When** `rgt init` runs `merge_array`, **Then** `shapes_match` detects the structural mismatch and rewrites the entry so `rgt doctor` reports `Active` rather than `Obsolete`.
5. **Given** a double-quoted Windows command path `"C:\Program Files\rgt\rgt.exe" hook post --agent claude-code`, **When** `shell_tokens` parses the command, **Then** backslashes before non-special characters are preserved verbatim.
6. **Given** `docker/Dockerfile.harnesses`, **When** the container image is built and `scripts/test-harnesses-e2e.sh` executes inside it, **Then** all 13 agent CLIs (`claude`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `agy`, `kilo`) are present on `PATH` and all 58 E2E checks pass with `0` failures and `0` skips.

---

### User Story 5 - MCP Server Mode, Turtle Export & Release/CI/CD Pipeline Integrity (Priority: P1)

RGT's MCP server (`rgt mcp`), PROV-O Turtle exporter (`rgt graph --format ttl`), POSIX installer (`install.sh`), and GitHub Actions release workflows (`cd.yml`, `release.yml`) must operate without stdio deadlocks, path leaks, or shell/permission failures.

**Acceptance Scenarios**:

1. **Given** an MCP `record_values` tool call with inline `content: "revenue,250\n"`, **When** `RgtMcpServer::record_values` executes, **Then** `execute_record_with_content` parses the in-memory `content` string directly without reading from the MCP server's `stdin`, and `get_status` runs `sync_staleness` before returning node counts.
2. **Given** a Turtle export (`rgt graph --format ttl`) on a project with deleted files or `<stdin>` records, **When** `source_location` resolves paths against the canonical project root, **Then** deleted project-internal files retain `rgt:relativePath` (with leading `./` stripped), while `"<stdin>"` emits neither `rgt:relativePath` nor `rgt:absolutePath`.
3. **Given** `install.sh`, `.github/workflows/cd.yml`, and `.github/workflows/release.yml`, **When** executed in CI/CD or POSIX `sh`, **Then** `install.sh` contains zero `local` bashisms and enforces SHA-256 verification + atomic `mv -f`, `cd.yml` creates and pushes `v$VERSION` before promoting `develop` to `main`, and `release.yml` updates all per-platform `url`/`sha256` blocks in `Formula/rgt.rb`.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `DbStore::init_in_project` MUST initialize `.rgt/store.db` in the target directory without walking up ancestor directories, while `DbStore::open_in_project` MUST walk up ancestor directories and `DbStore::normalize_path_for_project` MUST rebase subdirectory relative paths to the project root.
- **FR-002**: `record_file_batch` MUST invalidate missing root nodes across both path and `(dev, ino)` matches on content hash change, and MUST skip self-invalidation when `file_path == "<stdin>"`.
- **FR-003**: `execute_derive_core` and `insert_or_reuse_duration` MUST propagate `is_stale = true` (`PARENT_NODE_STALE`) whenever any parent node is stale.
- **FR-004**: `extract_numbers` MUST preserve multi-group US thousands integers (`1,000,000`) in prose while splitting unspaced CSV rows (`10,200,300`, `10,1.5e10`), and MUST reject non-unit alphanumeric suffixes, hex literals, git SHAs, and UUID segments.
- **FR-005**: `approx_eq` MUST reject opposite-sign non-zero numbers and enforce `diff <= 1e-9 * largest`.
- **FR-006**: `RgtMcpServer::record_values` MUST process inline `content` in memory via `execute_record_with_content`, and `RgtMcpServer::get_status` MUST run `sync_staleness`.
- **FR-007**: `docker/Dockerfile.harnesses` MUST install both Python agent CLIs (`mistral-vibe` $\rightarrow$ `/usr/local/bin/vibe` and `hermes-agent` $\rightarrow$ `/usr/local/bin/hermes`) alongside the 7 NPM agent CLIs and 4 GUI/rules-tier shims so that container test runs execute with 0 skipped harnesses.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass with 0 warnings and 0 errors.
- **SC-002**: `cargo test --all` passes 100% of unit, contract, and integration test suites.
- **SC-003**: Running `./scripts/test-harnesses-e2e.sh` inside the `rgt-harness-env` Docker container passes all **58 / 58** tests (`58 passed, 0 failed, 0 skipped`).
