# Data Model: Value Extraction Correctness

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

Domain model for `extract_values_from_content` and root-node ID generation.

## Entities

### ExtractedValue

A recognized value from a line of source content.

| Attribute | Type | Notes |
|---|---|---|
| `value` | `ValueData` | `Number` / `Date` / `Duration` (unchanged). |
| `line_number` | `Option<u32>` | 1-based source line. |
| `occurrence` | `u32` | **New** — 0-based index of this value within its line's emitted values (emission order: dates first, then non-masked numbers). |

Rules:
- Deterministic: identical content → identical `(value, line_number, occurrence)` triples (FR-004).
- Two values on the same line always differ in `occurrence`; two identical values on the same line differ only in `occurrence` (FR-002).

### DateSpan

The byte range `[start, end)` of a recognized ISO-8601 date within its line.

| Attribute | Type | Notes |
|---|---|---|
| `start` | `usize` | Byte offset of the date match start. |
| `end` | `usize` | Byte offset of the date match end. |

Rules:
- A number match is suppressed iff its range lies within some `DateSpan` (FR-001).

### RootNodeId

Derived from the extracted value's identity.

| Attribute | Type | Notes |
|---|---|---|
| `file_path` | `String` | Source document path. |
| `line_number` | `Option<u32>` | Source line. |
| `occurrence` | `u32` | Hashed into the ID **only when > 0** (FR-003). |
| `value` | `ValueData` | Serialized via `to_string_repr`. |

Rules:
- A line's first/only value (`occurrence == 0`) produces the pre-change ID — backward compatible, re-record dedupes (FR-003).
- A same-line repeat (`occurrence >= 1`) produces a distinct ID — no silent collision (FR-002).
- Deterministic in all inputs (FR-004).

## Validation rules (from FR-001..FR-005)

- **No leak**: a recognized date yields exactly one `Date`, zero `Number` (FR-001).
- **Distinct repeats**: identical same-line values → distinct IDs (FR-002).
- **Backward compat**: occurrence-0 IDs unchanged (FR-003).
- **Determinism**: identical runs → identical IDs (FR-004).
- **Exact-count test**: date fixture asserts total length (FR-005).

## Out of scope

Locale-aware number parsing (§3.2), non-ISO date recognition, 48-bit ID widening (§7.2), and storage/schema changes.
