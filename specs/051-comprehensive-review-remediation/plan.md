# Implementation Plan: Spec 051 - Comprehensive Critical Review Remediation & Verification Hardening

## Overview
Consolidates the architecture, design decisions, and verification strategy for all findings from the critical code review (`critical_review.md` §§1–6), the post-review verification audit, and the 13-agent Docker testing environment (`docker/Dockerfile.harnesses`).

## Architecture & Component Changes

### 1. Store, Schema, Staleness & Path Rebasing (`src/store/`, `src/graph/`, `src/detection/`)
- **`src/store/db.rs`**:
  - `DbStore::find_project_root`: Walks up parent directories to locate `.rgt/store.db`.
  - `DbStore::init_in_project`: Creates `.rgt/store.db` directly in the requested directory for `rgt init` without attaching to an ancestor store.
  - `DbStore::normalize_path_for_project`: Rebases relative paths recorded from a subdirectory onto `project_root` while preserving `file_path` verbatim when `cwd == project_root`.
  - `DbStore::open_with_timeout`: Validates `schema_version(&conn) <= SCHEMA_VERSION` before running `initialize_schema`.
- **`src/store/schema.rs`**:
  - `SCHEMA_VERSION = 3`: Migrates v1/v2 `tracked_nodes` tables by adding `duration_secs INTEGER` and updating `CHECK(value_kind IN ('NUMBER', 'DATE', 'DURATION'))` and `CHECK(is_stale IN (0, 1))`, skipping table rebuilds when v3 columns already exist.
- **`src/store/queries.rs`**:
  - `upsert_source_document`: Resolves `(dev, ino)` aliases in-place without triggering `ON DELETE SET NULL` on `tracked_nodes.source_doc_id`.
  - `record_file_batch`: Diffs incoming root node IDs against existing root nodes (by path and `(dev, ino)`), marks removed/changed nodes `SOURCE_FILE_MODIFIED`, cascades `PARENT_NODE_STALE`, and skips self-invalidation on `"<stdin>"`.
  - `insert_derivation_edge`: Uses `ON CONFLICT(parent_node_id, child_node_id) DO UPDATE SET operation_type = excluded.operation_type, expression = excluded.expression`.
  - `delete_obsolete_nodes`: Iteratively peels stale leaf nodes until reaching a fixed point and cleans up orphaned `source_documents` rows for deleted files.
- **`src/cli/derive.rs` & `src/store/queries.rs`**:
  - Propagates `any_parent_stale` (`is_stale = true`, `stale_reason = Some("PARENT_NODE_STALE")`) to newly created or reused derived nodes.

### 2. Value & Date Extraction (`src/hooks/parser.rs`)
- **`iso_date_regex`**: Matches full ISO-8601/RFC-3339 spans (including `[tT]`, `[zZ]`, `[+-]HH:MM`) and masks invalid calendar dates (`2026-02-30`) so they never shatter into numbers.
- **`extract_numbers` & `is_allowed_unit_suffix`**:
  - Preserves negative signs across currency symbols (`-$1,200.00`, `-€500`) and rejects multi-hyphen CLI flags (`--100`).
  - Parses scientific notation (`1.5e10`, `1.5e-4`) including inside CSV rows (`10,1.5e10`).
  - Splits unspaced CSV rows (`10,200,300`, `1.5,200`) while keeping prose millions/billions (`1,000,000`, `$12,345,678`) intact.
  - Validates trailing word boundaries against `is_allowed_unit_suffix` and rejects hex strings (`0x1a2b`, `7e3a9f1`), UUID segments, and ordinals (`1st`).
- **`normalize_number`**: Rejects multiple decimal points (`if decimal_seen { return None; }`), rejects `!val.is_finite()`, and canonicalizes `-0.0` to `0.0`.

### 3. Verification & Duration Formatting (`src/verify/expression.rs`, `src/types/value.rs`)
- **`validate_parent_variable_references`**: Ensures every parent variable `a..` is referenced as an identifier token in `EXPRESSION`.
- **`approx_eq`**: Rejects opposite signs when both numbers are non-zero and uses `diff <= 1e-9 * largest`.
- **`ValueData::Duration::to_string_repr`**: Includes non-zero `secs` even when `days > 0`.

### 4. 13-Agent Hook/Plugin Protocols, Editor & Docker Environment (`src/hooks/`, `docker/`)
- **`src/hooks/parser.rs`**: Supports Claude Code nested `tool_response.file.content` and OpenCode `text.split('\n').count()` line matching.
- **`src/hooks/glue/mod.rs`**: Removes `pre_tool_call` from Hermes `plugin.py`; updates `CLINE_RULES`, `ANTIGRAVITY_RULES`, `KILO_RULES`, and `COPILOT_CLI_RULES` with full RGT CLI feature parity.
- **`src/hooks/editor.rs`**: Validates structural object shape (`shapes_match`) in `merge_array` and preserves backslashes in double-quoted Windows paths in `shell_tokens`.
- **`src/hooks/installer.rs` & `src/cli/doctor.rs`**: Scope-aware `rgt init -g`, layered user+project `rgt doctor` evaluation, and `.claude/hooks.json.rgt.bak` backup on migration.
- **`docker/Dockerfile.harnesses`**: Installs all 7 NPM agent CLIs (`claude`, `opencode`, `codex`, `pi`, `kilo`, `copilot`, `gemini`), both Python agent CLIs in isolated venvs (`mistral-vibe` $\rightarrow$ `/usr/local/bin/vibe`, `hermes-agent` $\rightarrow$ `/usr/local/bin/hermes`), and 4 GUI/rules-tier shims (`cursor`, `windsurf`, `cline`, `agy`).

### 5. MCP Server, Turtle Export & Release Workflows (`src/mcp/`, `src/export/`, `.github/workflows/`, `install.sh`)
- **`src/mcp/mod.rs` & `src/cli/record.rs`**: `execute_record_with_content` handles inline `content` in memory without reading stdio; `get_status` calls `sync_staleness`.
- **`src/export/turtle.rs` & `src/cli/graph.rs`**: Resolves `project_root` via `DbStore::find_project_root`, excludes `"<stdin>"` from `source_location`, and preserves clean `rgt:relativePath` for deleted project-internal files.
- **`install.sh`, `.github/workflows/cd.yml`, `.github/workflows/release.yml`**: POSIX-compliant `install.sh` with mandatory checksum verification and atomic `mv -f`; `cd.yml` tag creation before `main` promotion; `release.yml` multi-block `Formula/rgt.rb` updater.
