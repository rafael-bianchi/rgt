# Contract: Number-Format Invariants

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-29

These invariants are the executable contract for the `--number-format` feature. They are verified by `tests/unit/hook_parsers.rs` and `tests/contract/` tests.

## I. Flag surface

- `rgt record --number-format <us|eu|auto>` — optional; defaults to the persisted setting, then `auto`.
- `rgt init --number-format <us|eu|auto>` — optional; persists to `project_settings` key `number_format`.
- Invalid value → exit code 2 (invalid input). Accepted literals are exactly `us`, `eu`, `auto`.

## II. Sign & parentheses (locale-independent, always on)

| Input | Result |
|-------|--------|
| `-42` | `Number(-42.0)` |
| `+42` | `Number(42.0)` |
| `(500)` | `Number(-500.0)` |
| `-(500)` | `Number(500.0)` |

## III. Separator interpretation (per resolved format)

| Format | Input | Result (one node) |
|--------|-------|-------------------|
| `us` | `1,234.56` | `Number(1234.56)` |
| `eu` | `1.234,56` | `Number(1234.56)` |
| `us` | `1,234` | `Number(1234.0)` |
| `eu` | `1,234` | `Number(1.234)` |
| `auto` | `1,234.56` | `Number(1234.56)` |
| `auto` | `1.234,56` | `Number(1234.56)` |
| `auto` | `1,234` | `Number(1234.0)` (single separator → `us`) |
| `auto` | `1.234` | `Number(1.234)` (single separator → `us`) |

## IV. Malformed handling (FR-005)

- A candidate that fails normalization under the active format is skipped and counted, never emitted and never split into bogus nodes.
  - Example: `1.234.567,89` under `us` → malformed (two `.` in `us` grouping) → skipped.
- `rgt record` prints a warning to stdout when the skip count is > 0.
- Passive hooks skip silently (no output, never block).

## V. Persistence & resolution (FR-003)

- `rgt init --number-format eu` → `project_settings.number_format = "eu"`.
- Subsequent flagless `rgt record`/hook capture parses `eu`.
- Explicit `rgt record --number-format us` overrides a persisted `eu`.
- No flag + no setting → `auto`.

## VI. Non-regression (Constitution VIII, IX)

- Hooks never rewrite or block commands; extraction is capture-only.
- Existing ISO-date masking still applies (dates never leak number nodes).
- `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
