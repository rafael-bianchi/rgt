# Feature Specification: Pass-2 Adversarial Critical Review Remediation

**Feature Branch**: `fix/052-adversarial-review-remediation`

**Created**: 2026-10-17

**Status**: Draft

**PriorityOrder**: Complete Pass-2 Adversarial Review Remediation (Spec 052)

**Input**: Complete findings from the Pass-2 Adversarial Critical Review (`critical_review.md` §§1–3) covering live 13-agent harness compatibility, shell/payload parsing, root & derived node identity invariants, extraction false-positive elimination, expression verification AST/token hardening, SQLite `IMMEDIATE` transaction & `sync_staleness` atomicity, MCP/CLI/updater hardening, and CI/CD & E2E test suite integrity.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Live 13-Agent Harness Compatibility, Shell Parsing & Config Editor Hardening (Priority: P1)

When any of the 13 supported AI coding harnesses executes a native file read or shell read command, or when a user runs `rgt init` / `rgt doctor`, RGT must handle real-world live payloads (including Claude Code `cat -n` line-numbered `Read` outputs, OpenCode `<file>\n00001| ...` outputs and relative/absolute path normalization, Codex CLI `shell`/`local_shell`/`exec_command` with array `cmd` and `output`, and CLI `event_type` fallback), avoid 800ms watchdog stalls on `bash`, support upstream config paths across Gemini CLI, Mistral Vibe, Hermes, Pi, Windsurf, Cursor, Copilot, and Cline/Roo Code while retaining backwards compatibility for existing RGT configs, and protect config files against JSONC comment corruption and `.rgt.bak` overwrite.

**Why this priority**: Fixes 100% capture drops on live Claude Code, OpenCode, and Codex CLI payloads, eliminates 800ms shell hook stalls, and ensures real upstream harness configurations work alongside existing contract tests.

**Independent Test**:
1. Feed a Claude Code `PostToolUse` `Read` payload containing `cat -n`-prefixed lines (`"     1→Metric: 1250\n     2→Cost: 250\n"`) to `rgt hook post --agent claude-code`; verify the prefixes are stripped, the file snapshot matches on disk, and only `1250` and `250` (not line numbers `1` and `2`) are recorded.
2. Feed an OpenCode `tool.execute.after` payload with `<file>\n00001| Metric: 1250\n</file>` or mismatched relative/absolute `args.filePath` vs `display.path`, plus an OpenCode `bash` read event; verify both capture values cleanly.
3. Feed a Codex CLI `PostToolUse` payload with `tool_name: "local_shell"`, `tool_input: {"cmd": ["bash", "-lc", "cat sample.txt"]}`, and `tool_response: {"output": "..."}`; verify capture succeeds.
4. Measure `posix_direct_hook_command` execution time under `bash` and verify it completes in `<100ms` without waiting for the 800ms watchdog timer.

**Acceptance Scenarios**:

