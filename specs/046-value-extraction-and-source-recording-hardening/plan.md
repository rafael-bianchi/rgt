# Implementation Plan: Value Extraction and Source Recording Hardening

**Branch**: `fix/046-value-extraction-and-source-recording-hardening` | **Spec**: `specs/046-value-extraction-and-source-recording-hardening/spec.md`

## Summary of Changes

1. **Unspaced CSV & Locale Number Extraction** (`src/hooks/parser.rs`):
   - Replace greedy `[\d.,]*` tokenization with structured token extraction.
   - Disallow thousands separators following decimal points.
   - Detect unspaced comma delimiters between distinct numbers (`1.5,200` -> `1.5` and `200`; `10,20,30` -> `10`, `20`, `30`; `10,200,300` -> `10`, `200`, `300`).
   - In `resolve_auto_format`, require valid formatted thousands grouping before allowing `,` or `.` to influence locale choice, preventing `1.5,2` from switching the file to `Eu`.
2. **RFC 3339 / ISO-8601 Timestamps** (`src/hooks/parser.rs`):
   - Update `iso_date_regex` to match optional fractional seconds and UTC offsets (`Z` or `[+-]\d{2}:\d{2}`).
   - Support timezone-less local datetimes (`YYYY-MM-DDTHH:MM:SS[.f]`) by treating them as UTC.
   - Mask the byte spans of all regex matches (including invalid calendar dates like `2026-02-30`) so no component digits shatter into `Number` nodes.
3. **Scientific Notation, Unit Suffixes, Currency Negatives & Float Guards** (`src/hooks/parser.rs`):
   - Support scientific notation (`1.5e10`, `-2.5E-3`).
   - Match numbers followed by unit suffixes (`12.5kg`, `99.9ms`, `100%`) without backtracking across the decimal point.
   - In `candidate_context`, look through currency symbols (`$`, `€`, `£`, `¥`) to attach `-` or `(...)` signs.
   - Ignore multi-hyphen prefixes (`--100`).
   - Reject non-finite floats (`f64::is_finite`) and normalize `-0.0` to `0.0`.
4. **`rgt record --stdin` Syntax & Staleness Consistency** (`src/main.rs`, `src/cli/record.rs`, `src/graph/invalidation.rs`):
   - Make `<file>` optional for `Commands::Record` when `--stdin` is set (defaulting to `"<stdin>"`).
   - When an existing disk file is passed with `--stdin`, compute `source_documents.blake3_hash` and metadata from the on-disk file bytes.
   - Ignore virtual sources (`"<stdin>"`) during filesystem staleness evaluation in `evaluate_and_invalidate_all`.
5. **Bounded Passive Hooks & PDF Decompression** (`src/hooks/mod.rs`, `src/extract/pdf.rs`):
   - Enforce `VALUE_LIMIT` (`10_000`) in `handle_passive_hook_event` and `record_pdf_values`.
   - Enforce 10 MiB limit on PDF decoded base64 and decompression output.

---

## Technical Architecture & Edge Cases

### CSV Comma vs. Thousands Separator Resolution
- In US formatting, `,` is a thousands separator ONLY if:
  1. It appears before any decimal point `.`.
  2. The group of digits preceding it has 1..=3 digits, and the group immediately following it has exactly 3 digits (`\d{3}`).
  3. If there is no decimal point and there are multiple unspaced commas (`10,200,300`), it is treated as comma-separated columns unless enclosed in quotes or surrounded by text.
  4. Any comma after a decimal point (`1.5,200`) or followed by non-3 digits (`10,20,30`) splits the candidate into independent numbers.

### Date Span Masking
- Dates are recognized in a first pass. Every candidate span matching `iso_date_regex` is recorded in `date_spans`.
- If `chrono` parses a valid `DateTime<Utc>`, a `ValueData::Date` node is emitted.
- If `chrono` fails (e.g. `2026-02-30`), `skipped_malformed` is incremented.
- In both cases, the span is masked so the number extraction second pass NEVER extracts digits from date candidates.

---

## Test Plan

- Unit tests in `tests/unit/hook_parsers.rs` covering:
  - Unspaced CSV rows: `10,20,30`, `10,200,300`, `1.5,200`, `1.5,2.5`, `12345,678`.
  - Timestamp parsing: `2026-01-15T10:30:00-03:00`, `2026-01-15T10:30:00+05:30`, `2026-01-15T10:30:00`, `2026-02-30`.
  - Scientific notation & units: `1.5e10`, `1.5e-4`, `12.5kg`, `99.9ms`.
  - Currency negative signs: `-$1,200.00`, `($1,200.00)`, `-€500`.
  - Float bounds: 309+ digit strings, `-0.0`.
- Contract tests in `tests/contract/test_cli_record_derive.rs` covering:
  - `echo "Amount: 1234.56" | rgt record --stdin`
  - `rgt record data.csv --stdin` preserving on-disk file hash.
