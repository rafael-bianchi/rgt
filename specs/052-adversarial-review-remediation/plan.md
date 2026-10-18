# Implementation Plan: Spec 052 - Pass-2 Adversarial Critical Review Remediation

## Overview
Remediates all findings from the Pass-2 Adversarial Critical Review (`critical_review.md` §§1–3), covering live 13-agent harness compatibility, shell/payload parsing, root & derived node identity stability, value extraction false-positive elimination, expression verification hardening, SQLite `IMMEDIATE` transaction & `sync_staleness` atomicity, MCP/CLI/updater security & completeness, and CI/CD & E2E test suite integrity.

## Architecture & Component Changes

### 1. Live 13-Agent Harness Compatibility, Shell Parsing & Config Editor (`src/hooks/`, `src/cli/doctor.rs`)
- **`src/hooks/parser.rs` & `src/hooks/mod.rs`**:
  - Unwrap Claude Code `cat -n`-style line prefixes (`^\s*(\d+)[→\t](.*)$`) and OpenCode `<file>\n00001| ...` wrappers before snapshot comparison (`matching_file_snapshot`) and value extraction, populating `source_line_mapping` from the parsed line numbers.
  - Pass an optional default `CapturePhase` from the CLI `event_type` (`"post"` $\rightarrow$ `CapturePhase::PostToolUse`) when the JSON payload omits an event phase key.
  - Normalize paths (`./` stripping / canonical suffix matching) when comparing `args.filePath` and `display.path` in `opencode_native_read_dialect`, and allow OpenCode `bash` reads via `command_dialect` when `opencode_native_read_dialect` returns `None`.
  - Extend `is_qualifying_read_event` and `command_dialect` for Codex CLI tool names (`shell`, `local_shell`, `exec_command`, `container.exec`), array `cmd`/`command` (`["bash", "-lc", "..."]`), and `tool_response.output`.
  - Allow `capture.content_complete = true` for any agent whose extracted content is verified against the on-disk file snapshot.
  - Harden `extract_path_from_command` to handle quoted paths with spaces, env var prefixes (`FOO=bar`), compound `cd <dir> && ...` segments, `/bin/` prefixes, `bat`/`sed`, and reject `python -c` / `python3 -c` inline scripts; honor payload `cwd` in `handle_passive_hook_event`.
  - Support `filePath` alias and `tool_response.file.filePath` fallback in `default_dialect` / PDF capture.
- **`src/hooks/installer.rs`, `src/hooks/paths.rs`, `src/hooks/registration.rs`, `src/hooks/glue/mod.rs`**:
  - Retain `PreToolUse` and `PostToolUse` hook compatibility per Constitution Principle VI.1 while adding `trap 'kill "$_rgt_timer" 2>/dev/null || true; exit 0' 0 TERM INT` inside the `posix_direct_hook_command` watchdog subshell so `bash` hooks exit in `<100ms` without stalling for 800ms.
  - Fix `write_hermes` to honor `global: bool` for guidance path resolution and emit `plugin.yaml` alongside `plugin.py`/`__init__.py`; update Vibe (`post_tool` + `.vibe/config.toml` support), Gemini (`.gemini/settings.json` `AfterTool` support alongside `.gemini/hooks.toml`), Pi (`pi.on("tool_result", ...)`), Windsurf (`.windsurf/hooks.json` preferred, `.windsurf`/`.devin` detection), and separate Roo Code (`.roo/rules/`) from Cline (`.clinerules`) so `rgt doctor` never emits duplicate rows for `.clinerules`.
- **`src/hooks/editor.rs` & `src/cli/doctor.rs`**:
  - Ensure `jsonc_merge_hook_entries` inserts a newline before `,` when the tail preceding `}` or `]` contains a `//` line comment without a trailing newline.
  - Compare and update `matcher` in `merge_array` (and honor `force` in `ensure_json_hooks`).
  - Preserve existing `<path>.rgt.bak` in `backup_before_write` so repeated runs never clobber the user's original pre-RGT config backup.
  - Separate `hooks_present` from `GuidanceOnly` in `rgt doctor`.

### 2. Root & Derived Node Identity & Lineage Order (`src/hooks/parser.rs`, `src/types/node.rs`, `src/store/queries.rs`, `src/cli/derive.rs`, `src/export/turtle.rs`, `src/query/mod.rs`)
- **`src/hooks/parser.rs`**:
  - Scope intra-line `occurrence` per `(value_kind, canonical_value_repr)` on each line instead of a single shared counter across all dates and numbers on that line.
