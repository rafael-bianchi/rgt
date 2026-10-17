# Data Model: Agent Instructions Feature Parity

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data. The entity is the installed instruction artifact.

## 1. GlueArtifact

An instruction text RGT writes at `rgt init` for a rules-file/prompt/instructions-file agent. Must teach the canonical feature set.

## 2. CanonicalFeatureSet (the parity target)

Taught by the repo's `AGENTS.md` (current): `record <file> [--number-format us|eu|auto]`, `derive`, `status`, `query`, `graph`, `gc [--vacuum]`, `doctor`, plus the `with_instruction` non-text-source block.

| Artifact | Style | Additions |
|---|---|---|
| CLINE_RULES | structured (`###`) | number-format in record; gc + doctor bullets |
| ANTIGRAVITY_RULES | structured (`##`) | same |
| KILO_RULES | structured (`##`) | same |
| VIBE_PROMPT | compact (prose) | number-format in record; doctor + gc sentences |
| COPILOT_CLI_RULES | structured (`###`) | number-format in record; gc + doctor bullets |
| codex_section() | structured (`###`) | same |
| Windsurf base | compact (prose) | number-format in record; doctor + gc sentences |

## 3. with_instruction wrapper (unchanged)

`NON_TEXT_SOURCE_INSTRUCTION` + `RGT_END_MARKER_LINE` appended by `glue::with_instruction` at write time — preserved byte-for-byte.
