# Feature Specification: Store & Graph Integrity

**Feature Branch**: `feat/024-store-graph-integrity`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "The provenance store and graph must be concurrency-safe and corruption-resistant: a derivation cycle must be rejected at the database-insert layer (not only in the transient in-memory graph), concurrent access must not silently mis-report staleness, node IDs must be wide enough to avoid collisions (and re-recording must reflect the latest write, not silently keep stale data), and hook-driven inserts must be transactional. Findings §7.2."

Source context: `docs/findings-hooks-and-value-extraction.md` §7.2 (plan: `docs/spec-plan-from-findings.md`, spec #6, P1). Out of scope per the plan: schema migrations, garbage collection, and the `total_invalidated` double-count (§7.2 LOWs → `029-store-maintenance`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Derivation Cycle Is Rejected At Insert Time (Priority: P1)

An agent records a derivation that would create a cycle in the provenance graph (or a node deriving from itself). Today the cycle guard runs only while *rebuilding* the in-memory graph — the actual `INSERT` into `derivation_edges` has no cycle or self-loop check, and the reload path silently drops the closing edge (`let _ =`), with no guarantee about *which* edge is dropped. A real persisted dependency can vanish with zero diagnostic trail, so a genuinely stale node could be reported fresh. The cycle (and self-loop) must be rejected at the database-insert layer, before the edge is written.

**Why this priority**: §7.2 HIGH — the graph's correctness promise ("every derived value's lineage is trustworthy") is undermined when a cycle can persist and then be silently dropped on reload.

**Independent Test**: Unit/contract tests attempt to insert a self-loop edge and a multi-node cycle edge directly and assert the insert is rejected (a clear error, nothing written); the reload path must not silently discard a rejected edge.

**Acceptance Scenarios**:

1. **Given** an insert of `parent == child` (self-loop), **When** the edge is written, **Then** it is rejected with a clear error and nothing is persisted.
2. **Given** an insert that would close a 2-node cycle (A→B exists; B→A attempted), **When** the edge is written, **Then** it is rejected and the graph stays acyclic.
3. **Given** a `build_from_db` reload over a database that (somehow) already contains a cycle, **When** it runs, **Then** the dropped-edge situation is logged, never silently swallowed.

---

### User Story 2 - Concurrent Access Never Mis-Reports Staleness (Priority: P1)

Two `rgt` processes touch the same `.rgt/store.db` at once (a PreToolUse/PostToolUse pair, or a hook racing an interactive `rgt status`). Today no connection sets a `busy_timeout`, so SQLite's default `busy_timeout = 0` makes the loser fail immediately with `SQLITE_BUSY` — and `rgt status` discards the staleness-check result (`let _ =`), so it can print "all fresh" when the check never actually ran. Connections must wait on locks, and correctness-relevant failures must surface.

**Why this priority**: §7.2 HIGH — silent false "all fresh" reports directly undermine trust in the tool's core output.

**Independent Test**: Tests assert every `DbStore` connection is configured with a non-zero `busy_timeout`, and that `execute_status` propagates (rather than discards) an `evaluate_and_invalidate_all` failure.

**Acceptance Scenarios**:

1. **Given** a connection is opened, **When** the store is constructed, **Then** a non-zero `busy_timeout` is set on it.
2. **Given** `evaluate_and_invalidate_all` fails during `rgt status`, **When** the command runs, **Then** it reports the error (non-zero) instead of printing node counts as if the check succeeded.

---

### User Story 3 - Node IDs Are Wide Enough and Re-Records Update Values (Priority: P1)

A long-lived project accumulates many nodes. Today IDs are truncated to 48 bits, so two genuinely distinct inputs can collide (birthday-bound ~50% odds around ~16.7M nodes) — and the `ON CONFLICT` clause only refreshes staleness fields, never the value columns, so a colliding re-record silently keeps the *first* writer's value and source document forever. IDs must be widened, and a re-record of an existing ID must reflect the latest write in the value-bearing columns.

**Why this priority**: §7.2 MEDIUM-HIGH — silent, permanent corruption of node data on a collision, and stale values surviving re-record.

**Independent Test**: Unit tests assert the generated root/derived ID is at least 128 bits (32 hex chars); a re-record of the same ID updates the value-bearing columns (`number_val`/`date_val`/`duration_secs`/`source_doc_id`/`line_number`), not just staleness.

**Acceptance Scenarios**:

1. **Given** node-ID generation, **When** a root or derived ID is produced, **Then** it is at least 128 bits (32 hex characters).
2. **Given** an existing node ID is re-inserted with a different value/source, **When** the insert runs, **Then** the value-bearing columns are updated to the latest write (no silently-stale value).

---

### User Story 4 - Hook-Driven Inserts Are Transactional (Priority: P2)

A hook captures a file with many values; the process is killed or a lock error occurs partway through. Today the hook inserts rows one at a time with no transaction, so a failure leaves a partially-recorded, inconsistent state with no signal — and multiplies lock acquire/release cycles. The hook's extraction loop must be wrapped in a single transaction (like `rgt record` already does).

**Why this priority**: §7.2 MEDIUM — partial/inconsistent captures and extra lock contention compound the concurrency finding above.

**Independent Test**: A unit test verifies the hook insert path commits a batch of nodes in one transaction (all-or-nothing), matching `rgt record`'s pattern.

**Acceptance Scenarios**:

1. **Given** a hook capture with N values, **When** the hook records them, **Then** they are committed in a single transaction (no per-row autocommit).
2. **Given** a failure mid-batch, **When** it occurs, **Then** the transaction rolls back (no partial rows), consistent with `rgt record`.

---

### Edge Cases

- A cycle that spans many nodes (not just self-loop or 2-node) — is it caught by the same recursive check?
- An `ON CONFLICT(id)` where the incoming value has a *different* `value_kind` than the stored row (e.g. Number vs Date at the same ID)?
- Two connections writing the same node concurrently — does the widened ID + `ON CONFLICT ... DO UPDATE` still lose updates, or does the value update make it correct?
- A `build_from_db` reload where the DB is empty vs. one already containing a cycle.
- `busy_timeout` on a read-only filesystem or a filesystem without WAL support (does WAL fall back gracefully)?
- A very long-running lock wait — is `busy_timeout` bounded so a stuck process doesn't hang forever?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Inserting a derivation edge that would create a cycle (including a self-loop `parent == child`) MUST be rejected at the database-insert layer — before the edge is written — with a clear error. The insert path MUST NOT rely on the transient in-memory graph's cycle check.
- **FR-002**: The graph-reload path (`build_from_db`) MUST NOT silently discard a rejected edge; a cycle encountered during reload MUST be surfaced (logged) rather than dropped with `let _ =`.
- **FR-003**: Every `DbStore` connection MUST be configured with a non-zero `busy_timeout` so concurrent access waits instead of failing immediately with `SQLITE_BUSY`.
- **FR-004**: `rgt status` MUST propagate failures from `evaluate_and_invalidate_all` (report the error and exit non-zero) rather than discarding them and printing "all fresh".
- **FR-005**: Generated node IDs (root and derived) MUST be at least 128 bits (32 hex characters) — widened from the current 48 bits — to make collisions cryptographically negligible.
- **FR-006**: Re-recording an existing node ID MUST update the value-bearing columns (`number_val`, `date_val`, `duration_secs`, `source_doc_id`, `line_number`) to the latest write, not just the staleness columns.
- **FR-007**: The hook capture path MUST record its extracted nodes in a single transaction (all-or-nothing), matching `rgt record`'s existing transaction pattern.

### Key Entities *(include if feature involves data)*

- **TrackedNode**: The stored value node; its `id` is now ≥128 bits, and re-record updates value-bearing columns.
- **DerivationEdge**: A parent→child provenance edge; its insert is cycle-guarded at the DB layer.
- **DbStore/Connection**: The SQLite connection; every instance carries a non-zero `busy_timeout`.
- **InvalidationCascade**: The rebuilt in-memory graph; its edge-loading path surfaces (never silently drops) cycle rejections.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of cycle-inducing edge inserts (self-loop, 2-node, multi-node) are rejected at the insert layer with nothing persisted (asserted by unit/contract tests).
- **SC-002**: 0 silent edge drops on reload — a cycle encountered in `build_from_db` is logged/surfaced (asserted).
- **SC-003**: 100% of `DbStore` connections carry a non-zero `busy_timeout` (asserted by a unit test inspecting the connection).
- **SC-004**: `rgt status` with a failing invalidation check returns a non-zero error instead of "all fresh" (asserted).
- **SC-005**: All generated node IDs are ≥32 hex characters (asserted).
- **SC-006**: A re-record of an existing ID updates value-bearing columns (asserted by a contract test).
- **SC-007**: The hook insert path is a single transaction (asserted by a unit test).

## Assumptions

- Scope is the §7.2 HIGH/MEDIUM-HIGH findings: cycle guard at the DB-insert layer, `busy_timeout`, ID widening + value-column `ON CONFLICT` update, and hook-insert transaction. Schema migrations (`PRAGMA user_version`), garbage collection, and the `total_invalidated` double-count are out of scope (spec `029-store-maintenance`).
- ID widening changes the ID *format* for **new** writes only. Existing stored 48-bit IDs remain valid (opaque strings); re-recording a file produces new 128-bit IDs that won't match old 48-bit nodes, so historical data isn't retroactively merged — reconciling/re-keying existing rows (if desired) is deferred to the migration spec (029).
- The cycle check at the insert layer uses a recursive-CTE (or equivalent) pre-insert guard plus a `CHECK (parent_node_id <> child_node_id)` self-loop constraint; both are added to the schema idempotently (the schema init already runs `CREATE ... IF NOT EXISTS`).
- `busy_timeout` is set to a bounded, human-reasonable value (e.g. 5 seconds) to avoid indefinite hangs (edge case).
- `value_kind` mismatch on `ON CONFLICT` (Number vs Date at the same ID) is out of scope; with 128-bit IDs it is cryptographically negligible, and the value columns are typed per-row by `value_kind`.
