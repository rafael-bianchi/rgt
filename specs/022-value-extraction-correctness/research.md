# Research: Value Extraction Correctness

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Resolves the plan's Technical Context unknowns by hand-tracing the current extraction code (no external fetch needed). The `regex` crate API facts used below are confirmed against the project's existing usage.

## 1. Date-span masking (FR-001)

**Decision**: In `extract_values_from_content`, for each line collect the byte spans of all recognized ISO-8601 dates first, then skip any number match whose `[start, end)` lies within a date span.

**Rationale**: The bug is that the number regex `\b(\d+(?:\.\d+)?)\b` matches each digit run of `2026-01-15` independently of the date regex. `regex::Captures::get(0)` returns the full match with `.start()`/`.end()` byte offsets, so a date's span is exact. A number match is "inside a date" iff `num.start >= date.start && num.end <= date.end` (dates always wrap their digit runs fully). This is a minimal, deterministic O(matches) change — no combined-alternation regex needed, and it keeps the two regexes readable.

**Alternatives considered**:
- Single combined pattern `(?P<date>...)|(?P<number>...)` (the findings doc's suggestion) — equally correct, but it restructures the whole extraction loop and makes "skip masked spans" less obvious; the span-masking approach is a smaller diff against the existing two-regex structure.
- Post-hoc dedup of numbers that are substrings of dates — rejected: string-based, fragile to overlapping/nearby digits.

## 2. Per-line occurrence discriminator (FR-002/FR-003)

**Decision**: Add `occurrence: u32` to `ExtractedValue` = the value's 0-based index within its line's emitted values (dates first, then non-masked numbers). In `generate_root_id(file, line, occurrence, value)`, hash `occurrence` only when `> 0`.

**Rationale**: The collision is between identical values on the same line; `line_number` is already in the hash, so the only missing axis is "which occurrence within the line". A per-line counter is the least-code option (vs. column offset), and hashing it only when non-zero keeps single-value lines byte-identical to the pre-change IDs — preserving the findings-confirmed re-record dedupe for the common case (no ID churn on upgrade). Determinism holds because iteration order is fixed.

**Alternatives considered**:
- Column offset of the match (byte position within the line) — also unique, but shifts the ID whenever surrounding punctuation changes; the counter is more stable and less code.
- A global (whole-file) counter — works but unnecessary; per-line is the precise axis of the collision.
- Always hashing occurrence — simpler but changes every node ID on upgrade, creating duplicate nodes for existing data; rejected for backward compatibility.

## 3. Call-site threading

`generate_root_id` is called from exactly two places — `src/cli/record.rs:58` and `src/hooks/mod.rs:74` — both already in scope and both must pass `ext.occurrence`. No other producers of root IDs exist (verified via grep).

## 4. Test strategy

- `tests/unit/test_record.rs::test_extract_dates_from_content` → assert `extracted.len() == 3` (FR-005/SC-002).
- New unit tests: `extract_values_from_content` on `start: 2026-01-15` → len 1 (Date only); on `revenue,120000,discount,120000` → two values with occurrences 0 and 1; determinism (two runs equal).
- `src/types/node.rs` unit tests: `generate_root_id(f, line, 0, v) == generate_root_id(f, line, 0, v)` (deterministic), and `generate_root_id(f, line, 0, v) != generate_root_id(f, line, 1, v)` for the same value (collision fixed), while the occurrence-0 ID equals the pre-change formula.