1. **Given** a Claude Code `Read` response with `cat -n` prefixes (`^\s*\d+[→\t]`) or content blocks, **When** `rgt hook post --agent claude-code` runs, **Then** RGT strips the line-number prefixes, verifies the unwrapped content against `matching_file_snapshot`, maps extracted values to their true source line numbers, and never extracts line numbers as `ValueData::Number` nodes.
2. **Given** a hook payload that omits `event`/`hook_event_name` in JSON stdin, **When** invoked via `rgt hook post`, **Then** `parse_event` falls back to the CLI `event_type` (`"post"` $\rightarrow$ `CapturePhase::PostToolUse`).
3. **Given** an OpenCode `read` event with relative `args.filePath` and absolute `display.path` (or `<file>\n00001| ...` in `output`), or an OpenCode `bash` tool event, **When** `normalize_agent_event` runs, **Then** path comparison normalizes both paths, `<file>` line-numbered output is unwrapped and snapshot-verified, and `command_dialect` (`bash`) is allowed when `opencode_native_read_dialect` does not apply.
4. **Given** a Codex CLI `shell`, `local_shell`, `exec_command`, or `container.exec` event with array `cmd`/`command` (`["bash", "-lc", "cat file.txt"]`) and `tool_response.output`, **When** parsed, **Then** `is_qualifying_read_event` and `command_dialect` extract the inner command string and output text.
5. **Given** shell commands with quoted paths (`cat "my report.txt"`), env prefixes (`FOO=1 cat file.txt`), compound segments (`cd sub && cat file.txt`), `/bin/cat`, `bat`/`sed`/`head`/`tail`, or `python3 -c "..."`, **When** `extract_path_from_command` runs, **Then** quoted paths and compound commands resolve accurately (using payload `cwd` when present) while `python3 -c "..."` code strings are never misidentified as file paths.
6. **Given** `posix_direct_hook_command` running under `bash`, **When** `rgt hook` exits before 800ms, **Then** the watchdog subshell traps `TERM`/`INT` (killing its child `sleep` immediately) so `wait "$_rgt_watchdog"` returns without stalling for 800ms.
7. **Given** a JSONC file with a `//` line comment on the line immediately preceding `}` or `]`, **When** `jsonc_merge_hook_entries` splices new entries, **Then** it inserts a leading newline before the comma/property so the splice never lands inside the `//` comment; `merge_array` also compares and updates `matcher`, and `backup_before_write` preserves an existing `<path>.rgt.bak` instead of overwriting the original backup.
8. **Given** `rgt doctor`, **When** only static markdown guidance (`RegistrationHealth::GuidanceOnly`) is present without any active hook or plugin, **Then** `doctor` does not report active hook registration as satisfied (`hooks_present`), and `roo-code` vs `cline` do not emit duplicate diagnostics for `.clinerules`.

---

### User Story 2 - Root & Derived Node Identity Stability & Lineage Order Preservation (Priority: P1)

When files are edited or re-recorded, or when derived values are created, queried, or exported to Turtle, RGT must scope intra-line `occurrence` per `(value_type, canonical_value)`, reconcile pure line-number shifts without invalidating untouched values or hijacking adjacent root IDs, include `expression` and length delimiters in `generate_derived_id`, reject duplicate parent IDs in `EXPRESSION`, and preserve declared parent operand order in `rgt query`.

**Why this priority**: Prevents silent provenance corruption where adding a date or line invalidates unrelated values, prevents formula overwrites on derived ID collisions, and preserves non-commutative operand order (`a - b`, `date2 - date1`).

**Independent Test**:
1. Record a line `Revenue: 500`, then edit it to `Revenue: 500 on 2026-01-01` or `Cost: 100, Revenue: 500`; verify `500` keeps `occurrence = 0` and its root node ID remains unchanged and fresh.
2. Insert a blank line at the top of a tracked file and re-record; verify existing root nodes update their `line_number` in place without invalidating downstream derivations or hijacking another line's root ID.
3. Derive `a + b = 4` and `a * b = 4` from parents with values `2` and `2`; verify they produce distinct derived node IDs and neither overwrites the other's `expression`.
4. Run `rgt derive --parents id1,id1 --operation EXPRESSION --expression "a + b" --result 4`; verify duplicate parent IDs are rejected with exit code `2`.
5. Run `rgt query` on a `DATE_DIFF` or `EXPRESSION` node and verify `parents` are returned in their declared operand order (`id_z, id_a`), not sorted alphabetically.

**Acceptance Scenarios**:

