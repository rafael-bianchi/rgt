# Feature Specification: Fix Query JSON Flag Behavior

**Feature Branch**: `feat/040-fix-query-json-flag-behavior`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "`rgt query <node_id> --json` always prints JSON regardless of the flag — `src/cli/query.rs` calls `serde_json::to_string_pretty` in both branches, contradicting both the README and the function's own doc comment. Real CLI/doc bug, verified by running it."

Source context: `docs/weakpoints-and-growth-opportunities.md` §3 item 4 ("Real CLI/doc bug, not just a doc issue"). Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #7, P1 (smallest, fully self-contained).

## Clarifications

### Session 2026-08-31

- Q: Fix the docs to say "always JSON," or implement a real plain-text lineage renderer for the non-`--json` path? → A: Implement a real plain-text renderer. The README's Commands table describes `rgt query <id>` as "Traces a value's lineage back to its source files" — a human-facing use — and the `--json` flag is documented as opt-in ("Output lineage as JSON"). The documented contract is the correct one; the code is wrong. The CLI's default output for a lineage trace should be human-readable text, with `--json` reserved for machine consumption.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Human-Readable Lineage by Default (Priority: P1)

A developer runs `rgt query node_drv_Z` (no flag) after `rgt derive` to understand where a derived value came from. Today they get a raw JSON blob with `{"lineage": ...}` — the same format as with `--json`, just wrapped. They expect, per the README and the flag's own help text, a human-readable trace of the value's lineage: which source file/line it came from, its kind and value, and its ancestry chain. With `--json`, they expect clean, parseable JSON without a redundant wrapper and without the human formatting.

**Why this priority**: P1 per the growth plan — a genuine, user-visible correctness/documentation mismatch, verified by running the command; small and fully self-contained, listed first to clear it off the board.

**Independent Test**: `cargo test --test test_query_contract` (new) asserts the default output is plain text containing the node's value/kind/ancestry, and `--json` output is valid JSON that parses and matches the node's data.

**Acceptance Scenarios**:

1. **Given** a store with a recorded node and a derived value, **When** the developer runs `rgt query <id>` without `--json`, **Then** the output is human-readable plain text (not a JSON object) that includes the node's value, its value kind, its staleness state, and its ancestor/lineage steps.
2. **Given** the same store, **When** the developer runs `rgt query <id> --json`, **Then** the output is valid JSON parseable by `jq`, containing the lineage structure, and is NOT prefixed/wrapped by the `{"lineage": ...}` envelope that the buggy non-`--json` branch added.
3. **Given** an unknown node ID, **When** either form runs, **Then** the command errors with the existing "Node not found" message and a non-zero exit, identical to current behavior.
4. **Given** a freshly built binary, **When** `rgt query --help` is read, **Then** the `--json` flag help ("Output lineage as JSON") now accurately describes the actual behavior.

---

### Edge Cases

- A node with no ancestors (a root): the text renderer must still print a coherent single-node trace (e.g. "No ancestors" / zero lineage steps), not an empty or malformed block.
- A deep lineage (many steps): the text renderer must not drop or truncate steps; order should be stable and readable (e.g. most-recent node first, then ancestors, or the reverse — consistent and documented).
- Stale nodes in the chain: staleness state and reason must be visible in text form, matching the information JSON already carries.
- Empty `stale_reason` (None): the text renderer must handle a missing reason without printing a raw `null`/panic.
- `node_type`/`value_kind` formatting: text output should use the human-friendly strings already used elsewhere in the CLI (e.g. `rgt status`'s `Kind:` output), not the `Debug`-style `{:?}` form used inside the JSON.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `rgt query <node_id>` (no flag) MUST print a human-readable plain-text lineage trace instead of JSON.
- **FR-002**: `rgt query <node_id> --json` MUST print valid JSON representing the node's lineage, WITHOUT the buggy `{"lineage": ...}` wrapper that made the two branches differ only cosmetically; the JSON shape MUST be a stable, documented machine contract.
- **FR-003**: The two output modes MUST be mutually exclusive and selected solely by the `--json` flag; the default is always plain text.
- **FR-004**: Error behavior MUST be unchanged: unknown node ID returns the existing "Node not found" error and non-zero exit in both modes; no panic on any input.
- **FR-005**: The text renderer MUST present, for each lineage step: the node's value, value kind, staleness state (and reason when present), and its parent relationships, in a stable, human-readable order; root nodes MUST produce a valid single-node trace.

### Key Entities *(include if feature involves data)*

- **LineageStep**: one node in the ancestry chain with `node_id`, `value_kind`, `value`, `is_stale`, `stale_reason`, `parent_ids`, `line_number`, `node_type` (already produced by `query_provenance`).
- **QueryOutput**: the rendered result — either human text or JSON, depending on `--json`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The default (no-flag) output contains no JSON object syntax (no leading `{`, no `"lineage"`) and reads as a coherent human trace (contract-tested).
- **SC-002**: `--json` output is valid JSON per a JSON parser, with the lineage fields intact and no outer wrapper key (contract-tested).
- **SC-003**: Both modes produce the same lineage *data* (same steps, same order), differing only in presentation (contract-tested).
- **SC-004**: The README's `rgt query` line and the `--help` text remain accurate after the change (README already matches the new behavior; no doc drift).

## Assumptions

- The human-readable format is a new design decision; it should be consistent with the project's existing plain-text output style (see `rgt status`'s text branch).
- The JSON output becomes the "canonical" machine shape; removing the `{"lineage": ...}` wrapper is a deliberate simplification (the wrapper was the only thing distinguishing the two buggy branches, and `query_provenance` already returns a self-contained object).
- No other command or integration depends on the `{"lineage": ...}` wrapper shape (checked: `execute_query` has no test coverage today and nothing greps for `"lineage"` output).

## Out of scope

- Changing `query_provenance`'s internal JSON construction (`src/query/mod.rs`) — the lineage data stays identical; only the CLI's presentation layer (`src/cli/query.rs`) changes.
- The `--stale-only` dead-branch issue in `src/cli/status.rs` (separate finding, not in this spec's scope).
- Any other command's `--json` behavior (`rgt status --json` already behaves correctly).
