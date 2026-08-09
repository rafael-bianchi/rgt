# Research: Human-Readable Duration Display

**Feature**: `008-human-readable-durations` | **Date**: 2026-08-01

## Decision 1: Modify `to_string_repr()` vs. Create Separate Display Method

**Decision**: Modify `to_string_repr()` directly.

**Rationale**:
- The project is v0.1.0 with no release stability guarantees — backward compatibility is not a requirement at this stage.
- Creating a separate display-only method (e.g., `to_display_string()`) would require updating 11 call sites across `src/cli/`, `src/mcp/`, and `src/types/` to determine which method to use in each context. This adds unnecessary complexity.
- The issue description explicitly proposes modifying `to_string_repr()` — this is the intended approach.
- Node ID generation (`generate_root_id`, `generate_derived_id`) incorporates `to_string_repr()` output into BLAKE3 hashes. After the format change, new node IDs for Duration values will differ from those generated before the change. This means existing `.rgt/store.db` databases with Duration nodes will see ID mismatches if they were created with the old format. Users can regenerate with `rgt init -g --force`. This is acceptable for v0.1.0.

**Alternatives considered**:
1. **Separate `to_display_string()` method**: Keeps hash stability but doubles the API surface and requires call-site decisions (hash vs. display). Rejected as over-engineering for v0.1.0.
2. **Add a `display_format` parameter to `to_string_repr()`**: Adds complexity to every call site. Rejected for the same reason.
3. **Only change display paths, keep hash input as raw seconds**: Would require two different string representations of the same value, increasing cognitive load. Rejected.

## Decision 2: Unit Thresholds and Format Rules

**Decision**: Use the exact format from the issue proposal: days (≥86,400s), hours (≥3,600s), minutes (≥60s), seconds (otherwise). Trailing zero units are omitted; middle zero units are preserved.

**Rationale**:
- These thresholds naturally decompose any `chrono::Duration` into the largest meaningful unit.
- Trailing zero omission (e.g., `14d` not `14d 0h 0m`) improves readability for clean values.
- Middle zero preservation (e.g., `1h 0m 5s`) preserves the precision boundary between displayed and omitted units.
- Year and month units are intentionally excluded because `chrono::Duration` has no `num_years()` or `num_months()` methods — these are variable-length calendar units that depend on the specific date range. This is documented in the `date_diff()` doc comment.

**Alternatives considered**:
1. **Always show all units (d/h/m/s) even when zero**: Verbose, reduces readability. Rejected.
2. **Show years/months via approximate calculation**: Would be imprecise and misleading since year/month length varies. Rejected — documented as a known limitation instead.
3. **Use ISO 8601 duration format (P14DT0H0M)**: Less readable for agent consumption. Rejected.

## Decision 3: Negative Duration Handling

**Decision**: Prepend `-` prefix to the human-readable string when `num_seconds() < 0`.

**Rationale**:
- `chrono::Duration` can be negative (when date2 < date1 in `date_diff()`).
- A leading `-` sign is unambiguous and naturally conveys direction.
- Using `abs()` for unit decomposition ensures the magnitude is computed correctly regardless of sign.

**Alternatives considered**:
1. **Use "ago" suffix (e.g., "14d ago")**: Changes the semantic from a signed value to a relative description. Rejected — a signed duration is more precise for computation graphs.
2. **Output zero for negative durations**: Hides information. Rejected.

## Decision 4: No `--json` Split Path for Format

**Decision**: The human-readable format applies uniformly to both JSON and human-readable CLI output. There is no separate raw-seconds path retained for JSON.

**Rationale**:
- The spec (FR-005) explicitly requires consistency across all channels.
- JSON consumers (MCP clients) benefit from the same readability improvement.
- If raw seconds are needed programmatically, consumers can parse the existing `duration_seconds` field in `record_derivation` input/output instead of parsing the display string.

**Alternatives considered**:
1. **Keep raw seconds in JSON, human-readable in text output**: Violates FR-005 and creates an inconsistent API. Rejected.
