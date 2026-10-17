# Contract: Store-Maintenance Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Executable invariants verified by `tests/unit/store_integrity.rs`, `tests/unit/test_invalidation.rs`, and `tests/contract/test_store_maintenance_contract.rs`.

## I. Schema-version contract (FR-001, FR-002)

| Stored `user_version` | Result |
|-----------------------|--------|
| `0` (unversioned) | stamped to `SCHEMA_VERSION` (1), no error |
| `1` (== current) | opens normally |
| `> 1` (newer) | error (exit 1) naming the version and the supported max |

- A forward migration (none today) must preserve all existing rows.

## II. `rgt gc` contract (FR-003, FR-004)

- Deletes only nodes that are stale AND have no derived dependents (no `derivation_edges` where the node is the parent).
- Reports the exact number of nodes removed; `0` when nothing is obsolete.
- Never removes a non-stale node or any node with dependents.
- Runs transactionally (all-or-nothing).
- `--vacuum` runs `VACUUM` after commit; the default run does not.

## III. Invalidated-count contract (FR-005)

- Invalidating a file with two roots that share one downstream node reports that downstream node exactly once.
- The count equals the number of distinct nodes invalidated.

## IV. Non-regression (Constitution VIII, IX, XI)

- `rgt gc` is a new opt-in subcommand; existing commands are unchanged.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
