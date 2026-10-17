# Feature Specification: Value Extraction Correctness

**Feature Branch**: `feat/022-value-extraction-correctness`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "Extracting numeric/date values from file content must be correct: a recognized ISO-8601 date must emit exactly one Date node and must NOT leak its digit components (year/month/day) as spurious Number nodes, and two identical numeric values on the same source line must be tracked as two distinct nodes instead of silently colliding into one. Findings §3.3 (cross-contamination) + §7.3 (same-line collision)."

Source context: `docs/findings-hooks-and-value-extraction.md` §3.3 + §7.3 (plan: `docs/spec-plan-from-findings.md`, spec #4, P1). Out of scope per the plan: locale-aware number parsing (§3.2 → `027-locale-aware-extraction`) and 48-bit node-ID widening (§7.2 → `024-store-graph-integrity`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A Date Emits One Date Node, Never Spurious Numbers (Priority: P1)

A developer reads a file containing `start: 2026-01-15`. RGT must record **one** `Date` node for that line — not three extra `Number` nodes (`2026`, `1`, `15`) alongside it. Today the ISO-date regex and the number regex both run independently over the same line, so every recognized date's digit components are also captured as unrelated numbers, polluting the provenance graph with values the agent never actually read.

**Why this priority**: This is a confirmed correctness bug in the core extraction engine (§3.3) that silently corrupts the value graph for any document containing dates — the single most common date/provenance scenario. The existing test masks it by only asserting `date_count == 3`, never the total extracted count.

**Independent Test**: Extend `tests/unit/test_record.rs::test_extract_dates_from_content` to assert `extracted.len() == 3` (not `>= 3`), i.e. the fixture yields exactly three nodes, all `Date`.

**Acceptance Scenarios**:

1. **Given** the line `start: 2026-01-15`, **When** values are extracted, **Then** exactly one `Date` node is produced and zero `Number` nodes.
2. **Given** three date lines (the existing fixture), **When** extraction runs, **Then** `extracted.len() == 3` and all are `Date`.
3. **Given** a line with an ISO datetime (`2026-01-15T12:30:00Z`), **When** extraction runs, **Then** exactly one `Date` node is produced and no digit fragments leak.

---

### User Story 2 - Identical Values on the Same Line Are Tracked Separately (Priority: P1)

A developer records a CSV row like `revenue,120000,discount,120000` — the same numeric value appears twice on one line. RGT must track **two** distinct nodes, one per occurrence. Today both map to the same root-node ID (the ID hashes only `file_path + line_number + value`), so the second silently merges into the first and is never independently trackable again.

**Why this priority**: Confirmed empirically (§7.3): the CLI reports "2 values" but the graph stores only one node. Silently losing one of two distinct, independently-meaningful values is a data-loss bug in the provenance graph's foundation.

**Independent Test**: Contract/unit tests record `revenue,120000,discount,120000` on one line and assert two distinct nodes are produced (distinct IDs, both present), while a control line with two *different* values still yields two nodes.

**Acceptance Scenarios**:

1. **Given** the line `revenue,120000,discount,120000`, **When** the file is recorded, **Then** two distinct root nodes are created (distinct IDs, both value `120000`).
2. **Given** two different values on one line (`a,100,b,200`), **When** recorded, **Then** two distinct nodes are created (unchanged behavior).
3. **Given** a single value on its own line, **When** recorded, **Then** it produces one node with the same ID as before this change (backward compatible — no duplicate on re-record).

---

### User Story 3 - Extraction Is Deterministic and Re-Recording Dedupes (Priority: P2)

A developer re-records the same unchanged file. RGT must produce the same node IDs as the previous run — no duplicates, no churn. The new occurrence discriminator must be assigned deterministically so the same content always yields the same IDs.

**Why this priority**: The constitution (Principle IX) requires deterministic behavior, and the findings note re-recording already dedupes correctly today; the fix must not regress that.

**Independent Test**: Unit tests assert the same content yields identical occurrence assignment and node IDs across repeated extraction; an integration-style test asserts re-recording a single-value file produces the same IDs.

**Acceptance Scenarios**:

1. **Given** the same file content, **When** extraction runs twice, **Then** both runs produce identical values with identical occurrence discriminators (and therefore identical node IDs).
2. **Given** a file whose lines each contain a single value, **When** re-recorded, **Then** its node IDs are byte-identical to the pre-change scheme (no new duplicate nodes).

---

### Edge Cases

- A date immediately followed by a number with no separator (`2026-01-15,120000`)?
- A line with two identical dates (`2026-01-15, 2026-01-15`)?
- An ISO date embedded in a longer token (e.g. `x2026-01-15y`) — is it recognized?
- A datetime with fractional seconds and timezone (`2026-01-15T12:30:00.123Z`)?
- Non-ISO dates (`27/08/2026`, `08/27/2026`, `27-Aug-2026`) — explicitly out of scope: their fragments still become numbers (documented, addressed by a later locale/date-format spec)?
- Negative numbers (`-42`) and accounting parens (`(500)`) — out of scope (locale spec), still unchanged?
- A value that is exactly at a regex boundary with word characters adjacent?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Extraction MUST recognize ISO-8601 dates first and exclude each matched date's byte span from the number scan, so a recognized date emits exactly one `Date` node and zero spurious `Number` nodes (its year/month/day components never leak).
- **FR-002**: Every extracted value MUST carry a per-line occurrence discriminator (its 0-based index within the line's extracted values, in emission order), and root-node ID generation MUST incorporate that discriminator so two identical values on the same line produce distinct IDs.
- **FR-003**: The occurrence discriminator MUST be included in the ID hash only when it is non-zero, so a line's first (or only) value keeps the pre-change ID — re-recording single-value files dedupes exactly as before (no ID churn for the common case).
- **FR-004**: Extraction MUST be deterministic: the same content yields the same values, in the same order, with the same occurrence assignment (and therefore the same node IDs) on every run.
- **FR-005**: The date-extraction test MUST assert the exact total extracted count (`extracted.len() == 3` for the three-date fixture), not merely the filtered date count, so the cross-contamination regression is caught.

### Key Entities *(include if feature involves data)*

- **ExtractedValue**: A recognized value with its source `line_number` (1-based) and a new per-line `occurrence` (0-based) discriminator. `value` is `Number`, `Date`, or `Duration`.
- **DateSpan**: The byte range of a recognized ISO-8601 date within its line, used to mask it from the number scan.
- **RootNodeId**: Derived from `file_path + line_number + (occurrence when > 0) + value`; unchanged for a line's first value, distinct for same-line repeats.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For `start: 2026-01-15`, `extract_values_from_content` returns exactly 1 node (a `Date`), not 4.
- **SC-002**: For the three-date fixture, `extracted.len() == 3` and all are `Date` (the test that previously only checked `date_count == 3` now fails pre-fix and passes post-fix).
- **SC-003**: Recording `revenue,120000,discount,120000` on one line produces 2 distinct root nodes (distinct IDs).
- **SC-004**: A line with a single value produces the same root-node ID as the pre-change scheme (no duplicate nodes on re-record of the common case).
- **SC-005**: Repeated extraction of identical content yields identical occurrence assignment and identical node IDs (determinism).

## Assumptions

- Scope is the §3.3 cross-contamination bug (ISO dates leaking number nodes) + the broken test assertion + the §7.3 same-line identical-value collision. Locale-aware number parsing (§3.2), non-ISO date-format recognition, and 48-bit node-ID widening are explicitly out of scope (specs 027/024).
- Non-ISO dates remain unrecognized as dates; their digit fragments continue to be extracted as numbers — documented, and addressed by a later date-format/locale spec, not this one.
- The occurrence discriminator is per-line (0-based, in emission order: dates first, then numbers), assigned deterministically, and hashed only when non-zero for backward compatibility (FR-003).
- `extract_values_from_content` is the single extraction entry point (used by both `rgt record` and the hook path); the ID generation change is threaded through `TrackedNode::generate_root_id` and its call sites in `src/cli/record.rs` and `src/hooks/mod.rs`.
- No new dependencies; the existing `regex` crate is sufficient (masking is a span-overlap check, not a new parser).
