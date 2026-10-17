# Research: Fix Query JSON Flag Behavior

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `src/cli/query.rs` (the buggy both-branches-JSON), `src/query/mod.rs` (`query_provenance`'s returned JSON shape), `src/cli/status.rs` (existing plain-text style), `src/cli/graph.rs` (existing DAG text style), and `src/types/node.rs` (`ValueKind`/`ValueData` display).

## 1. The bug (FR-001, FR-002)

**Finding**: `execute_query` (`src/cli/query.rs:9-18`) branches on `json_output` but both branches call `serde_json::to_string_pretty`. The only difference: the non-`--json` branch wraps the value in `{"lineage": result}`. So `--json` is functionally a no-op, and the default output contradicts the README (`rgt query <id> [--json]` implies text default) and the flag help ("Output lineage as JSON"). The growth research confirmed this by running it (`docs/weakpoints-and-growth-opportunities.md` §3.4).

## 2. The JSON contract (FR-002, SC-002, SC-003)

**Decision**: The `--json` output is exactly the `serde_json::Value` returned by `query_provenance` — a self-contained object with `node_id`, `is_stale`, `total_ancestors`, and `lineage_steps` (each step has `node_id`, `value_kind`, `value`, `is_stale`, `stale_reason`, `parent_ids`, `line_number`, `node_type`). The `{"lineage": ...}` wrapper is removed. This is a deliberate simplification: the wrapper was the only cosmetic distinction between the buggy branches, and nothing in the repo depends on it (verified: no test and no code greps for a `lineage` wrapper key).

**Rationale**: A stable machine contract that equals the domain function's native output; no invented envelope.

**Alternatives considered**: Keep the wrapper for the default branch and only fix `--json` — rejected: leaves the default branch printing JSON, which is the bug. Add a new envelope — rejected: needless invention.

## 3. The plain-text renderer (FR-001, FR-005)

**Decision**: A pure function `render_lineage_text(result: &serde_json::Value) -> String` in `src/cli/query.rs`. Format, modeled on `rgt status`/`rgt graph`'s text style:

```text
=== RGT Provenance Lineage ===
node: <node_id> | kind: <value_kind> | value: <value> | type: ROOT|DERIVED | stale: YES/NO
total ancestors: N
<blank>
STEP <i> — <node_id>  (DERIVED)
  value: <value>
  kind:  <value_kind>
  stale: YES (reason) / NO
  line:  <line_number or "n/a">
  parents: <parent_ids joined by ", " or "none">
```

Steps are printed most-recent-node-first (the order `query_provenance` already returns: the queried node, then its ancestors in BFS order). Staleness state and reason are always shown; a missing `stale_reason` prints without a reason suffix; a missing `line_number` prints `n/a`; a root node (empty `parent_ids`) prints `parents: none`. The `node_type` uses the human `ROOT`/`DERIVED` display (already implemented via `NodeType::fmt` in `src/types/node.rs:11-18`) rather than `Debug`.

**Rationale**: Consistent with the CLI's existing plain-text voice; a pure function is trivially unit-testable.

**Alternatives considered**: Reuse `query_provenance`'s internal node objects directly — rejected: they're not returned (only JSON is), so the renderer consumes the JSON value. Print via `println!` directly — rejected: a pure `-> String` renderer is more testable and keeps `execute_query` a thin wrapper.

## 4. Error behavior unchanged (FR-004)

**Finding**: `execute_query` calls `query_provenance(node_id)?` first; unknown nodes already return `Err("Node not found: ...")` from `src/query/mod.rs:16`, propagated by `main.rs`'s generic handler with exit code 1.

**Decision**: Keep the early `?` — errors short-circuit before any rendering, in both modes. No panic paths introduced (all rendering reads via `serde_json::Value` accessors with safe fallbacks).

## 5. Test strategy (SC-001..SC-004)

**Decision**: New contract test target `test_query_contract` in `tests/contract/test_query_contract.rs` (registered in `Cargo.toml`). It builds an in-memory store, records a root value, derives a child, then calls `execute_query` capturing stdout (or asserts via a refactored helper). Asserts: (a) no-flag output contains no leading `{` and no `"lineage"`; (b) `--json` output parses as JSON with the expected fields and no wrapper key; (c) both modes carry the same steps/order; (d) unknown node errors in both modes.

**Rationale**: Constitution IX — behavior changes must have coverage. The test target name follows the `test_*_contract` convention established for `test_cli_record_derive_contract`, `test_store_maintenance_contract`, etc.
