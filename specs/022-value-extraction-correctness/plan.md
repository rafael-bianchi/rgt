# Implementation Plan: Value Extraction Correctness

**Branch**: `022-value-extraction-correctness` | **Date**: 2026-08-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/022-value-extraction-correctness/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Fix two correctness bugs in the value-extraction path (findings §3.3 + §7.3):

1. **Date→number cross-contamination (FR-001)**: `extract_values_from_content` (`src/hooks/parser.rs`) runs the ISO-date regex and the number regex independently over each line, so a recognized `2026-01-15` also emits `2026`, `1`, `15` as spurious `Number` nodes. Fix: collect each line's ISO-date match spans first, then skip any number match whose byte range lies inside a date span.
2. **Same-line identical-value collision (FR-002/FR-003)**: `TrackedNode::generate_root_id` (`src/types/node.rs:57-66`) hashes only `file_path + line_number + value`, so two identical values on one line collide into one node. Fix: add a per-line `occurrence` discriminator to `ExtractedValue` and hash it (only when non-zero) so same-line repeats get distinct IDs while single-value lines keep their pre-change IDs.
3. **Broken test assertion (FR-005)**: `test_extract_dates_from_content` (`tests/unit/test_record.rs`) asserts only `date_count == 3`, letting the 9 spurious number nodes pass; change it to `extracted.len() == 3`.

## Technical Context

**Language/Version**: Rust (edition 2021, stable).

**Primary Dependencies**: `regex` (already present) — the ISO-date and number regexes are reused; masking is a byte-span overlap check, no new parser. **No new dependencies.**

**Storage**: N/A — no schema change; the occurrence discriminator is encoded into the node ID string, not a new column.

**Testing**: `cargo test` — extend `tests/unit/test_record.rs` (extraction count + occurrence/ID determinism) and add unit tests for `generate_root_id` occurrence behavior in `src/types/node.rs`; existing contract/unit suites must stay green.

**Target Platform**: macOS, Linux, Windows (pure string/byte-range logic).

**Project Type**: CLI tool (Rust single binary).

**Performance Goals**: unchanged — the per-line date-span collection and overlap check are O(matches) per line; `extract_values_from_content` remains linear in content size.

**Constraints**:
- FR-001: a recognized ISO-8601 date emits exactly one `Date` and zero `Number` nodes (its digit runs are masked from the number scan).
- FR-002: `ExtractedValue` gains `occurrence: u32` (per-line 0-based index in emission order: dates first, then non-masked numbers).
- FR-003: `generate_root_id` includes `occurrence` in the hash only when `> 0`, so a line's first/only value keeps the pre-change ID (re-record dedupe preserved).
- FR-004: determinism — same content → same values, order, occurrences, IDs.
- FR-005: date fixture test asserts the exact total count.

**Scale/Scope**: rework `src/hooks/parser.rs` (`extract_values_from_content` + `ExtractedValue`), `src/types/node.rs` (`generate_root_id`), and the two call sites (`src/cli/record.rs:58`, `src/hooks/mod.rs:74`); update tests in `tests/unit/test_record.rs` and `src/types/node.rs`. No other surfaces touched.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | No new dependencies. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected. |
| III. Graph Correctness | ✅ PASS | The fix preserves graph correctness — distinct values stay distinct. |
| IV. Rich Value Types | ✅ PASS | Date/Number types unchanged. |
| V. Multi-Channel Distribution | ✅ PASS | Unaffected. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Unaffected. |
| VII. Permissive OSS License | ✅ PASS | No new deps. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Unaffected. |
| IX. Test-Driven Verification | ✅ PASS | Test-first: exact-count date fixture, occurrence/ID determinism, same-line collision. |
| X. Documentation Matches Behavior | ✅ PASS | No user-facing flag/CLI change; behavior change is a bug fix (documented in CHANGELOG if applicable). |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | No new deps; fmt/clippy enforced. |

**Gate Result**: PASS — 11/11; no amendments.

## Project Structure

### Documentation (this feature)

```text
specs/022-value-extraction-correctness/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (extraction design)
├── data-model.md        # Phase 1 output (extraction domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (extraction invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── hooks/
│   ├── parser.rs        # MODIFIED: date-span masking; ExtractedValue.occurrence
│   └── mod.rs           # MODIFIED: pass occurrence to generate_root_id
├── types/
│   └── node.rs          # MODIFIED: generate_root_id gains occurrence param
└── cli/
    └── record.rs        # MODIFIED: pass occurrence to generate_root_id

tests/
└── unit/test_record.rs  # MODIFIED: exact-count date assertion + occurrence tests
```

**Structure Decision**: Single-project layout. The change is localized to the extraction entry point, the ID generator, and the two call sites — no new modules.

## Complexity Tracking

None — no constitution violations; no new dependencies.
