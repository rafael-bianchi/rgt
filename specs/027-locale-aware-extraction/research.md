# Research: Locale-Aware Extraction

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-29

All decisions are grounded in the current `src/hooks/parser.rs` (`extract_values_from_content`, number regex `\b(\d+(?:\.\d+)?)\b`) and `src/store/` (SQLite via `rusqlite`). Findings §3.2 already established that no drop-in Rust crate parses locale numbers (ICU4X `icu_decimal` is formatting-only, ECMA-402 formatting-only), so parsing is hand-rolled.

## 1. NumberFormat enum + extraction signature (FR-001–FR-004)

**Decision**: Add a `NumberFormat` enum (`Us`, `Eu`, `Auto`, `Default = Auto`) in `src/hooks/parser.rs`; change `extract_values_from_content(content: &str)` to `extract_values_from_content(content: &str, format: NumberFormat)`; change the return type from `Vec<ExtractedValue>` to `ExtractionResult { values: Vec<ExtractedValue>, skipped_malformed: usize }`.

**Rationale**: `Auto` is the documented default and the passive hook has no flag, so the enum carries `Default = Auto`. The return type must expose the malformed-skip count so `rgt record` can warn (FR-005) while hooks stay silent.

**Alternatives considered**: keeping a separate `extract_values_from_content_with_format` and leaving the old function for backward compat — rejected: two entry points invite drift; the spec's default change is intentional and existing tests are updated in the same changeset (Constitution IX).

## 2. Number-candidate tokenization (FR-001, FR-002)

**Decision**: After ISO-date masking (existing, FR-001 of the prior feature), match a number candidate as a maximal run of digits with `.`/`,` separators, an optional leading `-`/`+`, and an optional enclosing accounting pair `(...)`, bounded by non-digit/non-separator characters. The candidate is then validated/normalized per the resolved format (see §4). A candidate that fails validation is counted in `skipped_malformed` and dropped.

**Rationale**: A single candidate regex that *includes* sign and both separators prevents the old `1,234.56` → `1` + `234.56` split. Validation (not the regex) decides whether the separators are well-formed, so a malformed token is rejected whole rather than re-split.

**Alternatives considered**: separate sign/paren and separator regexes composed at runtime — rejected: a single candidate capture plus a deterministic normalizer is simpler to reason about and test.

## 3. Sign and parentheses normalization (FR-001)

**Decision**: A leading `-` negates; a leading `+` is ignored; an enclosing `(...)` negates. These apply before separator normalization and are locale-independent (always on). `-(500)` negates the parenthesized magnitude (double-negative → positive, matching the plain-arithmetic reading).

**Rationale**: Accounting negatives are standard (`(500)` = -500). Applying sign before separator stripping keeps the transform composable and easy to unit-test.

## 4. Separator normalization per format (FR-002, FR-005)

**Decision**: For a resolved concrete format, the decimal separator `D` and thousands separator `T` are fixed (`us`: `D=.`/`T=,`; `eu`: `D=,`/`T=.`). Normalize a candidate by: stripping sign/parens → removing all `T` characters → requiring at most one `D` → requiring every thousands group (the digits between `T`s and before the single `D`) to be exactly 3 digits → parsing the remainder as `f64`. Any violation marks the candidate malformed (skip + count).

**Rationale**: The 3-digit-group rule distinguishes a valid `1,234` (thousands) from a bogus `1,23,456` and rejects a misplaced separator instead of silently dropping digits — this is what "never silently split" means. It is deterministic and locale-pinned once `D`/`T` are fixed.

**Alternatives considered**: accept any separator arrangement and just strip non-decimal chars — rejected: that would accept `1,2,3` as `123`, masking data errors.

## 5. `auto` inference, once per file (FR-004)

**Decision**: When the resolved format is `Auto`, do one pre-pass over the whole content: collect separator characters (`.`/`,` only) inside number candidates. If any candidate contains **both** `.` and `,`, the decimal separator is the **rightmost** separator in that candidate and the format becomes the corresponding concrete format (`us` or `eu`). If no candidate contains both, fall back to `us`. The resulting concrete format is then applied to every line.

**Rationale**: Matches the clarified rule (rightmost-wins when both present; single separator → `us`) and the clarified granularity (once per file, same format for all lines). A single pre-pass keeps extraction effectively linear.

**Alternatives considered**: per-line inference — rejected during clarification: mixed-locale files would flip parse rules mid-file.

## 6. Persistence via `project_settings` (FR-003)

**Decision**: Add a `project_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)` table to `src/store/schema.rs` (idempotent `CREATE TABLE IF NOT EXISTS`), plus `get_setting`/`set_setting` helpers in `src/store/queries.rs`. `rgt init --number-format X` writes key `number_format` = `X`; `rgt record` and `handle_passive_hook_event` read it when no explicit flag is given. Resolution order: explicit `--number-format` flag > persisted setting > `auto`.

**Rationale**: Both the record and hook paths already open `.rgt/store.db`, so the setting is read with zero new I/O or file-locking concerns, and it survives the passive path (where the agent can't pass a flag).

**Alternatives considered**: a `.rgt/config` file — rejected during clarification: a second source of truth plus atomic-write handling; the DB is already the single source of truth.

## 7. CLI surface and error semantics (FR-003, Constitution XI)

**Decision**: `rgt record --number-format us|eu|auto` and `rgt init --number-format us|eu|auto`. The value is validated by a clap `value_parser` restricted to the three literals; any other value is invalid input → exit code 2. `rgt record` prints a one-line warning to stdout when `skipped_malformed > 0`.

**Rationale**: Constitution XI requires invalid input to exit 2 and silent failures are prohibited; the warning surfaces a misconfigured format without blocking the passive hook (which simply ignores the count).

**Alternatives considered**: silently accepting unknown formats as `auto` — rejected: hides misconfiguration.
