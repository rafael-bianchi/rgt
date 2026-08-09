# Research: Documentation Reconciliation

**Feature**: `012-documentation-reconciliation` | **Date**: 2026-08-08

## Decision 1: Source of Truth

**Decision**: The code and tests are the single source of truth. Documentation must match them, never the reverse.

**Rationale**: Constitution X (Documentation Matches Behavior). RGT's README is read by both human users and AI agents (via hooks/MCP). Any mismatch causes agent errors and user confusion.

## Decision 2: Doc Comment Scope

**Decision**: Every `pub fn` in `src/` MUST have a `///` doc comment. This includes internal modules (e.g., `updater/checksum.rs`) since they are `pub` within the crate and appear in `cargo doc`.

**Rationale**: `cargo doc` output is the API contract. Missing comments make public items undiscoverable and trigger `missing_docs` lint potential.

**Alternatives considered**: Only document externally-facing modules — rejected because `cargo doc` includes all `pub` items.

## Decision 3: Doc Comment Content

**Decision**: Doc comments MUST include:
- One-line summary of purpose
- `# Arguments` section (when parameters are non-obvious)
- `# Returns` / error conditions section

**Rationale**: Matches the existing style used in `src/verify/` and `src/mcp/handlers.rs`. Consistent format is easier to maintain.

## Decision 4: README/AGENTS Verification

**Decision**: Verify (not rewrite) README and AGENTS.md against code. Fix only confirmed mismatches.

**Rationale**: The README was recently rewritten during the docs update task and already covers all 8 CLI subcommands. AGENTS.md install paths were corrected during the distribution fix. Full rewrite would introduce risk without benefit.

## Decision 5: rustdoc Link Escaping

**Decision**: Reference array indices in doc comments as `` `parent[0]` `` (backticked) to prevent `broken_intra_doc_links` warnings.

**Rationale**: rustdoc parses `[0]` as an intra-doc link reference when not escaped. Backticks prevent this (verified: 2 warnings fixed by backticking).

## Decision 6: Tooling Directory Gitignore

**Decision**: Add `.kilocode/` to `.gitignore` alongside `.kilo/`.

**Rationale**: Both are local tooling workspace directories. Committing them would leak machine-specific config and violate the "no local tooling in repo" convention already established for `.kilo/`.
