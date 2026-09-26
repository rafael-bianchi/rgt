# Research: Safe Duration Calculations and Unit Display

**Date**: 2026-09-25 | **Feature**: [spec.md](spec.md)

This research is based on the current RGT source and the clarified feature contract. It resolves the design questions needed before the data model and CLI/graph contracts. No external service or new runtime library is required.

## 1. Separate calculation, claim, and presentation

**Decision**: Compute temporal results from parent nodes first. `derive` may omit `--result` for `DATE_DIFF`, `DURATION_SUM`, and `DURATION_AVG`; if supplied, it is an assertion checked before writing. `verify` still requires a claim. `--result-unit` qualifies a claim; `--unit` selects derive/query presentation. Keep `EXPRESSION`'s required numeric result and existing semantics.

**Rationale**: The current CLI requires `--result: f64` for both commands (`src/main.rs`), verifies it, then turns a `DATE_DIFF` result into `Duration::seconds(result as i64)` (`src/cli/derive.rs`). Calculation by RGT avoids an agent's conversion error and keeps the graph value independent of display.

**Alternatives considered**: Reusing one `--unit` for input and output is ambiguous. Requiring an LLM-calculated result preserves the observed 45-day/45-second failure mode. Making `verify` compute without a claim removes its existing purpose.

## 2. Exact duration arithmetic and lexical claims

**Decision**: Parse temporal `--result` from its raw decimal token, including optional sign, decimal point, and scientific exponent. Convert with checked integer/decimal scaling to exact seconds; require a whole-second, supported-range result. Preserve the existing f64 path for `EXPRESSION`. Compute date gaps at full timestamp precision, test sign before truncation, and reject fractional-second gaps. Sum signed integer seconds with checked wide arithmetic; divide average only when exactly divisible by parent count.

**Rationale**: Current `verify/mod.rs` casts f64 to i64, and `verify/date_diff.rs` calls `num_seconds()` before checking sign. This can truncate claims and subsecond or small negative gaps. `0.1 minutes` must mean exactly six seconds, while a decimal indistinguishable after f64 rounding must not pass accidentally. SQLite already stores Duration as integer seconds.

**Alternatives considered**: f64 conversion plus epsilon is inappropriate for exact provenance. A decimal crate could simplify parsing but adds a runtime dependency and does not remove range checks. Silently rounding fractional-second averages would record a result not equal to its parents.

## 3. Selected-unit display

**Decision**: Convert canonical signed seconds to the selected fixed unit only for presentation. Use exact quotient/remainder arithmetic. If the decimal terminates, show its exact shortest decimal; otherwise show a deterministically rounded decimal prefixed as approximate. Always include exact seconds. On `query --unit`, keep the existing `value` and both query wrapper shapes; add `duration_seconds` as a decimal string and a structured selected-unit display only to the requested Duration step. No unit option leaves the existing response unchanged.

**Rationale**: `src/query/mod.rs` currently emits JSON `lineage_steps` with a `value` string and `src/cli/query.rs` wraps or directly prints that object. Replacing `value` would alter existing consumers. A string retains all signed i64 values even for JSON readers whose numbers have less precision.

**Alternatives considered**: Persisting the selected unit duplicates presentation state in graph nodes. Replacing `value` breaks old query readers. Floating-point display without an approximation marker could be mistaken for an exact input to later calculations.

## 4. Canonical graph identity and historical reuse

**Decision**: Keep existing root and Number-derived ID algorithms. For new Duration derivations, hash a versioned, domain-separated preimage containing operation, value kind, exact signed seconds, and length-delimited parent IDs, retaining the `node_drv_<hex>` outward shape. Keep Date parent order for `DATE_DIFF`; sort distinct parent IDs for `DURATION_SUM`/`DURATION_AVG`. Before inserting, check the historical `DATE_DIFF` ID candidate and reuse it only when stored type, exact seconds, operation, and complete parent set match. Never overwrite an existing row with a conflicting value or unsupported lineage.

**Rationale**: `TrackedNode::generate_derived_id` hashes `ValueData::to_string_repr()` (`src/types/node.rs`), while the Duration display omits seconds from some day-scale values (`src/types/value.rs`). `insert_tracked_node` currently upserts and replaces values on ID conflict (`src/store/queries.rs`). The new preimage makes distinct canonical durations distinguishable. `derivation_edges` stores a set of parent-child pairs, so duplicate parents cannot express multiplicity; clarified behavior rejects them. Sorting gives commutative operations one identity.

**Alternatives considered**: Changing `to_string_repr()` and continuing to hash display changes existing IDs and keeps identity tied to presentation. Adding a multiplicity/order schema migration is unnecessary under duplicate rejection and commutative canonicalization. Blindly reusing an old display-derived ID risks overwriting a different canonical Duration. Previously overwritten values cannot be recovered automatically.

## 5. Transactional graph and Turtle compatibility

**Decision**: Validate operation, units, parent types/count/uniqueness, exact result, legacy candidate, and new-ID collision before writes. Insert node and all edges in one transaction; reuse a fully matching existing derivation without adding edges. Extend Turtle's derivation classifier to recognize the canonical Duration ID scheme in addition to historical IDs and the unchanged Number scheme. Keep typed `xsd:duration` export and project-scoped IRIs.

**Rationale**: The edge table has `UNIQUE(parent_node_id, child_node_id)` and no multiplicity (`src/store/schema.rs`). Turtle currently reconstructs an activity only when it can recompute the child ID with the legacy algorithm (`src/export/turtle.rs`); failing to update it would silently omit the new operation activity. Transactional no-write failures preserve provenance integrity.

**Alternatives considered**: A store-wide migration or graph rescan would increase risk and violate the minimal-subgraph/latency goals. Exporting only parent links for every new typed operation would discard verifiable activity and operation evidence.

## 6. Strict temporal reads and legacy expressions

**Decision**: Typed temporal calculations must fail on missing, malformed, or mismatched stored Date/Duration columns. Prefer a strict node decoder shared by read paths, with a targeted compatibility review for existing malformed stores. Keep legacy `EXPRESSION` coercion available, but emit a recoverable warning on temporal parents; direct agents to typed sum/average. Do not change the numerical meaning of old `EXPRESSION` nodes in this feature.

**Rationale**: `get_tracked_node` currently substitutes current UTC time for an invalid Date and zero for a missing Duration (`src/store/queries.rs`), which could create non-deterministic or false derivations. `verify/expression.rs` converts Dates and Durations to seconds and `cli/derive.rs` records an expression result as Number.

**Alternatives considered**: Preserving silent temporal defaults fails closed behavior. Immediately rejecting all legacy temporal expressions is a breaking CLI change requiring a separate migration decision.

## 7. Verification and performance

**Decision**: Cover exact-decimal syntax and overflow, signed and fractional cases, no-write failures, parent permutations/duplicates, legacy ID reuse/collision, query response preservation, Turtle activity reconstruction, and existing seconds-based commands with deterministic unit/contract/integration tests. Use bounded parent processing and indexed lookup; avoid whole-graph searches. Perform local validation during implementation. Do not invoke CI/CD or GitHub workflows under the current user instruction.

**Rationale**: RGT's constitution requires tests for user-facing behavior, sub-second CLI work, no silent failures, and documentation parity. The project already has unit, contract, and integration test patterns under `tests/`.

**Alternatives considered**: Relying only on arithmetic unit tests misses graph/CLI compatibility. A networked workflow run conflicts with the user's current cost constraint and is unnecessary to produce the plan.
