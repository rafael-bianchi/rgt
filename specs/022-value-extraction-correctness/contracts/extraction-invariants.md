# Contract: Value Extraction Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

Referenced by the unit tests for `extract_values_from_content` and `TrackedNode::generate_root_id`.

## Extraction invariants

1. **No cross-contamination**: for a recognized ISO-8601 date (and datetime), extraction emits exactly one `Date` value and zero `Number` values from that date's digit components.
2. **Occurrence ordering**: values on a line are emitted in a fixed order — dates first, then non-masked numbers — and each carries `occurrence` = its 0-based index in that sequence.
3. **Determinism**: identical content yields identical `(value, line_number, occurrence)` triples on every run.
4. **Unchanged number extraction**: numbers not adjacent to (and not part of) a recognized date are extracted exactly as before.

## ID invariants

1. `generate_root_id(file, line, occurrence, value)` is deterministic.
2. `generate_root_id(file, line, 0, value)` equals the pre-change ID (only `file + line + value` hashed) — backward compatible.
3. `generate_root_id(file, line, 1, value) != generate_root_id(file, line, 0, value)` for the same `value` — same-line repeats are distinct.

## Acceptance linkage

- US1/FR-001/SC-001/SC-002 → extraction invariant 1.
- US2/FR-002/SC-003 → ID invariant 3.
- US3/FR-003/FR-004/SC-004/SC-005 → ID invariants 1–2 + extraction invariant 3.
