# Implementation Plan: Translated README Resync

**Branch**: `feat/038-translated-readme-resync` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/038-translated-readme-resync/spec.md`

## Summary

All six translated READMEs (`README_{es,fr,ja,ko,pt,zh}.md`) are frozen at a 221-line pre-hardening snapshot; the English README is now 278 lines with the `Doctor`, expanded `Update` (checksums + attestations + semver no-downgrade), `Self-Update`, `Pre-built Binaries`, `Verify Installation`, `--number-format`, `gc`, non-text-source, and fuller Commands content. Surgically reconcile each translation to the English structure, preserving the already-correct passages and adding the missing sections in each file's established register and terminology.

## Technical Context

**Language/Version**: N/A — Markdown documentation in 6 languages (es, fr, ja, ko, pt, zh).

**Primary Dependencies**: None.

**Storage**: N/A.

**Testing**: Markdown structural checks (balanced fences, table sanity, link counts) + `cargo test` (must stay green — no `src/` changes).

**Target Platform**: All.

**Project Type**: Documentation.

**Performance Goals**: N/A.

**Constraints**: Constitution X (docs match behavior). No `src/` changes. No Intel-Mac claims (spec 035). Preserve existing translations (FR-002). Localized anchors must match localized headings (FR-003).

**Scale/Scope**: 6 files, ~57 lines of English content each to add, translated.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | Docs only. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Localized discovery improved. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | Unaffected. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | Markdown-structure + grep checks; full suite must stay green. |
| X. Documentation matches behavior | PASS | The entire point: translations re-synced to actual behavior. |
| XI. Idiomatic Rust, exit codes | PASS | No code change. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/038-translated-readme-resync/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── readme-parity-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
README_es.md   # reconciled
README_fr.md
README_ja.md
README_ko.md
README_pt.md
README_zh.md
```

**Structure Decision**: Docs-only. Each translation is edited in place; a small verification step (a shell/one-off check or a dedicated test) validates structural parity.

## Complexity Tracking

> None — no constitution violations requiring justification.
