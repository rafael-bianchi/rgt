# Research: Installer Config Preservation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-27

Consolidates the findings-doc research (§2, with citations) plus fresh verification of the candidate editing crates (2026-08-27, docs.rs + crates.io metadata). Resolves the plan's Technical Context unknowns.

## 1. JSONC parsing & byte-for-byte-preserving merge

**Decision**: Add `jsonc-parser` (dprint port, MIT, pure Rust) with the **`cst` feature**, and implement hook merging as a **text splice** using the AST `Ranged` offsets so that untouched bytes are copied verbatim.

**Rationale**:
- Claude Code `settings.json`, Cursor `hooks.json`, and Copilot Chat VS Code settings are JSON-family files that legitimately contain comments and trailing commas (VS Code settings are JSONC by convention). `serde_json` rejects both and re-serialization destroys all comments/formatting, so it cannot satisfy FR-001/SC-001 (byte-for-byte) or FR-010 (JSONC tolerance).
- Verified on docs.rs/jsonc-parser: `ParseOptions` exposes `allow_comments` and `allow_trailing_commas` (defaults on) — entry points `parse_to_value` / `parse_to_ast` / `parse_to_serde_value` / `CstRootNode::parse`. Every AST node implements `Ranged` with `range() -> Range` (start/end byte indices) — enabling precise, minimal text splices (docs.rs/jsonc-parser/latest/jsonc_parser/, .../ast/enum.Node.html).
- License MIT — compatible with RGT's Apache-2.0.

**Alternatives considered**:
- `serde_json` alone: rejected — no JSONC support; re-serialization is lossy.
- `json-patch` / RFC 6902: rejected — operates on an in-memory `serde_json::Value`; "no text/spacing concept," comments and formatting lost (docs.rs/json-patch).
- Hand-rolled JSONC comment-stripping: rejected — exactly the fragile string-surgery class of bug (§2.1/§2.2) this feature exists to eliminate; no AST offsets to locate the `hooks` span.
- `jsonc-parser`'s CST re-serialization (`CstRootNode` → `to_string()`): format-preserving for trivia (comments/whitespace/newlines are CST nodes) but re-serializes the whole document, so "byte-for-byte for untouched text" is not an explicit contract. **Decision: use the AST `Ranged` splice** for a hard byte-for-byte guarantee (SC-001); keep the CST API available as a fallback if the splice proves impractical in a specific shape.

**Notes for implementation**: the exact source of the VS Code `modify`/`applyEdits` algorithm does **not** exist in this Rust crate (that is the JS package); the splice must be built from `Ranged` offsets. Algorithm sketch: parse tolerant → locate the `hooks` subtree span → within the target `PostToolUse`/`PreToolUse` array, find nodes whose command string starts with `rgt hook` (existing RGT entries) → remove their spans and insert the fresh RGT entries at a deterministic position inside the array span → concatenate original text with only that region replaced.

## 2. TOML-safe plugin/table edits

**Decision**: Add `toml_edit` (MIT OR Apache-2.0, pure Rust) and replace `insert_into_enabled`'s string splice and `write_toml_block`'s block-append with document edits.

**Rationale**:
- Verified on docs.rs/toml_edit: "This crate allows you to parse and modify toml documents, while preserving comments, spaces and relative order of items." `Array::push` appends to `plugins.enabled`; `Table::entry` + `Entry::or_insert` creates the `[plugins]` table **only if absent** — duplicate `[plugins]` tables are prevented by construction (the table is keyed by unique keys, and a redefined header is a parse error), directly fixing §2.3's invalid-TOML bug.
- Multi-line arrays and comments are preserved, which the current `enabled = [` substring search cannot handle (§2.3 verified against toml.io/en/v1.0.0).

**Alternatives considered**:
- Plain `toml` crate: rejected — it deserializes to a semantic tree and rewrites the whole file, losing comments/formatting (findings §2.3 explicitly confirmed this tradeoff).
- String search/splice (current code): rejected — the bug being fixed.

## 3. Text/rules block replacement (paired markers)

**Decision**: Every RGT block written to a text/rules file ends with an explicit end marker (`<!-- /RGT Integration -->`, new `glue::RGT_END_MARKER`). `--force` replaces only the bytes between the open marker and the end marker; content below the block is preserved. When an existing block has the open marker but **no end marker** (legacy install), `--force` refuses to rewrite and errors loudly with a manual-remediation hint, because RGT cannot distinguish its own block from user content below it.

**Rationale**:
- Matches the findings-doc §2.4 suggested fix ("Locate both the marker's start line and a matching end marker (e.g. `<!-- /RGT -->`), and only replace the bytes between them") and prior art: conda's `CONDA_INITIALIZE_RE_BLOCK` replaces text strictly between two literal markers in user rc files (github.com/conda/conda/blob/main/conda/core/initialize.py).
- The legacy-block refusal is the only behavior that guarantees "never destroy user content" (SC-004) when the block boundary is unknowable.

**Alternatives considered**:
- `truncate_from_marker` (replace from open marker to EOF): rejected — deletes user content below the block (§2.4).
- Heuristic section-boundary detection (blank-line delimiters): rejected — unreliable on arbitrary user formatting; risk of partial user-content deletion.
- Always replace to EOF on `--force` and require users to keep notes above the block: rejected — RGT's own instructions tell agents to append to these files, so content below the block is an expected, supported layout.

## 4. Backup-before-write

**Decision**: A single `backup_before_write(path)` helper copies an existing file to `<path>.rgt.bak` immediately before any writer modifies it. Backups are never auto-deleted (spec assumption; SC-006).

**Rationale**: findings §2.1 suggested writing a `.bak` copy before any destructive rewrite; this feature extends that to any write that modifies an existing file (additive merges included, FR-004), and it makes the FR-008 "report and recover" path concrete (the error message points the user at the backup).

**Alternatives considered**: in-memory rollback on error only — rejected: does not survive a crash mid-write; on-disk backup is the recoverable artifact SC-006 requires.

## 5. Per-agent failure isolation in `rgt init`

**Decision**: Replace `configure()`'s `?` short-circuit (first failure aborts all remaining agents) with a per-agent result collection: each writer returns `AgentOutcome { Agent, Status: Configured|Skipped|Failed(reason) }`. `rgt init` prints every outcome and returns `Err(...)` (exit 1) if any agent failed, while still configuring the healthy ones (FR-008).

**Rationale**: The current code in `src/hooks/installer.rs:173-252` uses `?`, so one malformed file today prevents all subsequent agents from being configured. Constitution IX/XI: silent failures prohibited, errors must be user-recoverable, exit codes meaningful (1 = expected error). Partial success with a clear report is the recoverable behavior.

**Alternatives considered**: hard-abort on first failure — rejected: unnecessarily blocks healthy agents (US2 Scenario 5); "best-effort with exit 0" — rejected: silent non-success (SC-002).

## 6. Test strategy

**Decision**: Editor-level unit tests for byte-for-byte preservation and malformed-input errors (`tests/unit/hook_editors.rs`); per-writer contract tests with realistic fixtures (`tests/contract/hook_installer.rs`); end-to-end `rgt init` runs against isolated temp homes (`tests/integration/test_hooks_installer.rs`). Existing 018-era assertions that expected byte-identical installer output are **updated** to the new merge expectations (constitution IX).

**Rationale**: The preservation guarantee is only meaningful as a byte-level, fixture-driven assertion; the current contract tests already follow this pattern and must be extended rather than replaced.
