# Graph Contract: Typed Duration Lineage and Identity

**Feature**: [../spec.md](../spec.md) | **Date**: 2026-09-25

## Canonical value and derivations

Every accepted `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG` result is stored as `value_kind='DURATION'` with exact signed `duration_secs`. The operation is attached to each parent-child derivation edge. A successful node has one edge from each distinct parent. A failed calculation, assertion, or identity check leaves both node and edge tables unchanged.

| Operation | Parent kind/count | Parent order | Result |
|---|---|---|---|
| `DATE_DIFF` | Exactly two Dates | Significant: second Date minus first Date | Nonnegative whole-second Duration |
| `DURATION_SUM` | 2–26 distinct Durations | Insignificant | Checked signed whole-second sum |
| `DURATION_AVG` | 2–26 distinct Durations | Insignificant | Checked signed whole-second mean; non-integral seconds rejected |

The same Duration may not be listed twice: `derivation_edges` has one parent-child edge per pair and cannot record multiplicity. Sum and average sort the validated distinct IDs before identity computation and edge insertion. `DATE_DIFF` preserves caller order in its identity computation and records edges in that order so later export can reconstruct it.

## New Duration identity

The identity preimage is domain-separated and versioned, rather than using a human display string. A concrete encoding is:

```text
"rgt:derived-duration:v2\0"
|| length("DURATION") || "DURATION"
|| length(operation) || operation
|| signed-seconds as fixed-width big-endian i64
|| parent count as fixed-width integer
|| repeated (length(parent-id) || parent-id bytes)
```

Lengths make the encoding unambiguous. Hash with the project's existing BLAKE3 convention and retain the outward `node_drv_<32 hex>` shape. `DATE_DIFF` uses its ordered pair; sum/average use sorted distinct IDs. Result and operation are part of identity; requested display unit and claim unit are not. Keep root IDs and Number-derived IDs on their existing algorithms.

Before inserting a canonical ID, load any row already at that ID. Reuse it only when its Duration kind, exact seconds, operation, and full parent evidence match. If its value or lineage conflicts, fail without an upsert. Do not use the existing unconditional `insert_tracked_node` upsert for this collision-sensitive path.

## Historical DATE_DIFF compatibility

Existing stores may contain 12- or 32-hex-character Duration-derived IDs generated from `ValueData::to_string_repr()`. They remain valid. On a new `DATE_DIFF` request, calculate the legacy candidate from the requested ordered parents and canonical Duration. Reuse the historical node only if the stored row's type, exact seconds, `DATE_DIFF` edge labels, and complete parent set agree. If the legacy candidate exists with another canonical value or unsupported lineage, leave it untouched and use the new canonical ID, subject to the collision check above. A historical value already overwritten by an old collision cannot be inferred or rebuilt automatically.

This lookup uses candidate IDs and indexed parent/child edges; it does not scan all graph nodes or rewrite historical IDs. Repeated equivalent calls reuse one node. Any insert of a new node and all edges occurs in one transaction, including cycle checks.

## Strict reads and malformed stores

Temporal arithmetic and historical reuse must reject missing or malformed typed columns. A Date row cannot become current UTC time, and a Duration row cannot become zero by fallback. A corrupt candidate or parent yields an actionable error with no graph write. This is necessary because both operations would otherwise produce false, potentially changing results from the same node ID.

## Query and export

SQLite retains each node's exact canonical `duration_secs` and parent lineage. Turtle exposes that canonical value as a typed `xsd:duration` and preserves supported lineage. `query --unit` adds exact seconds and presentation fields only in the response; it never creates a node or modifies `duration_secs`. A query without `--unit` and the existing text graph view keep their current display contracts and need not add a seconds field. The selected unit is not graph provenance and is not emitted as a stored property.

Turtle export must recognize both historical and new canonical Duration IDs when it classifies a derivation. For supported new sums/averages and date differences, the child remains a `prov:Entity` with typed `xsd:duration`, an activity `prov:used` every parent, and `prov:wasGeneratedBy` on the child. The operation label remains available. If the store does not establish one supported activity, export keeps only evidence-backed direct lineage under the existing uncertainty rule. Existing project-scoped IRIs and size/deadline limits remain unchanged.

## Compatibility assertions

- Equivalent `1 hour` and `60 minutes` claims for the same ordered Date parents resolve to one node.
- Reversing the distinct parents of a sum/average resolves to one node and the same edge set; reversing a `DATE_DIFF` Date pair remains a different and invalid calculation under its nonnegative rule.
- Two distinct canonical second values never share a new Duration ID merely because their human display strings match.
- A historical valid `DATE_DIFF` node keeps its ID and lineage after an equivalent new request.
- A mismatching legacy candidate or canonical ID is never overwritten.
- New typed Duration activities survive Turtle export classification; historical 12- and 32-character IDs continue to export as before.
