# Data Model: Locale-Aware Extraction

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

## 1. NumberFormat (enum)

The active parsing mode controlling separator interpretation.

| Value | Decimal separator | Thousands separator |
|-------|-------------------|---------------------|
| `Us`  | `.`               | `,`                 |
| `Eu`  | `,`               | `.`                 |
| `Auto`| inferred (see below) | inferred |

- `Auto` is the `Default`.
- `Auto` resolves to a concrete format once per file: if any number candidate contains both `.` and `,`, the rightmost separator is the decimal; otherwise `us` (FR-004).
- Parsed from the CLI literals `us`/`eu`/`auto` (case-sensitive); any other value is invalid input (exit 2).

## 2. NumberToken (candidate, transient)

A maximal run matched during extraction, not persisted.

- Contains digits with optional `.`/`,` separators, an optional leading `-`/`+`, and an optional enclosing `(...)`.
- After normalization (per resolved format) it becomes an `f64` in an `ExtractedValue`.
- A candidate that fails normalization is **malformed** → dropped and counted in `ExtractionResult.skipped_malformed`.

Normalization rules (deterministic, per FR-002/FR-005):
1. Enclosing `(...)` or leading `-` → negate; leading `+` ignored.
2. Strip all thousands separators `T`.
3. At most one decimal separator `D`.
4. Every thousands group (digits between `T`s and before the single `D`) is exactly 3 digits.
5. Parse remainder as `f64`.

## 3. ExtractedValue (existing, extended semantics)

Unchanged shape; `value: ValueData::Number(f64)` may now be negative.

## 4. ExtractionResult (new)

Return type of `extract_values_from_content`.

| Field | Type | Meaning |
|-------|------|---------|
| `values` | `Vec<ExtractedValue>` | Successfully extracted values (dates + numbers), as today. |
| `skipped_malformed` | `usize` | Count of number candidates rejected as malformed. |

- `rgt record` warns on stdout when `skipped_malformed > 0`.
- `handle_passive_hook_event` ignores the count (silent skip; never blocks).

## 5. ProjectSetting (new table)

Persisted per-project format.

Table: `project_settings`

| Column | Type | Constraints |
|--------|------|-------------|
| `key`  | TEXT | PRIMARY KEY |
| `value`| TEXT | NOT NULL |

- Key `number_format` holds `us`/`eu`/`auto` (written by `rgt init`, read by `rgt record` and the hook).
- Additive schema migration (`CREATE TABLE IF NOT EXISTS`); no change to existing tables.
- Resolution order: explicit `--number-format` flag > `number_format` setting > `auto`.
