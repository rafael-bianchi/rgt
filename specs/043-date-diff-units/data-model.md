# Data Model: Safe Duration Calculations and Unit Display

**Feature**: [spec.md](spec.md) | **Date**: 2026-09-25

The existing SQLite tables remain authoritative. This design adds command-level value objects and stronger identity/read rules; it does not add a stored unit column.

## Entities and value objects

### DurationUnit

| Attribute | Meaning |
|---|---|
| `name` | One of `seconds`, `minutes`, `hours`, `days`, `weeks` (exact lowercase CLI spelling). |
| `seconds_per_unit` | Exact positive scale: 1, 60, 3,600, 86,400, or 604,800. |

`--result-unit` qualifies a claim; `--unit` selects presentation on derive/query. Neither is persisted with a node. Months and years are not fixed Duration units.

### DurationClaim

| Attribute | Meaning |
|---|---|
| `lexeme` | Original `--result` decimal token, retained until exact validation. |
| `unit` | Explicit `--result-unit`, or `seconds` by default. |
| `exact_seconds` | Validated signed whole-second result after exact scale conversion. |

The claim may be absent for `derive` temporal operations. It is required for `verify`. Syntax, finite range, exact divisibility, and equality to the computed result are checked before writes. A syntactically valid but wrong claim fails verification; a malformed or unrepresentable claim is invalid input.

### ComputedDuration

| Attribute | Meaning |
|---|---|
| `operation` | `DATE_DIFF`, `DURATION_SUM`, or `DURATION_AVG`. |
| `parent_ids` | Ordered Date pair for `DATE_DIFF`; unique sorted Duration IDs for sum/average identity. |
| `seconds` | Exact signed, representable whole seconds. |
| `value_kind` | Always `DURATION`. |

`DATE_DIFF` requires two Date parents, the earlier first, and an elapsed gap that is nonnegative and exactly whole seconds. Sum/average require 2–26 distinct Duration parents. They accept signed values. Sum uses checked addition; average requires exact divisibility by parent count. No temporal calculation creates a bare Number.

### TrackedNode and DerivationEdge

Existing `tracked_nodes` holds a derived Duration with `duration_secs` and `value_kind='DURATION'`. Existing `derivation_edges` holds exactly one edge per unique parent-child pair with the operation label. A successful derivation creates or reuses one node linked to every parent. Repeated parent IDs are invalid because edge multiplicity is unavailable. Display/claim units are absent from both tables.

### DerivedIdentity

| Case | Identity rule |
|---|---|
| New Duration derivation | Versioned domain tag, value kind, operation, complete signed `seconds`, and length-delimited parent IDs form the hash preimage. |
| `DATE_DIFF` parents | Preserve the two Date IDs' order in the preimage. |
| Sum/average parents | Sort distinct parent IDs before hashing and edge insertion. |
| Historical `DATE_DIFF` | Reuse the stored legacy ID only after exact type/value/operation/parent verification. |
| Existing Number/root nodes | Keep existing ID rules. |

The outward node ID stays in the existing `node_drv_<hex>` family. A hash/ID match never authorizes an upsert over an incompatible stored row. Historical 12-character and 32-character IDs remain queryable and exportable. A collision whose old value was already overwritten cannot be reconstructed from the current store.

### SelectedUnitDisplay

| Attribute | Meaning |
|---|---|
| `unit` | Requested fixed display unit. |
| `decimal` | Exact finite decimal or deterministically rounded decimal. |
| `approximate` | True only when the unit conversion has no finite decimal representation at the chosen presentation precision. |
| `duration_seconds` | Lossless decimal string of the canonical signed seconds. |

For derive, the output includes the node ID, selected-unit text, and exact seconds. For query, the selected-unit fields are added only to the requested Duration step; its existing `value` and all lineage entries remain intact. A query without `--unit` has the same shape as before.

## Relationships and validation

```text
Date + Date --DATE_DIFF--> Duration
Duration + Duration [+ more distinct Duration parents] --DURATION_SUM/AVG--> Duration
Duration --presentation request--> SelectedUnitDisplay (not stored)
Duration --optional assertion--> DurationClaim (validated, not stored)
```

- Each accepted derived Duration has exactly the expected parent set and operation label. `DATE_DIFF` parent order remains material even though the edge table itself is a set; its identity preimage preserves the order.
- Temporal parent loading rejects malformed or missing typed columns rather than substituting current time or zero.
- Existing historical derivations are reused only when node value and full edge evidence agree. A conflicting legacy candidate is left untouched while the new canonical ID is considered. A conflicting canonical ID is an error; neither row is overwritten.
- Turtle reconstruction recognizes both historical and new Duration identity schemes so it can emit supported activities and uses without fabricating lineage.

## State transitions

1. **Input → validated request**: Check operation, applicable flags, unit spellings, parent count, unique IDs, exact claim syntax, and parent existence/type.
2. **Validated request → computed duration**: Calculate at full precision; reject reversed/subsecond Date gaps, overflow, or fractional-second averages.
3. **Computed duration → verified claim**: If a claim exists, convert it exactly and compare its canonical seconds to the computed value. Mismatch ends with no write.
4. **Verified duration → reused or new node**: Within one transaction, inspect a compatible historical candidate and the canonical ID. Reuse only complete matches; otherwise insert the new node and all edges. Any conflict rolls back.
5. **Stored node → selected display**: Convert for the response without changing the graph.
