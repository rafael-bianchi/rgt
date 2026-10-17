# Implementation Plan: Human-Readable Duration Display

**Branch**: `008-human-readable-durations` | **Date**: 2026-08-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/008-human-readable-durations/spec.md`

## Summary

Replace the raw-seconds-only `Duration` display format (`1209600s`) with a human-readable multi-unit format (`14d 0h 0m`) in `ValueData::to_string_repr()`. Add a doc comment to `date_diff()` documenting `chrono::Duration` year/month precision limitations. The change is centralized in a single function and propagates to all output channels automatically.

## Technical Context

**Language/Version**: Rust Edition 2021, `Cargo.toml` targets MSRV implied by `chrono 0.4`, `clap 4.5`

**Primary Dependencies**: `chrono 0.4` (DateTime<Utc>, Duration with serde), `serde` / `serde_json` (serialization), `blake3 1.5` (node ID hashing), `rusqlite 0.31` (bundled SQLite)

**Storage**: SQLite via `rusqlite` (bundled, WAL mode) — `.rgt/store.db`. Duration values are stored as `ValueData::Duration(Duration)` serialized via serde. The string representation is computed on read, not stored separately.

**Testing**: `cargo test --all` with `cargo test -- --test-threads=1` for deterministic execution. Unit tests in `tests/unit/test_date_diff.rs`. Integration tests in `tests/integration/test_derivation_chain.rs`.

**Target Platform**: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64) — cross-platform single binary.

**Project Type**: Single Rust binary CLI tool with MCP server.

**Performance Goals**: `to_string_repr()` is called during node ID generation (hashing) and on every display path. The format computation must remain sub-microsecond — simple integer division, no allocation beyond the final String.

**Constraints**: Must not change the storage format or database schema. Node IDs derived from Duration values will change (hash input changed), which means existing `.rgt/store.db` databases will generate different IDs for the same Duration nodes — acceptable for v0.1.0.

**Scale/Scope**: Single function change (`to_string_repr()` in `src/types/value.rs`), one doc comment addition on `date_diff()`, and updated test assertions. 11 call sites across the codebase consume the new format automatically.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | Notes |
|---|---|---|---|
| I | Rust-Only Single Binary | ✅ PASS | Pure Rust change, no new dependencies |
| II | Two-Tier Detection & BLAKE3 | ✅ PASS | No impact on detection path; BLAKE3 hashing of Duration values will produce different hashes (acceptable for v0.1.0) |
| III | Graph Correctness | ✅ PASS | No graph algorithm changes; invalidation unchanged |
| IV | Rich Value Types | ✅ PASS | Enhances Duration display, aligns with the principle of first-class temporal types |
| V | Multi-Channel Distribution | ✅ PASS | No distribution changes |
| VI | Dual Integration Surface | ✅ PASS | MCP responses and CLI output both updated via `to_string_repr()` |
| VII | Permissive OSS | ✅ PASS | No licensing changes |

**Gate result**: All principles pass. No violations to justify.

## Project Structure

### Documentation (this feature)

```text
specs/008-human-readable-durations/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── duration-format.md
└── tasks.md             # Phase 2 output (speckit.tasks)
```

### Source Code (repository root)

```text
src/types/
├── value.rs             # PRIMARY: to_string_repr() Duration variant + date_diff() doc comment
├── node.rs              # SECONDARY: generate_root_id/generate_derived_id use to_string_repr()

src/cli/
├── status.rs            # CONSUMER: to_string_repr() in JSON and text output
├── query.rs             # CONSUMER: delegates to MCP handler
├── graph.rs             # CONSUMER: to_string_repr() in mermaid/dot/text format

src/mcp/
├── handlers.rs          # CONSUMER: to_string_repr() in query_provenance, list_stale_values

tests/unit/
├── test_date_diff.rs    # UPDATE: add format assertions; add doc comment test
```

**Structure Decision**: Single project (Option 1). No new modules or directories needed.

## Complexity Tracking

No violations to justify.
