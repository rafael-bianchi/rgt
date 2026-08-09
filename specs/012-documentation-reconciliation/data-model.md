# Data Model: Documentation Reconciliation

**Feature**: `012-documentation-reconciliation` | **Date**: 2026-08-08

## Overview

No runtime data model changes. This feature reconciles documentation artifacts against the codebase. The "entities" below are documentation artifacts, not runtime data.

## Entities

### CLI Surface (single source of truth)

**Source**: `src/main.rs` `Commands` enum.

| Command | Flags | Documented in README? |
|---|---|---|
| `init` | `-g`, `--force`, `--agent` | ✅ |
| `status` | `--stale-only`, `--json` | ✅ |
| `query` | `<node_id>`, `--json` | ✅ |
| `graph` | `-f text\|mermaid\|dot` | ✅ |
| `hook` | `<pre\|post>` | ✅ |
| `mcp` | — | ✅ |
| `update` | `--check`, `-y`, `--version` | ✅ |
| `verify` | `--parents`, `--operation`, `--expression`, `--result` | ✅ |

### Public API Doc Comments

**Source**: `src/**/*.rs` `pub fn` declarations.

| Module | Functions Documented |
|---|---|
| `src/verify/` | `verify_expression`, `verify_date_diff`, `verify` |
| `src/updater/` | `parse_checksums`, `find_checksum`, `compute_sha256`, `verify_checksum`, `fetch_latest_release`, `fetch_release_by_tag`, `find_asset`, `find_checksum_asset`, `download_asset`, `verify_asset_checksum` |
| `src/cli/` | `execute_init`, `execute_status`, `execute_query`, `execute_graph`, `execute_update` |
| `src/hooks/` | `handle_passive_hook_event`, `parse_hook_payload`, `extract_values_from_content`, `detect_and_configure_hooks` |
| `src/detection/` | `evaluate_file_change`, `get_metadata_snapshot`, `tier1_check_changed`, `compute_blake3_hash` |
| `src/store/` | `initialize_schema`, `upsert_source_document`, `get_source_document_by_path`, `insert_tracked_node`, `get_tracked_node`, `insert_derivation_edge`, `get_child_edges`, `get_parent_edges`, `mark_node_stale`, `list_stale_nodes`, `list_all_nodes`, `list_nodes_by_source_doc` |
| `src/mcp/` | `handle_record_value`, `handle_record_derivation`, `handle_query_provenance`, `handle_list_stale_values` |

### Documentation Artifacts

| Artifact | Status | Purpose |
|---|---|---|
| `README.md` | ✅ Current | End-user onboarding |
| `AGENTS.md` | ✅ Current | AI agent development guide |
| `CONTRIBUTING.md` | ✅ Created | Contributor onboarding |
| `.gitignore` | ✅ Updated | Excludes `.kilocode/` |

## Validation Rules

- Every `pub fn` MUST have a preceding `///` line (script-verified).
- README MUST mention all 8 CLI subcommands.
- No `rafael-bianchi/tap` or bare `cargo install rgt` in any doc.
- `cargo doc --no-deps` MUST build with zero warnings.
