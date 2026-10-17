# Feature Specification: Value Extraction and Source Recording Hardening

**Feature Branch**: `fix/046-value-extraction-and-source-recording-hardening`

**Created**: 2026-10-17

**Status**: Draft

**PriorityOrder**: 2 of 6 (P1 — Critical Extraction & Recording Accuracy)

**Input**: Critical review findings §1.1–1.7 (extraction) and §2.5 (CLI `--stdin`): fix unspaced CSV column merging and false rejections in `extract_values_from_content`, fix RFC 3339 / ISO-8601 timezone offset and timezone-less datetime parsing so dates do not shatter into numbers, fix backtracking truncation on scientific notation (`1.5e10`) and unit-suffixed decimals (`12.5kg`), preserve negative signs across currency symbols (`-$1,200.00`, `($1,200.00)`), reject non-finite `f64::INFINITY` and canonicalize `-0.0`, enforce `VALUE_LIMIT` and PDF size/time bounds in passive hooks, and fix `rgt record --stdin` syntax and hash/staleness semantics.

**Strict Non-Overlap Boundary**:
- **Owns**: `src/hooks/parser.rs` (lines 1–354: value/date extraction and `NumberFormat` normalization), `src/extract/pdf.rs`, `src/cli/record.rs`, `src/main.rs` (`Commands::Record` args only).
- **Does NOT touch**: `src/store/*` (`045`), `src/verify/*` or `src/types/*` (`047`), hook event payload dialects in `src/hooks/parser.rs` lines 355+ (`048`), `rgt init`/`rgt doctor` (`049`), or Turtle/updater/CI (`050`).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Accurate Extraction of Unspaced CSV Rows and Locale-Formatted Numbers (Priority: P1)

When RGT extracts values from standard comma-separated data without spaces after commas (e.g., `10,20,30`, `10,200,300`, `1.5,200`, `1.5,2.5`), each cell is extracted as its own distinct `Number` node rather than being merged across commas into a single corrupted number (`10200300.0` or `1.52`) or rejected as malformed.

**Why this priority**: CSV files are the primary tabular input format for numeric provenance; corrupting `10,200,300` into `10200300` or dropping `1.5,2.5` breaks core numeric tracking on standard CSVs.

**Independent Test**: Run `extract_values_from_content` on `"10,200,300\n1.5,200\n10,20,30\n1.5,2.5"` under `NumberFormat::Auto` and `NumberFormat::Us` and verify all 10 individual numbers are extracted accurately with `skipped_malformed == 0`.

**Acceptance Scenarios**:

1. **Given** an unspaced CSV row `1.5,200` or `1.5,2.5`, **When** extracted under `Us` or `Auto`, **Then** RGT extracts `[1.5, 200.0]` and `[1.5, 2.5]` without merging digits after a decimal point across commas and without switching `Auto` mode to `Eu`.
2. **Given** an unspaced CSV row `10,20,30` or `12345,678`, **When** extracted under `Us` or `Auto`, **Then** RGT splits on the comma delimiter and extracts `[10.0, 20.0, 30.0]` and `[12345.0, 678.0]`.
3. **Given** a formatted thousands-grouped number with surrounding whitespace or text (e.g., `Total: 1,234,567.89` in `Us` or `1.234.567,89` in `Eu`), **When** extracted, **Then** RGT requires the leading group before the first thousands separator to be `1..=3` digits and all subsequent thousands groups before the decimal separator to be exactly `3` digits.

---

### User Story 2 - Full ISO-8601 / RFC 3339 Timestamps Never Shatter Into Spurious Numbers (Priority: P1)

When a file contains ISO-8601 / RFC 3339 timestamps with explicit UTC offsets (e.g., `2026-01-15T10:30:00+02:00`, `2026-01-15T10:30:00-03:00`) or timezone-less local ISO timestamps (`2026-01-15T10:30:00`), RGT parses them as single `ValueData::Date` nodes (defaulting timezone-less ISO datetimes to UTC, consistent with `YYYY-MM-DD`) and masks their entire character span so no component digits leak as `Number` nodes.

