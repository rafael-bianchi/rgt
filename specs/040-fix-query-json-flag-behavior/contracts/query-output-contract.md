# Contract: Query Output

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/contract/test_query_contract.rs`.

## I. Default output is plain text (FR-001, FR-005)

- `rgt query <id>` (no flag) output MUST NOT start with `{` and MUST NOT contain a `"lineage"` JSON key.
- It MUST include the node's value, value kind, staleness state, and ancestor/lineage steps.
- A root node MUST render a valid single-node trace (`parents: none`, `total ancestors: 0`).

## II. `--json` output is clean, parseable JSON (FR-002)

- `rgt query <id> --json` output MUST parse via `serde_json`.
- It MUST NOT have a top-level `lineage` wrapper key.
- It MUST contain `node_id`, `is_stale`, `total_ancestors`, and `lineage_steps` matching `query_provenance`.

## III. Same data, both presentations (SC-003)

- Text and JSON modes MUST present the same steps in the same order.

## IV. Errors unchanged (FR-004)

- An unknown node ID returns `Err("Node not found: ...")` (exit non-zero) in BOTH modes; no panic.

## V. Non-regression (Constitution X, XI)

- README's `rgt query <node_id> [--json]` line and the flag's `--help` text remain accurate.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
