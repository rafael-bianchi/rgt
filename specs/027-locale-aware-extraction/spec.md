# Feature Specification: Locale-Aware Extraction

**Feature Branch**: `feat/027-locale-aware-extraction`

**Created**: 2026-08-29

**Status**: Draft

**Input**: User description: "Numeric extraction must respect locale conventions and sign semantics: a leading minus and accounting parentheses must be part of the number, and decimal/thousands separators must be interpreted per a configured format (US `1,234.56` vs EU `1.234,56`) rather than splitting every digit run. Findings §3.2."

Source context: `docs/findings-hooks-and-value-extraction.md` §3.2 (plan: `docs/spec-plan-from-findings.md`, spec #9, P2).

## Clarifications

### Session 2026-08-29

- Q: How should `auto` disambiguate a number with a single separator (e.g. `1,234` or `1.234`)? → A: Rightmost-separator-wins when both `.` and `,` are present; a single separator falls back to `us` (`.` = decimal, `,` = thousands).
- Q: Should `auto` infer per file or per line? → A: Infer once per file; apply the same format to every line.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Signs Are Part of the Number (Priority: P1)

A developer records `-42` or an accounting-style `(500)`. Today the leading `-` is not matched, so `-42` is extracted as `42` (a wrong value recorded as trustworthy), and `(500)` as `500`. Sign handling is locale-independent and must always be applied: a leading `-` belongs to the number, and parentheses denote a negative amount.

**Why this priority**: §3.2 HIGH failure modes — a silently-wrong sign is a correctness bug in the core extraction, independent of any locale flag.

**Independent Test**: Unit tests assert `-42` extracts as `Number(-42.0)` and `(500)` as `Number(-500.0)`.

**Acceptance Scenarios**:

1. **Given** the text `-42`, **When** extraction runs, **Then** a single `Number(-42.0)` is produced (not `42`).
2. **Given** the text `(500)`, **When** extraction runs, **Then** a single `Number(-500.0)` is produced (not `500`).
3. **Given** the text `+42`, **When** extraction runs, **Then** `Number(42.0)` is produced (optional unary plus tolerated).

---

### User Story 2 - Decimal and Thousands Separators Are Interpreted Per Locale (Priority: P1)

A developer records `1,234.56` (US) or `1.234,56` (EU). Today `.` is the only decimal separator and there is no thousands-separator awareness, so `1,234.56` becomes two bogus nodes (`1` and `234.56`) and `1.234,56` becomes three (`1`, `234`, `56`). With a chosen format, the whole number must be one node with the correct value.

**Why this priority**: §3.2 HIGH — the most common real-world case (financial/CSV data in non-US locales) currently produces garbage nodes.

**Independent Test**: Unit tests assert `1,234.56` under `us` → `Number(1234.56)` (one node), and `1.234,56` under `eu` → `Number(1234.56)` (one node).

**Acceptance Scenarios**:

1. **Given** `--number-format us` and the text `1,234.56`, **When** extraction runs, **Then** one `Number(1234.56)` node is produced.
2. **Given** `--number-format eu` and the text `1.234,56`, **When** extraction runs, **Then** one `Number(1234.56)` node is produced.
3. **Given** `--number-format us` and the text `1.234.567,89` (a malformed-for-us value), **When** extraction runs, **Then** it is skipped and counted, never silently split into bogus nodes (and `rgt record` warns on stdout).

---

### User Story 3 - The Format Persists for Passive Capture (Priority: P2)

A developer configures the locale once with `rgt init --number-format eu`. Passive hook captures (`rgt hook post`) then use that format automatically, so non-US numbers are captured correctly without the agent passing a flag on every call.

**Why this priority**: The hook path is passive — the agent can't pass `--number-format` per event. Persisting the format (via `rgt init` into a `project_settings` table in the SQLite store) is what makes the fix reach the passive-capture path.

**Independent Test**: Contract tests assert `rgt init --number-format eu` persists the setting, and a subsequent `rgt record`/hook capture uses `eu` parsing.

**Acceptance Scenarios**:

1. **Given** `rgt init --number-format eu`, **When** a later `rgt record` runs without a flag, **Then** it parses numbers in `eu` format.
2. **Given** `rgt init --number-format eu` then `rgt record --number-format us`, **When** the record runs, **Then** the per-call `us` flag overrides the persisted `eu`.
3. **Given** no flag and no persisted setting, **When** extraction runs, **Then** it uses `auto` (inference) as the default.

---

### User Story 4 - `auto` Infers the Locale (Priority: P2)

A developer doesn't know or want to set a locale. `--number-format auto` infers the decimal separator: when both `.` and `,` appear in a number, the rightmost is the decimal; when only one appears, it falls back to `us` (`.` decimal, `,` thousands).

**Why this priority**: §3.2 suggests `auto` as an option; it reduces friction and is the sensible default for mixed or unknown sources.

**Independent Test**: Unit tests assert `auto` on `1,234.56` yields `1234.56` and on `1.234,56` yields `1234.56` (via the inference heuristic).

**Acceptance Scenarios**:

1. **Given** `--number-format auto` and `1,234.56`, **When** extraction runs, **Then** one `Number(1234.56)` node is produced.
2. **Given** `--number-format auto` and `1.234,56`, **When** extraction runs, **Then** one `Number(1234.56)` node is produced.

---

### Edge Cases

- A number with both a thousands separator and a decimal (`1,234,567.89`)?
- A bare thousands-separated integer (`1,234` under `us`)?
- A number adjacent to a comma in prose (`costs,60000` — is the comma a separator or punctuation)?
- A percent/currency symbol (`$1,234.56`, `12.5%`) — the symbol is dropped but the magnitude is kept?
- `1,234` under `eu` → `1.234` (`,` is the decimal separator; resolved, see contract III).
- A leading `-` immediately before `(500)` (`-(500)`)?
- An exponent form (`1.2e3`) — unchanged?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Extraction MUST include a leading `-` (and tolerate an optional leading `+`) as part of the number, and MUST interpret parenthesized numbers `(N)` as `-N` (accounting style). This is locale-independent and always on.
- **FR-002**: Extraction MUST interpret decimal and thousands separators according to a selectable number format (`us`: `.` decimal / `,` thousands; `eu`: `,` decimal / `.` thousands), so a correctly-grouped number is a single node with the right value.
- **FR-003**: `rgt record` MUST accept a `--number-format us|eu|auto` flag; `rgt init` MUST accept the same flag and persist it to the `project_settings` table so the passive hook path uses it. The default (no flag, no persisted setting) is `auto`.
- **FR-004**: `--number-format auto` MUST infer the decimal separator deterministically: when both `.` and `,` appear in a number, the rightmost separator is the decimal; when only one separator type appears, fall back to `us` (`.` = decimal, `,` = thousands). The inference is performed once per file and the resulting format is applied to every line of that file.
- **FR-005**: A number that cannot be parsed under the active format MUST be skipped (never silently split into bogus nodes); `rgt record` MUST warn on stdout, and passive hooks MUST skip silently (never block the agent).

### Key Entities *(include if feature involves data)*

- **NumberFormat**: The active parsing mode — `Us`, `Eu`, or `Auto` — controlling decimal/thousands separator interpretation.
- **NumberToken**: A numeric string recognized by extraction, including sign and separator handling.
- **ProjectSetting**: The persisted per-project format stored in the `project_settings` key/value table (written by `rgt init`, read by `rgt record` and the hook).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `-42` and `(500)` extract as `-42` and `-500` respectively (unit-tested), regardless of format.
- **SC-002**: `1,234.56` under `us` and `1.234,56` under `eu` each yield a single `Number(1234.56)` node (unit-tested).
- **SC-003**: `rgt init --number-format eu` persists the setting and a subsequent flagless `rgt record` parses `eu` (contract-tested).
- **SC-004**: `auto` correctly infers both `1,234.56` and `1.234,56` (unit-tested).
- **SC-005**: A malformed-for-format number is skipped (and warned on `record`), not silently split (unit-tested).

## Assumptions

- Default format is `auto` (inference), not `us`. This is a deliberate behavior change: previously `1,234.56` split into `1` + `234.56`; now it becomes a single `1234.56`. Affected node IDs change accordingly (intended).
- Persistence is a per-project setting in a new `project_settings` key/value table in the existing SQLite store (both `rgt record` and the hook path already open the DB, so no new file or atomic-write concern). There is no new dependency (the findings confirmed no drop-in Rust locale-number-parsing crate exists — ICU4X `icu_decimal` is formatting-only, ECMA-402 is formatting-only).
- The `auto` heuristic (rightmost-separator-wins when both `.` and `,` are present; single separator falls back to `us`) is deterministic and documented in FR-004.
- Percent/currency symbols are dropped (magnitude kept); exponent forms are out of scope and unchanged.
- Sign/parentheses handling (FR-001) applies to the hook path and `rgt record` equally, since they share `extract_values_from_content`.

## Out of scope

Native binary parsing, date-format recognition (§3.3's non-ISO half), and any change to node-ID generation.