**Why this priority**: Today, any timestamp with `+HH:MM`/`-HH:MM` or without `Z` fails `parse_from_rfc3339` and shatters into 5–8 bogus `Number` nodes (including negative numbers like `-3` from `-03:00`).

**Independent Test**: Extract values from `"2026-01-15T10:30:00-03:00 and 2026-01-15T10:30:00"` and assert exactly 2 `Date` values and 0 `Number` values are returned.

**Acceptance Scenarios**:

1. **Given** `2026-01-15T10:30:00-03:00` or `2026-01-15T10:30:00+05:30`, **When** extracted, **Then** RGT records a single `ValueData::Date` converted to UTC (`2026-01-15T13:30:00Z` / `2026-01-15T05:00:00Z`) and zero `Number` nodes from the timestamp span.
2. **Given** a timezone-less datetime `2026-01-15T10:30:00` or `2026-01-15T10:30:00.123`, **When** extracted, **Then** RGT parses it as UTC `ValueData::Date` and masks its span from number extraction.
3. **Given** an invalid calendar date matching the regex pattern (e.g., `2026-02-30` or `2026-13-45T99:99:99Z`), **When** `chrono` rejects the date, **Then** its character span is still added to `date_spans` (and counted in `skipped_malformed`) so its digits are NEVER extracted as `Number` nodes.

---

### User Story 3 - Scientific Notation, Unit Suffixes, Currency Negatives & Finite Float Guards (Priority: P1)

RGT accurately extracts scientific notation (`1.5e10`, `1.5E-4`), unit-suffixed decimals (`12.5kg`, `99.9ms`), and negative currency values (`-$1,200.00`, `($1,200.00)`, `-€500`), while rejecting `--100` CLI flags, non-finite `f64::INFINITY` overflows, and normalizing `-0.0` to `0.0`.

**Why this priority**: Regex backtracking currently truncates `1.5e10` to `1.0` and `12.5kg` to `12.0`, while `-$1,200.00` is recorded as `+1200.0` (wrong sign).

**Independent Test**: Run `extract_values_from_content` on `"1.5e10, 1.5e-4, 12.5kg, -$1,200.00, ($500.25), --flag=10, -0.0"` and verify `[1.5e10, 0.00015, 12.5, -1200.0, -500.25, 10.0, 0.0]`.

**Acceptance Scenarios**:

1. **Given** scientific notation `1.5e10`, `-2.5E-3`, or `1e6`, **When** extracted, **Then** RGT parses the full significand and exponent as a single `f64` (`15000000000.0`, `-0.0025`, `1000000.0`).
2. **Given** a number immediately followed by unit letters (`12.5kg`, `99.9ms`, `100%`), **When** extracted, **Then** RGT extracts `12.5`, `99.9`, `100.0` rather than backtracking to the decimal point and extracting `12.0` or `99.0`.
3. **Given** `-$1,200.00`, `($1,200.00)`, `-€500`, or `-£42.50`, **When** extracted, **Then** `candidate_context` looks past leading currency symbols (`$`, `€`, `£`, `¥`, `R$`, `CHF`) to preserve the `-` or `(...)` negative sign (`-1200.0`, `-500.0`, `-42.5`).
4. **Given** `--100` or `---50`, **When** extracted, **Then** a multi-hyphen prefix is not treated as a unary minus sign.
5. **Given** a 309+ digit integer/decimal that overflows `f64` to `Infinity` or a negative zero `-0.0`, **When** normalized, **Then** non-finite values are rejected (`skipped_malformed += 1`) and `-0.0` is canonicalized to `+0.0`.

---

### User Story 4 - `rgt record --stdin` Syntax & Staleness Consistency and Bounded Hook/PDF Capture (Priority: P2)

Agents and users can run both `echo "Amount: 1234.56" | rgt record --stdin` (using a default virtual source path `"<stdin>"`) and `echo "Amount: 1234.56" | rgt record report.pdf --stdin` without false Tier-2 staleness invalidation, and passive hook captures enforce the `10,000` `VALUE_LIMIT` and bounded PDF extraction.