- **`src/store/queries.rs`**:
  - In `record_file_batch`, reconcile pure line shifts: when an existing non-stale root node's `(value_kind, canonical_value, file_occurrence)` matches an incoming value whose `line_number` shifted, update its `line_number` in place and preserve its `id` (unless that `id` is claimed by an exact same-line match), preventing false invalidations and cross-line ID hijacking; also canonicalize root node `file_path` to the existing document's `file_path` on `(dev, ino)` alias matches and normalize `\` to `/` in project-relative paths.
- **`src/types/node.rs`, `src/cli/derive.rs`, `src/export/turtle.rs`, `src/query/mod.rs`**:
  - Update `TrackedNode::generate_derived_id` (and `generate_derived_id_with_expr`) to use length-delimited hashing and incorporate the normalized `expression` when present, while keeping backward-compatible fallback validation in `classify_derivation_group` (`src/export/turtle.rs`) for legacy `EXPRESSION` node IDs.
  - Reject duplicate parent IDs in `EXPRESSION` derivations (`InvalidInput`, exit code `2`).
  - Preserve `parent_edges` insertion/declaration order (`ORDER BY id ASC`) in `rgt query` instead of calling `parent_ids.sort()`.

### 3. Value Extraction False-Positive Elimination & Expression AST Validation (`src/hooks/parser.rs`, `src/verify/expression.rs`)
- **`src/hooks/parser.rs`**:
  - In `resolve_auto_format` and `extract_numbers`, detect unambiguous single-comma European decimals in prose (`Price: 12,34 EUR`) and prevent `Us` mode from splitting prose single-comma numbers into two numbers unless the token is an actual unspaced multi-value CSV/list token.
  - Reject IPv4 addresses (`\b\d{1,3}(?:\.\d{1,3}){3}\b`) and multi-dot version strings (`v?\d+\.\d+\.\d+`) in `Eu` and `Us` modes.
  - Extend `iso_date_regex` to accept compact ISO-8601 offsets (`[+-]\d{4}` as well as `[+-]\d{2}:\d{2}`), and mask URLs (`https?://\S+`), standalone `HH:MM(:SS)?` time tokens, slash dates (`\b\d{1,2}/\d{1,2}/\d{2,4}\b`), and UUID/hex spans before number extraction.
  - Treat `-` immediately following `)` or `]` (`(a)-200`, `[x]-200`) as binary subtraction (`+200.0`).
- **`src/verify/expression.rs`**:
  - Reject semicolons (`;`), quotes/string literals (`"`, `'`), assignment/comparison operators, and excessive operator chains (> 256 operators) in `validate_expression`, and canonicalize `-0.0` to `0.0` in `verify_expression`.

### 4. SQLite `IMMEDIATE` Transactions, Atomic `sync_staleness` & Store Hygiene (`src/store/`, `src/graph/`, `src/cli/`, `src/mcp/`)
- **`src/store/db.rs`, `src/hooks/mod.rs`, `src/cli/record.rs`, `src/cli/derive.rs`, `src/cli/gc.rs`**:
  - Use `TransactionBehavior::Immediate` for all write transactions (`record_file_batch`, `derive`, `gc`, `sync_staleness`).
  - Check parent staleness inside the `IMMEDIATE` write transaction in `execute_derive_core`.
  - Add `DbStore::open_existing_in_project` for read-only / maintenance commands (`status`, `query`, `graph`, `verify`, `gc`) so running them in an uninitialized directory does not create `.rgt/store.db` on disk.
- **`src/graph/invalidation.rs` & `src/store/queries.rs`**:
  - Wrap `evaluate_and_invalidate_all` updates in an `IMMEDIATE` transaction, deduplicate downstream traversal (`if invalidated.insert(child_id.clone())`), and support an optional `deadline: Option<Instant>` for `execute_turtle`.
  - Rewrite `list_all_nodes`, `list_stale_nodes`, and `list_nodes_by_source_doc` to select all 11 columns directly in a single SQL query using a shared `row_to_tracked_node` mapper, and use `i64::try_from` / `u64` safe conversions for `dev` and `ino`.

### 5. MCP Server, CLI, Updater & CI/CD Workflows (`src/mcp/`, `src/cli/`, `src/updater/`, `.github/workflows/`, `install.sh`, `scripts/`)
- **`src/cli/record.rs` & `src/mcp/mod.rs`**:
  - Always compute `blake3_hash` from `text.as_bytes()` when `explicit_content` or `--stdin` is provided; only attach `meta` from disk if the on-disk file's bytes equal `text.as_bytes()`.
  - Extend MCP `VerifyParams`, `DeriveParams`, and `QueryParams` with optional `result` (accepting string or number without `f64` truncation), `unit`, and `result_unit`; add `export_graph` MCP tool; anchor project-root discovery to the target `file` path when provided.
- **`src/cli/graph.rs` & `src/cli/gc.rs`**:
  - Validate `--format` against `["text", "mermaid", "dot", "ttl"]` (exiting `2` on unknown formats), propagate `get_child_edges` and `gc` SQL errors, escape `"`/`\`/`]` in Mermaid and DOT outputs, and print `skipped_malformed` warnings in `record.rs` to `stderr`.
- **`src/updater/` & `src/cli/update.rs`**:
  - Validate `--version <tag>` with `version::parse_version` before calling `fetch_release_by_tag` (returning exit code `2` on invalid semver), bound HTTP body reads (`64 MiB` max for release archives/checksums, `16 MiB` max for API JSON and Sigstore bundles) and Snappy decompression (`16 MiB` max), check all entries in `.attestations` until one matches `rgt_policy()`, and omit `Authorization` headers when downloading from `browser_download_url`.
- **`.github/workflows/`, `install.sh`, `scripts/`, `docs/compatibility/support-matrix.md`**:
  - Replace inline `${{ ... }}` in `cd.yml` and `release.yml` `run:` blocks with `env:` variables; fix `sha256sum` in `release.yml` so `checksums.txt` never hashes itself; use exact filename matching in `install.sh`; add `push` triggers and `check-readme-parity.sh` / `check-version-state.sh` to `ci.yml`; update `turtle-performance.yml` branches; update `docs/compatibility/support-matrix.md` to reflect verified harness details; and make `scripts/test-harnesses-e2e.sh` strictly fail if PDF value `7777` is not captured.