1. **Given** a line containing multiple numbers and dates, **When** `extract_values_from_content` assigns `occurrence`, **Then** `occurrence` is counted per `(value_type, canonical_value)` on that line (`0` for the first occurrence of that specific value on the line, `1` for the second identical value on the same line).
2. **Given** a tracked file where lines are inserted or deleted above unchanged values, **When** `record_file_batch` runs, **Then** unchanged values matched by `(value_type, canonical_value, file_occurrence)` preserve their existing root node ID and update `line_number` in place without false invalidation or cross-line ID hijacking.
3. **Given** `TrackedNode::generate_derived_id`, **When** computing a derived node ID for `EXPRESSION`, **Then** it uses length-delimited hashing over `parents`, `op`, `val`, and the normalized `expression` (when present) so distinct formulas yielding the same numeric value never collide.
4. **Given** `--parents id1,id1 --operation EXPRESSION`, **When** `verify` or `derive` runs, **Then** duplicate parent node IDs are rejected as `InvalidInput` (exit code `2`).
5. **Given** a derived node with parents `[id_z, id_a]`, **When** `rgt query` (human or `--json`) inspects the node, **Then** `parents` preserves the declared edge order (`[id_z, id_a]`) instead of sorting alphabetically.

---

### User Story 3 - Extraction False-Positive Elimination & Expression AST Validation (Priority: P1)

When RGT extracts numbers and dates from files or validates `EXPRESSION` formulas, it must not split European decimals (`12,34`) into two numbers in `Auto`/`Us` mode, must not parse IPv4 addresses (`192.168.100.100`) or multi-dot version strings (`1.100.200`) as giant numbers in `Eu` mode, must mask colons in timestamps (`14:30:00`), slash dates (`05/17/2026`), URLs (`https://...`), and compact ISO offsets (`+0200`), must reject UUIDs starting with scientific-looking hex (`81e6-4faf-...`) and hex tokens ending in unit letters (`5678b`), must not turn `(a)-200` into `-200`, and must reject `evalexpr` semicolon (`;`) and string literal (`"..."`) bypasses in `validate_parent_variable_references`.

**Why this priority**: Eliminates severe extraction hallucinations (splitting `12,34` into `12` and `34` or turning IP addresses into `192168100100`) and closes the `evalexpr` unused-parent bypass (`"\"b\"; a"`).

**Independent Test**:
1. Run `extract_values_from_content` on `Price: 12,34 EUR` in `Auto` and `Us` modes; verify `12,34` is parsed as `12.34` in `Auto` mode and rejected as malformed (not split into `12` and `34`) in `Us` prose mode.
2. Run `extract_values_from_content` in `Eu` mode on `192.168.100.100` and `v1.100.200`; verify neither is extracted as a number.
3. Run `extract_values_from_content` on `14:30:00`, `2026-01-15T14:30:00+0200`, `https://example.com:8080/v2/item_1`, `81e6-4faf-1234-5678b`, and `(a)-200`; verify timestamps/URLs/UUIDs are not shattered into spurious numbers and `200` in `(a)-200` is positive `200.0`.
4. Run `verify_expression` on `"\"b\"; a"` and `"b; a"` with 2 parents; verify both are rejected as invalid expressions.

**Acceptance Scenarios**:

1. **Given** `Price: 12,34 EUR` (single comma with 1–2 decimal digits in prose), **When** parsed in `Auto` mode, **Then** `resolve_auto_format` detects unambiguous European decimal tokens (or `Us` mode rejects non-3-digit comma groups in prose as `Malformed` rather than splitting them as CSV cells unless the line is an actual multi-cell CSV/table row).
2. **Given** `192.168.100.100` (4 dot-separated octets) or version strings (`v1.100.200`), **When** parsed in `Eu` mode, **Then** IPv4 addresses and version tokens are skipped rather than parsed as thousands-grouped numbers.
3. **Given** `2026-01-15T14:30:00+0200` (compact `[+-]\d{4}` ISO-8601 offset), standalone times (`14:30:00`), URLs (`https://...` / `http://...`), or UUIDs/hex strings (`81e6-4faf-...`, `0x...`), **When** `extract_values_from_content` runs, **Then** compact ISO offsets are parsed with their true timezone offset, URLs and standalone `HH:MM:SS` times are masked, UUIDs are ignored, and `-` after `)` or `]` (`(a)-200`, `[x]-200`) is treated as binary subtraction (`+200.0`).
4. **Given** an `EXPRESSION` containing string literals (`"..."`), semicolons (`;`), comparison/boolean operators, or excessive operator chains, **When** `validate_expression` and `validate_parent_variable_references` run, **Then** string literals and statement separators (`;`) are rejected and every bound parent variable must be referenced as a real variable token, with `-0.0` canonicalized to `0.0`.

