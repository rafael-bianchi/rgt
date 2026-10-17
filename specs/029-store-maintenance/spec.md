# Feature Specification: Store Maintenance

**Feature Branch**: `feat/029-store-maintenance`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "Add store hygiene so the provenance graph degrades gracefully instead of silently: track and migrate the SQLite schema version (no 'no such column' on version skew), provide an explicit garbage-collection command for obsolete nodes, and stop double-counting the staleness blast radius. Findings §7.2 LOWs."

Source context: `docs/findings-hooks-and-value-extraction.md` §7.2 (no schema migration path, no garbage collection, `total_invalidated` double-count). Plan: `docs/spec-plan-from-findings.md`, spec #11, P3.

## Clarifications

### Session 2026-08-29

- Q: After `rgt gc` deletes obsolete node rows, should it also run SQLite `VACUUM` to physically reclaim freed disk space? → A: Delete rows only; `VACUUM` is behind an explicit `--vacuum` flag (default run stays fast and non-destructive).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Store Survives a Version-Skewed Binary (Priority: P1)

A developer upgrades (or downgrades) the `rgt` binary against an existing `.rgt/store.db` whose schema was created by a different version. Today the store has no recorded schema version, so a mismatched binary either fails deep inside a query with "no such column" or silently misbehaves. The store should record its schema version and fail fast with a clear, actionable message when the binary and database versions are incompatible.

**Why this priority**: §7.2 LOW-MEDIUM — this is the only item in the set that can corrupt data or brick every command on version skew; the others are cosmetic/hygiene.

**Independent Test**: Opening a store whose recorded schema version is newer than the binary supports returns a clear error naming the version mismatch (not "no such column").

**Acceptance Scenarios**:

1. **Given** a freshly initialized store, **When** it is inspected, **Then** it records the current supported schema version.
2. **Given** a store whose schema version is newer than the running binary supports, **When** any command opens it, **Then** it fails with a clear "store schema version N is newer than this build supports" error instead of a SQL error.
3. **Given** a store at an older (but supported) version, **When** a newer binary opens it, **Then** the schema is migrated forward idempotently (no data loss).

---

### User Story 2 - Obsolete Nodes Can Be Collected (Priority: P2)

A developer's project has accumulated stale nodes whose source files were deleted or whose derivations are no longer referenced. Today every recorded value is a permanent row with no way to remove it, so the store grows unbounded. The developer needs an explicit, opt-in command to collect obsolete nodes and reclaim them.

**Why this priority**: §7.2 LOW — hygiene; a leak-over-time that also raises the odds of ID collisions as a project ages.

**Independent Test**: `rgt gc` removes obsolete nodes and reports how many it removed, leaving non-stale and dependent nodes untouched.

**Acceptance Scenarios**:

1. **Given** a store with obsolete nodes (stale, no dependents), **When** the developer runs `rgt gc`, **Then** those nodes are removed and the count is reported.
2. **Given** a store with a stale node that still has derived dependents, **When** `rgt gc` runs, **Then** that node (and its dependents) are preserved.
3. **Given** a store with no obsolete nodes, **When** `rgt gc` runs, **Then** it reports zero and leaves the store unchanged.

---

### User Story 3 - Staleness Blast Radius Is Counted Once (Priority: P2)

A developer invalidates a file and reads the reported "blast radius" (number of nodes invalidated). Today, when a single derived node is reachable from two stale ancestors of the same file, it is counted twice, overstating the distinct nodes affected.

**Why this priority**: §7.2 LOW — cosmetic; the staleness *state* is correct, only the printed count is inflated.

**Independent Test**: Invalidating a file with two roots that share one downstream node reports that downstream node exactly once.

**Acceptance Scenarios**:

1. **Given** two root nodes in one file that both depend on the same derived node, **When** the file is invalidated, **Then** the reported invalidated count includes that derived node exactly once.
2. **Given** a file with N distinct invalidated nodes, **When** the file is invalidated, **Then** the reported count equals the number of distinct nodes invalidated.

---

### Edge Cases

- A store with `user_version` set but unknown/unsupported (e.g. a future major bump)?
- `rgt gc` interrupted mid-run (partial delete) — is the operation transactional?
- A node that is stale AND has dependents that are themselves stale — collected transitively or preserved?
- `VACUUM` after collection (space reclamation) vs. leaving free pages?
- The double-count fix interacting with `evaluate_and_invalidate_all` (which sums `invalidate_file` results across files)?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The store MUST record its schema version via SQLite's `PRAGMA user_version`, and every command MUST fail fast with a clear, actionable error when the recorded version is newer than the running binary supports (instead of surfacing a raw SQL error).
- **FR-002**: Schema changes MUST be expressed as an ordered, idempotent migration sequence applied on open, gated by `PRAGMA user_version` (no data loss on a supported forward migration).
- **FR-003**: The CLI MUST provide an opt-in `rgt gc` command that removes obsolete nodes — stale nodes with no derived dependents (hereafter "obsolete nodes") — and reports the number removed. By default it MUST only delete rows (no `VACUUM`); physical space reclamation MUST be opt-in via an explicit `--vacuum` flag.
- **FR-004**: `rgt gc` MUST be safe: it MUST NOT remove non-stale nodes or any node with derived dependents, and it MUST run transactionally (all-or-nothing).
- **FR-005**: The staleness invalidated-count MUST count each distinct node exactly once (no double-counting of a node reachable from multiple stale ancestors).

### Key Entities *(include if feature involves data)*

- **SchemaVersion**: The store's `user_version` integer — the single source of truth for which schema the binary expects vs. the store carries.
- **Migration**: An ordered, idempotent step that moves the schema from version `N` to `N+1`.
- **ObsoleteNode**: A tracked node eligible for GC — stale, with no derived dependents.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Opening a store with an unsupported (newer) schema version returns a clear version-mismatch error (contract-tested).
- **SC-002**: A fresh store records the current supported schema version (contract-tested); the migration framework is exercised by a synthetic no-op migration so a future schema change has a tested path.
- **SC-003**: `rgt gc` removes only obsolete nodes (stale, no dependents) and reports the exact count; a node with dependents is never removed (contract-tested).
- **SC-004**: Invalidating a file whose two roots share one downstream node reports that node exactly once in the invalidated count (unit-tested).

## Assumptions

- The current on-disk schema is designated "version 1"; `PRAGMA user_version` is introduced as version 1 for existing stores (backfill) and new stores are created at the current version.
- Migration is hand-rolled using `PRAGMA user_version` (no new dependency — the `rusqlite_migration` crate is a packaged wrapper over the same primitive, rejected per Constitution XI's preference for std/existing crates first).
- GC targets only obsolete nodes (stale, no dependents); preserving history is prioritized over reclaiming every possible byte. Physical space reclamation is opt-in via `rgt gc --vacuum` (default run only deletes rows).
- The double-count fix only changes the reported count, not the (already-correct) staleness state.

## Out of scope

ID-width widening (48-bit → full-width, covered in 024), the same-line value collision (covered in 022), and any change to the derivation-verification semantics.
