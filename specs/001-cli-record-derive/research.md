# Research: CLI Record & Derive Commands

**Feature**: 001-cli-record-derive
**Date**: 2026-08-08

## Decision: Reuse existing extraction engine for `rgt record`

**Decision**: `rgt record` calls `hooks::parser::extract_values_from_content()` directly, the same function used by passive hooks (`rgt hook post`).

**Rationale**: The extraction engine (regex-based number and ISO-8601 date extraction) is already tested via hook contract tests. Reusing it avoids duplication, ensures consistent behavior between active and passive paths, and satisfies SC-003.

**Alternatives considered**:
- New extraction logic: Rejected — violates DRY and SC-003; risks divergence between hook and CLI extraction behavior.
- Configurable extraction patterns: Rejected — over-engineered for v1; spec does not require custom regex.

## Decision: `rgt derive` wraps `verify::verify()` with a write step

**Decision**: `rgt derive` calls the existing `verify::verify()` function (same as `rgt verify` CLI) and, on success, calls `TrackedNode::generate_derived_id()` and `store::queries::insert_derivation_edge()` to persist the derivation.

**Rationale**: The `verify()` function already handles EXPRESSION and DATE_DIFF with proper exit code semantics. Wrapping it avoids duplicating the verification logic and ensures `rgt derive` and `rgt verify` produce identical results.

**Alternatives considered**:
- Inline verification in derive command: Rejected — duplicates logic, risks divergence.
- Separate verification crate: Rejected — unnecessary abstraction for two call sites.

## Decision: Value limit enforced in `rgt record` via early break in extraction loop

**Decision**: After extracting 10,000 values, `rgt record` prints a warning to stderr and stops iterating. Remaining lines in the file are not processed.

**Rationale**: A simple counter check in the extraction loop is O(min(N, 10000)) and adds no measurable overhead. The limit prevents accidental graph bloat from large files (log files, data dumps) without requiring complex streaming or batching infrastructure.

**Alternatives considered**:
- No limit: Rejected — agent could accidentally record a 500 MB log file, creating millions of nodes and degrading `rgt status` performance.
- Configurable limit via `--max-values` flag: Rejected — adds CLI complexity without clear user need; 10,000 is generous for provenance tracking use cases.
- Hard limit with error (exit 2): Rejected per user choice (Question 1 in clarifications); soft limit with warning is more user-friendly.

## Decision: `rgt derive` uses atomic write sequence (verify → insert node → insert edges)

**Decision**: The write sequence is: (1) call `verify()`, (2) if pass, call `insert_tracked_node()` for the derived node, (3) for each parent, call `insert_derivation_edge()`. All writes are in a single implicit transaction (SQLite auto-commit).

**Rationale**: The derived node ID is deterministic from parent IDs + operation + result, so the `ON CONFLICT(id) DO NOTHING` semantics on `insert_derivation_edge` prevent duplicate edges. No explicit transaction is needed since all DB operations use immediate execution.

**Alternatives considered**:
- Explicit BEGIN/COMMIT transaction: Rejected — adds complexity without benefit; individual operations are already idempotent.
- Verify at DB trigger level: Rejected — moves logic out of Rust binary, violating Principle VIII.

## Decision: `--stdin` variant reads content from stdin, uses path arg for node IDs

**Decision**: `rgt record --stdin <PATH>` reads file content from stdin (e.g., from a pipe or agent tool response) but uses `<PATH>` for node ID generation (`TrackedNode::generate_root_id(path, line, value)`). The file at `<PATH>` must exist on disk for metadata/hash computation.

**Rationale**: Agents like Kilo Code may have file content in memory from a tool call but need a stable path for node ID determinism. The path argument ensures re-recording the same logical file always generates the same node IDs.

**Alternatives considered**:
- Hash-based ID without path: Rejected — node IDs would be unstable across sessions.
- Content-only (no path required): Rejected — breaks source document linking and file-change detection.

## Decision: AGENTS.md template updated via `src/hooks/installer.rs`

**Decision**: The `codex_section` string constant in `installer.rs` is extended with `rgt record` and `rgt derive` instructions.

**Rationale**: Minimal change — the codex integration already writes to AGENTS.md. Adding two more instruction lines keeps the same format and detection logic (checks for `## RGT Integration` header).

**Alternatives considered**:
- Separate installer for CLI commands: Rejected — unnecessary complexity; AGENTS.md is the single rules-file for codex agents.