---

### User Story 4 - SQLite `IMMEDIATE` Transactions, Atomic `sync_staleness` & Store Hygiene (Priority: P1)

When parallel hooks or CLI commands write to `.rgt/store.db` or run `sync_staleness`, RGT must use `TransactionBehavior::Immediate` for all write transactions (preventing `SQLITE_BUSY_SNAPSHOT` on read-to-write upgrades), execute `sync_staleness` inside a single atomic transaction with deduplicated downstream traversal and deadline awareness, eliminate N+1 `SELECT` loops in `list_all_nodes` / `list_stale_nodes` / `list_nodes_by_source_doc`, and avoid creating `.rgt/store.db` on disk when read-only commands (`status`, `query`, `graph`, `verify`, `gc`) run in an uninitialized directory.

**Why this priority**: Prevents silent capture drops during parallel tool calls, prevents partial staleness corruption on interruption, eliminates 10,000-query N+1 bottlenecks, and stops read-only commands from polluting uninitialized directories with `.rgt/store.db`.

**Independent Test**:
1. Spawn 10 concurrent threads/processes running `record_file_batch` against the same `.rgt/store.db`; verify 100% succeed without `SQLITE_BUSY_SNAPSHOT`.
2. Run `rgt status`, `rgt query node_1`, `rgt graph`, `rgt verify ...`, and `rgt gc` in a temporary directory without `.rgt/store.db`; verify `.rgt/store.db` is **not** created on disk.
3. Verify `list_all_nodes`, `list_stale_nodes`, and `list_nodes_by_source_doc` fetch all columns in a single SQL query rather than N+1 queries.

**Acceptance Scenarios**:

1. **Given** write transactions in `hooks/mod.rs`, `cli/record.rs`, `cli/derive.rs`, `cli/gc.rs`, and `graph/invalidation.rs`, **When** opened on `rusqlite::Connection`, **Then** they use `TransactionBehavior::Immediate` so SQLite waits up to `busy_timeout` instead of failing immediately with `SQLITE_BUSY_SNAPSHOT`.
2. **Given** `sync_staleness` (`evaluate_and_invalidate_all`), **When** source files have changed or been deleted, **Then** all `source_documents` updates and `mark_node_stale` cascades execute atomically within a transaction, skipping already-visited downstream nodes (`if invalidated.insert(...)`), and checking the optional deadline during Turtle export.
3. **Given** `list_all_nodes`, `list_stale_nodes`, and `list_nodes_by_source_doc` in `src/store/queries.rs`, **When** loading nodes, **Then** each function executes a single `SELECT` query mapping rows directly via `row_to_tracked_node`.
4. **Given** an uninitialized directory (where `DbStore::find_project_root` returns `None`), **When** a read-only command (`status`, `query`, `graph`, `verify`, `gc`) or read-only MCP tool runs, **Then** RGT does not create `.rgt/store.db` on disk (returning an empty store view for `status`/`graph`/`gc` or node-not-found for `query`/`verify`).

---

### User Story 5 - MCP Server, CLI, Updater & CI/CD Pipeline Hardening (Priority: P1)

When RGT runs MCP tools, CLI commands (`record`, `graph`, `gc`, `update`), `install.sh`, or GitHub Actions workflows (`ci.yml`, `cd.yml`, `release.yml`, `turtle-performance.yml`, `test-harnesses-e2e.sh`), it must compute BLAKE3 hashes from the actual recorded content when inline/stdin content differs from disk, support `unit`/`result_unit`/optional `result` and `export_graph` in MCP, reject invalid `rgt graph --format` values with exit code `2`, escape Mermaid/DOT labels, validate and bound updater HTTP downloads and tags, eliminate GitHub Actions `${{ ... }}` script injection and `checksums.txt` self-hashing, and make the PDF and live CLI checks in `scripts/test-harnesses-e2e.sh` strictly verifiable.