**Why this priority**: Every installed agent instruction file currently tells agents to run `echo "Amount: 1234.56" | rgt record --stdin`, which fails with clap exit code 2, and passing a real file path with `--stdin` stores the stdin hash against the on-disk file's `mtime`, breaking `rgt status`.

**Independent Test**: Run `echo "Amount: 1234.56" | rgt record --stdin` and `echo "Amount: 1234.56" | rgt record report.pdf --stdin`, then run `rgt status` with forced Tier-2 hashing and verify nodes remain `VALID` until `report.pdf` actually changes on disk.

**Acceptance Scenarios**:

1. **Given** `rgt record --stdin` is invoked without a positional `<file>` argument, **When** stdin is provided, **Then** `<file>` defaults to `"<stdin>"` (stored as a virtual source document that is skipped during filesystem staleness checks).
2. **Given** `rgt record <file> --stdin` where `<file>` exists on disk, **When** recording `source_documents`, **Then** `blake3_hash`, `mtime_nsec`, and `file_size` are all computed from the on-disk `<file>` so Tier-2 hashing of `<file>` matches until `<file>` is modified on disk.
3. **Given** a passive hook capture (`handle_passive_hook_event` or `record_pdf_values`) on a file containing `> 10,000` values, **When** extracted, **Then** extraction is capped/gated at `VALUE_LIMIT` (`10_000`) so the store never exceeds Turtle export's `MAX_VALUES` bound.

### Edge Cases

- Unicode letters immediately preceding `-` (e.g., `α-100` or `café-2`): `preceded_by_word` checks UTF-8 alphanumeric characters so `-100` in `α-100` is not treated as negative.
- PDF base64 inputs whose decoded size exceeds `10 MiB` or decompression output exceeds `10 MiB`: rejected cleanly in `extract_pdf_text`.
- `render_additional_context` truncates at `MAX_CONTEXT_CHARS` in a single `char_indices` pass instead of 3 full UTF-8 scans.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `extract_values_from_content` MUST compile its regexes once via `std::sync::LazyLock` and MUST split unspaced comma-separated numeric sequences when they do not form a valid thousands-grouped number (`1..=3` leading digits, `,NNN` groups only before any decimal point).
- **FR-002**: `iso_date_regex` MUST match optional fractional seconds and `(?:Z|[+-]\d{2}:\d{2})?`, parse offset timestamps via `DateTime::parse_from_rfc3339`, parse timezone-less `YYYY-MM-DDTHH:MM:SS[.f]` as UTC, and mask all regex matches (including invalid calendar dates) in `date_spans`.
- **FR-003**: Number extraction MUST support scientific notation (`[+-]?\d+(?:\.\d+)?[eE][+-]?\d+`), preserve decimal digits before unit suffixes (`12.5kg` -> `12.5`), preserve negative signs before currency symbols (`-$1,200.00`, `($1,200.00)`), reject `--` prefixes as negative signs, reject `!value.is_finite()`, and normalize `-0.0` to `0.0`.
- **FR-004**: `Commands::Record` MUST allow `<file>` to be optional when `--stdin` is set (defaulting to `"<stdin>"`), and when `<file>` is an existing disk file with `--stdin`, `source_documents.blake3_hash` MUST hash the on-disk file bytes.
- **FR-005**: Passive hook captures in `src/hooks/mod.rs` MUST enforce `VALUE_LIMIT` (`10_000`).

### Key Entities

- **`ExtractedValue`**: Parsed `Number` or `Date` with 1-based line number and per-line occurrence index.
- **`ExtractionOutcome`**: Extracted values plus `skipped_malformed` counter.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of unspaced CSV test rows (`10,20,30`, `10,200,300`, `1.5,200`, `1.5,2.5`) extract exact cell values with zero cross-column merges.
- **SC-002**: Zero spurious `Number` nodes are produced from RFC 3339 timestamps with `+HH:MM`/`-HH:MM` offsets, local ISO datetimes, or invalid calendar date strings.
- **SC-003**: `echo "Amount: 1234.56" | rgt record --stdin` exits `0` and records `1234.56`.
