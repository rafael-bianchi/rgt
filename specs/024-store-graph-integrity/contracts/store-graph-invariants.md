# Contract: Store & Graph Integrity Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the unit/contract tests for the store/graph layer.

## Cycle guard (FR-001/FR-002)

- `insert_derivation_edge(parent, child, ...)` returns an error (writes nothing) when `parent == child`.
- It returns an error (writes nothing) when adding `parent → child` would create a cycle of any length.
- After a rejected insert, the `derivation_edges` table is unchanged.
- `InvalidationCascade::build_from_db` returns `Err` (does not `let _ =`) if an edge cannot be added during rebuild.

## Concurrency (FR-003/FR-004)

- Every `DbStore` opened via `open` or `open_in_memory` has a non-zero `busy_timeout`.
- `execute_status` returns `Err` (non-zero) when `evaluate_and_invalidate_all` fails; it never prints "all fresh" after a discarded error.

## IDs & re-record (FR-005/FR-006)

- `generate_root_id` and `generate_derived_id` produce IDs of length ≥ 32 hex characters (128 bits).
- Inserting a node with an ID that already exists updates `number_val`, `date_val`, `duration_secs`, `source_doc_id`, and `line_number` to the incoming values.

## Hook transactionality (FR-007)

- The hook capture path inserts all extracted nodes in a single transaction (all-or-nothing); a failure rolls back and the hook still returns `Ok(())`.

## Acceptance linkage

- US1/FR-001/FR-002/SC-001/SC-002 → cycle guard.
- US2/FR-003/FR-004/SC-003/SC-004 → concurrency.
- US3/FR-005/FR-006/SC-005/SC-006 → IDs & re-record.
- US4/FR-007/SC-007 → hook transactionality.