**Why this priority**: Closes provenance poisoning via mismatched inline content hashes, completes MCP feature parity, hardens updater and CI/CD security, and removes false-positive E2E tests.

**Independent Test**:
1. Call MCP `record_values` (or `rgt record <file> --stdin`) with inline content; verify the stored BLAKE3 hash matches the inline content bytes and `rgt graph --format bogus` exits with code `2`.
2. Call MCP `derive_and_verify` for `DATE_DIFF` without `result` and with `unit: "days"`, and call MCP `export_graph`; verify both succeed.
3. Run `cargo test --all` and run `./scripts/test-harnesses-e2e.sh` inside `rgt-harness-env`.

**Acceptance Scenarios**:

1. **Given** `execute_record_with_content` called with explicit `content` (via MCP `record_values` or `rgt record <file> --stdin`), **When** computing `hash`, **Then** `hash` is always computed from `text.as_bytes()` (the content actually extracted), and `meta` is only attached if the on-disk file bytes match `text.as_bytes()`.
2. **Given** `RgtMcpServer`, **When** `derive_and_verify`, `verify_only`, `query_provenance`, or `export_graph` is called, **Then** `result` accepts exact string or number claims (optional for `DATE_DIFF`/`DURATION_SUM`/`DURATION_AVG`), `unit` and `result_unit` are supported, `export_graph` exports `text`/`mermaid`/`dot`/`ttl`, and target file paths anchor project-root discovery.
3. **Given** `rgt graph --format <fmt>`, **When** `<fmt>` is not one of `text`, `mermaid`, `dot`, or `ttl`, **Then** RGT exits with code `2` (`InvalidInput`), propagates any DB errors from `get_child_edges` and `gc`, escapes quotes/brackets in Mermaid and DOT output, and prints `skipped_malformed` warnings to `stderr`.
4. **Given** `rgt update`, **When** `--version <tag>` is passed, **Then** `<tag>` is validated before URL construction, HTTP responses and Snappy decompression are size-bounded, all entries in `.attestations` are checked until one matches `rgt_policy()`, and `Authorization` headers are not leaked to cross-origin asset URLs.
5. **Given** `.github/workflows/cd.yml`, `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/workflows/turtle-performance.yml`, `install.sh`, and `scripts/test-harnesses-e2e.sh`, **When** inspected and executed, **Then** all `${{ ... }}` expressions in `run:` blocks use `env:` variables, `release.yml` excludes `checksums.txt` when running `sha256sum`, `install.sh` matches exact filenames in `checksums.txt`, `ci.yml` runs on `push` and `pull_request` (including `check-readme-parity.sh` and `check-version-state.sh`), and `test-harnesses-e2e.sh` fails if PDF value `7777` is not extracted.

---

### Edge Cases

