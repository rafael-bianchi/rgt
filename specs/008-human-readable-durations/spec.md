# Feature Specification: Human-Readable Duration Display

**Feature Branch**: `feat/008-human-readable-durations`

**Created**: 2026-08-01

**Status**: Draft

**Input**: User description: "ux: improve Duration display from raw seconds to human-readable format" (GitHub Issue #5)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Agent Reads a Derived Duration Value (Priority: P1)

An AI coding agent queries RGT for a derived duration — for instance, the elapsed time between a deadline date and today. Instead of receiving `1209600s`, the agent sees `14d 0h 0m`, immediately understanding the magnitude without mental arithmetic. This applies to all output channels: MCP tool responses (`record_derivation`, `query_provenance`), CLI status, and CLI query.

**Why this priority**: Duration values exist to be consumed by agents and developers. Raw seconds defeat this purpose. This is the core UX improvement that the entire issue requests.

**Independent Test**: Run `rgt query <node_id>` on a derived duration node spanning multiple days and confirm the output shows `Xd Xh Xm` format, not raw seconds. Can be tested in isolation by recording a date diff between two dates 14 days apart and verifying the query output.

**Acceptance Scenarios**:

1. **Given** two Date nodes 14 days apart with a derived Duration node, **When** the agent queries that Duration node via MCP `query_provenance` or `rgt query`, **Then** the displayed value is `14d`, not `1209600s`.
2. **Given** a derived Duration of exactly 3661 seconds (1h 1m 1s), **When** displayed, **Then** the output shows `1h 1m 1s`.
3. **Given** a derived Duration of exactly 90 seconds (1m 30s), **When** displayed, **Then** the output shows `1m 30s`.
4. **Given** a derived Duration of 45 seconds, **When** displayed, **Then** the output shows `45s`.

---

### User Story 2 - Negative Duration Display (Priority: P2)

When a date diff produces a negative duration (e.g., deadline is before reference date), the display includes a `-` prefix so the agent can immediately recognize the direction of the temporal difference.

**Why this priority**: Negative durations occur whenever the comparison order is reversed or when a date is in the past relative to a reference point. Without the sign prefix, the agent cannot distinguish past from future.

**Independent Test**: Record two Date nodes where the first date is later than the second, derive the duration, and verify the query output begins with `-`.

**Acceptance Scenarios**:

1. **Given** Date A = 2026-01-15 and Date B = 2026-01-10 with a derived Duration A→B, **When** the duration is displayed, **Then** the output begins with `-` (e.g., `-5d`).
2. **Given** a negative sub-minute duration of -30 seconds, **When** displayed, **Then** the output shows `-30s`.

---

### User Story 3 - Doc Comment for Year/Month Precision Limitation (Priority: P3)

Developers reading the `date_diff()` function are explicitly warned that `chrono::Duration` cannot represent variable-length calendar units (years, months). The doc comment directs callers needing year/month display to use calendar-aware methods on the original Date nodes instead.

**Why this priority**: This is a documentation concern that prevents misuse rather than a user-facing behavioral change. It protects future developers from accidentally treating second-precision durations as year/month precise.

**Independent Test**: Inspect the doc comment on `date_diff()` in the source code. It must mention the year/month precision limitation and suggest alternative approaches.

**Acceptance Scenarios**:

1. **Given** the source file containing `date_diff()`, **When** a developer reads the doc comment, **Then** the comment explicitly states that `chrono::Duration` has no `num_years()` or `num_months()` methods and that callers needing year/month display should use calendar-aware methods on the original Date nodes.

---

### Edge Cases

- What happens when a duration is exactly 0 seconds? Display as `0s`.
- What happens when a duration spans more than 365 days but cannot be split into years/months? Continue displaying in `d h m` format — this is expected and documented behavior per the year/month precision limitation.
- What happens in `rgt status --json` and `rgt query` (non-JSON mode) — must both use the new human-readable format consistently.
- Zero-padding or whitespace: trailing zero units are omitted (e.g., `14d` for exactly 14 days, not `14d 0h 0m 0s`). Middle zero units are shown (e.g., `1h 0m 5s`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST display `Duration` values in human-readable format using day (`d`), hour (`h`), minute (`m`), and second (`s`) unit suffixes, replacing the current raw-seconds-only format.
- **FR-002**: System MUST display durations using the largest applicable unit threshold: days when ≥86,400s, hours when ≥3,600s (but <86,400s), minutes when ≥60s (but <3,600s), seconds otherwise.
- **FR-003**: System MUST prepend a `-` prefix to duration displays when the underlying duration value is negative.
- **FR-004**: System MUST omit trailing zero units from the display when the duration evenly divides into a larger unit — for example, exactly 14 days displays as `14d`, exactly 2 hours displays as `2h` — to improve readability.
- **FR-005**: System MUST apply the human-readable format consistently across all output channels: MCP `record_derivation` responses, MCP `query_provenance` responses, CLI `rgt query` output, and CLI `rgt status` output.
- **FR-006**: The `date_diff()` function's doc comment MUST document the year/month precision limitation of `chrono::Duration` and direct callers to use calendar-aware methods on original Date nodes when year/month display is needed.

### Key Entities

- **Duration Value**: A derived graph node representing the time span between two Date nodes. Displayed in human-readable `d h m s` format rather than raw seconds. No structural change to the entity itself — only its string representation changes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Agent-facing duration values are immediately interpretable without mental arithmetic — a human reader can parse a 14-day duration as `14d` instead of counting digits in `1209600s`.
- **SC-002**: All existing tests that assert on Duration string representations pass after updating expected values to the new format, with no test failures caused by the format change.
- **SC-003**: New test assertions confirm that negative durations, sub-minute durations, and multi-unit durations all render correctly in human-readable format.
- **SC-004**: The `date_diff()` doc comment is present and references the year/month precision limitation, passing documentation review.

## Assumptions

- The human-readable format uses plain unit suffixes (`d`, `h`, `m`, `s`) with space separators and no zero-padding, consistent with the proposed fix in the issue.
- Year and month units are intentionally excluded from the display because `chrono::Duration` cannot represent them precisely. This is documented, not a gap.
- All downstream consumers of Duration string representations (tests, MCP clients) will be updated to expect the new format as part of this change.
- The format applies uniformly to both JSON and human-readable CLI output — there is no separate `--json` path that retains raw seconds.