- **Claude Code / OpenCode Line-Numbered Reads**: `"     1→Revenue: 1250"` and `"<file>\n00001| Revenue: 1250\n</file>"` unwrap to `"Revenue: 1250\n"` before snapshot verification and value extraction.
- **Pure Line Shifts vs. Modified Values**: Inserting a comment line at line 1 shifts a value from line 2 to line 3; its existing root node updates `line_number = 3` in place while keeping its `node_raw_*` ID and downstream derivations intact.
- **European Decimals vs. CSV Rows**: `Price: 12,34 EUR` is never split into `12` and `34`, while `10,20,30` in a CSV row still splits into `[10.0, 20.0, 30.0]`.
- **Evalexpr Statement Bypass**: `"\"b\"; a"` and `"b; a"` are rejected by expression validation.
- **Read-Only Commands in Uninitialized Directories**: Running `rgt status` in `/tmp` does not create `/tmp/.rgt/store.db`.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `parser.rs` and `hooks/mod.rs` MUST unwrap line-numbered `Read` tool outputs from Claude Code (`^\s*\d+[→\t]`) and OpenCode (`<file>\n00001| ...`), fall back to the CLI `event_type` argument when the JSON payload omits an event phase field, normalize paths when comparing `args.filePath` vs `display.path` in OpenCode, allow OpenCode `bash` reads via `command_dialect`, and support Codex CLI tool names (`shell`, `local_shell`, `exec_command`, `container.exec`), array `cmd`/`command`, and `tool_response.output`.
- **FR-002**: `normalize_agent_event` MUST permit `content_complete = true` for any agent payload whose extracted content passes `matching_file_snapshot` against the on-disk file; `installer.rs`, `paths.rs`, `registration.rs`, and `glue/mod.rs` MUST retain `PreToolUse` and `PostToolUse` passive hook compatibility per Constitution Principle VI.1 while supporting real upstream hook/plugin formats for Gemini CLI (`.gemini/settings.json` `AfterTool` alongside `.gemini/hooks.toml`), Mistral Vibe (`post_tool` in `.vibe/config.toml` and `.vibe/hooks.toml`), Hermes (`plugin.yaml` + `__init__.py` + scope-aware global guidance path), Pi (`pi.on("tool_result", ...)`), Windsurf (`.windsurf/hooks.json`), and Roo Code (`.roo/rules/` distinct from Cline `.clinerules`), eliminating duplicate `cline`/`roo-code` doctor rows and keeping `docs/compatibility/support-matrix.md` aligned per Principle X.
- **FR-003**: `extract_path_from_command` MUST properly tokenize quoted paths with spaces, strip env assignments and leading `cd <dir> &&` compounds, resolve relative paths against payload `cwd` when present, and never treat `python -c "..."` code arguments as file paths; `posix_direct_hook_command` MUST trap `TERM` and `INT` in its watchdog subshell so `bash` hooks do not stall for 800ms.
- **FR-004**: `jsonc_merge_hook_entries` MUST avoid splicing inside trailing `//` line comments, `merge_array` MUST compare and update `matcher` (and honor `--force`), `backup_before_write` MUST NOT overwrite an existing `<path>.rgt.bak`, and `rgt doctor` MUST NOT count `RegistrationHealth::GuidanceOnly` as active hook presence.
- **FR-005**: `extract_values_from_content` MUST scope intra-line `occurrence` per `(value_type, canonical_value)` rather than sharing a single counter across all dates and numbers on a line, and `record_file_batch` MUST update `line_number` in place for unchanged values that shifted lines rather than invalidating them or hijacking adjacent root IDs.
- **FR-006**: `TrackedNode::generate_derived_id` MUST use length-delimited hashing and include the normalized `expression` when present (while `classify_derivation_group` in `src/export/turtle.rs` accepts both the new expression-included length-delimited ID and the legacy `generate_derived_id` hash for backward compatibility with existing stores); `EXPRESSION` derivations MUST reject duplicate parent IDs (`InvalidInput`, exit code `2`); and `rgt query` MUST preserve declared parent operand order instead of sorting `parent_ids` alphabetically.
- **FR-007**: `extract_values_from_content` MUST NOT split prose European decimals (`12,34`) into two numbers in `Auto`/`Us` mode, MUST NOT parse IPv4 addresses (`192.168.100.100`) or multi-dot version strings (`v1.100.200`) as numbers in `Eu` mode, MUST support compact ISO-8601 timezone offsets (`[+-]\d{4}`), MUST mask URLs (`https://...`) and standalone `HH:MM:SS` times, MUST reject UUIDs/hex tokens (`81e6-4faf-...`, `5678b`), and MUST treat `-` after `)` or `]` as binary subtraction (`+200.0`).
- **FR-008**: `validate_expression` and `validate_parent_variable_references` MUST reject `evalexpr` statement separators (`;`), string literals (`"..."`), and excessive operator chains (> 256 operators), verify parent variable references only on real identifier tokens, and canonicalize `-0.0` to `0.0` in `verify_expression`.
- **FR-009**: All SQLite write transactions across `hooks/mod.rs`, `cli/record.rs`, `cli/derive.rs`, `cli/gc.rs`, and `graph/invalidation.rs` MUST use `TransactionBehavior::Immediate`, and `execute_derive_core` MUST verify parent staleness inside the write transaction.
- **FR-010**: `sync_staleness` (`evaluate_and_invalidate_all`) MUST execute its `source_documents` updates and `mark_node_stale` cascades inside an atomic transaction with deduplicated downstream traversal and optional deadline enforcement; `list_all_nodes`, `list_stale_nodes`, and `list_nodes_by_source_doc` MUST use single-query row mapping instead of N+1 `get_tracked_node` calls.
- **FR-011**: Read-only CLI commands (`status`, `query`, `graph`, `verify`, `gc`) and read-only MCP tools MUST NOT create `.rgt/store.db` on disk when run in an uninitialized directory.
- **FR-012**: `execute_record_with_content` MUST always compute `blake3_hash` from the recorded `text` bytes (only attaching on-disk `meta` when disk bytes match `text`), and `RgtMcpServer` MUST support optional `result`, `unit`, `result_unit`, string/numeric `result` claims without `f64` precision loss, `export_graph`, and file-anchored project root discovery.
- **FR-013**: `rgt graph --format` MUST reject unknown formats with exit code `2`, propagate DB errors in `get_child_edges` and `gc`, escape Mermaid and DOT labels, and emit `skipped_malformed` warnings in `rgt record` to `stderr`.
- **FR-014**: `rgt update` MUST validate `--version <tag>` before constructing GitHub API URLs, enforce explicit byte caps on HTTP responses (`64 MiB` max for release asset downloads; `16 MiB` max for GitHub API JSON, Sigstore bundle downloads, and Snappy decompressed payloads), iterate across all `.attestations` entries until one verifies against `rgt_policy()`, and omit `Authorization` headers on cross-origin `browser_download_url` requests.
- **FR-015**: `.github/workflows/cd.yml` and `.github/workflows/release.yml` MUST pass all `${{ ... }}` inputs via `env:` variables, `release.yml` MUST avoid hashing `checksums.txt` into itself, `install.sh` MUST match exact archive filenames in `checksums.txt`, `.github/workflows/ci.yml` and `.github/workflows/turtle-performance.yml` MUST run on `develop`/`main` with README parity and version-state checks, `docs/compatibility/support-matrix.md` MUST match the verified installer/parser behavior, and `scripts/test-harnesses-e2e.sh` MUST strictly assert PDF value `7777` capture and subshell failure propagation.

### Key Entities

- **`NormalizedCapture` & `ExtractedValue`**: Represent parsed hook events (with unwrapped content and source line mappings) and extracted numbers/dates with per-value line `occurrence` and file-level shift reconciliation.
- **`TrackedNode` & `DerivationEdge`**: Represent root and derived provenance nodes with collision-resistant length-delimited BLAKE3 IDs and ordered parent edges.
- **`DbStore` & `InvalidationOrchestrator`**: Manage `IMMEDIATE` SQLite transactions, non-polluting read-only store checks, and atomic staleness synchronization.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings` pass with 0 warnings and 0 errors.
- **SC-002**: `cargo test --all` passes 100% of unit, contract, and integration test suites, including new regression tests for every Pass-2 finding (`FR-001`–`FR-015`).
- **SC-003**: Rebuilding `rgt-harness-env` and running `./scripts/test-harnesses-e2e.sh` inside the container passes 100% of E2E checks with `0 failed` and `0 skipped`.
