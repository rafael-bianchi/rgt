# Feature Specification: Translated README Resync

**Feature Branch**: `feat/038-translated-readme-resync`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "All 6 translated READMEs are frozen at a pre-hardening-pass snapshot — identically 221 lines (vs. English's 278), zero mentions of 'doctor,' 'gc,' 'number-format,' or 'attestation' across all six. Re-sync all 6 translations against the current English README."

Source context: `docs/weakpoints-and-growth-opportunities.md` §3 item 1. Plan: `docs/spec-plan-from-growth-opportunities.md`, spec #5, P1.

## Clarifications

### Session 2026-08-31

- Q: Re-translate the entire file from scratch, or surgically insert the missing sections into each translation while preserving the existing (correct) translated passages? → A: Surgically reconcile each translation to the English README's current structure — the existing translations are high-quality for the sections they have (they match the English pre-hardening snapshot); the gap is entirely the sections added since (`Doctor`, the expanded `Update`/verification prose, `--number-format`, `gc`, the non-text-source note, pre-built-binaries/self-update details, `verify installation`, and the fuller Commands table). Preserve what's already correct; add the missing content translated with the same register and terminology each file already uses.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Non-English Users Can Discover the Full Toolset (Priority: P1)

A Spanish-, French-, Chinese-, Japanese-, Korean-, or Portuguese-speaking developer lands on the localized README and reads about `rgt init`, `rgt record`, and `rgt derive` — but never learns that `rgt doctor` exists to diagnose their most likely failure mode (hooks installed, nothing captured), that `rgt gc` can reclaim their store, or that `--number-format` handles their locale's `.`/`,` convention. The English README documents all of this; the six translations do not, because they were frozen before specs 019–029 shipped. After this fix, a non-English reader gets the same complete picture as an English reader.

**Why this priority**: P1 per the growth plan — non-English users currently have no localized path to discover the exact tool (`rgt doctor`) built to diagnose the most common failure mode; user-facing, not just internal hygiene.

**Independent Test**: Each translated README contains the localized equivalents of the `Doctor`, `Self-Update`, `Pre-built Binaries`, and `Verify Installation` sections, mentions `gc`/`--number-format`/attestation in its Commands/Update prose, and its line count tracks the English structure (i.e., the missing sections are present).

**Acceptance Scenarios**:

1. **Given** `README_es.md`, **When** it is compared to `README.md`, **Then** it contains a Doctor section, a Self-Update subsection, a Pre-built Binaries subsection, a Verify Installation subsection, and Commands entries for `gc` and `--number-format`, all in Spanish.
2. **Given** each of the 6 translated READMEs, **When** the sections that exist in both are compared, **Then** the pre-existing translations are preserved (not needlessly rewritten) and the added sections match the English structure and meaning.
3. **Given** any translated README, **When** its content is checked for the hardening-pass feature names, **Then** it mentions the localized `doctor`, `gc`, `--number-format`, and the update verification (attestation) flow.
4. **Given** the docs directory, **When** all READMEs are linted, **Then** no markdown is malformed (code fences balanced, tables intact) and the language-switcher links in every file (including the English original) still point at all 7 variants.

---

### Edge Cases

- Anchor links (`#doctor`, `#comandos`, etc.) must match each file's actual section headings (localized slugs differ per language) — the English TOC anchors must NOT be copied verbatim into translations.
- The `on_macos`/Intel note and the pre-built-binaries arch list must not introduce a claim the release doesn't support (e.g. don't advertise an Intel-Mac binary — see the Out-of-scope note).
- Tables (Commands, Supported AI Tools) must keep valid markdown after translation.
- The `rgt update` verification prose must faithfully convey "checksums + attestation + no silent downgrade" without inventing details.
- Terminology consistency per file: each translation should use the terms it already established (e.g. "hook" vs "gancho", "nodos" vs "nodes") rather than mixing.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each of the 6 translated READMEs (`README_{es,fr,ja,ko,pt,zh}.md`) MUST document the full current feature set: `rgt doctor`, `rgt gc [--vacuum]`, `--number-format`, the self-update verification flow (checksums + attestations + semver no-downgrade), the non-text-source capture note, and the complete Commands table — in that file's language.
- **FR-002**: The existing, already-accurate translated passages MUST be preserved (no gratuitous re-translation or content loss).
- **FR-003**: Each translated README's internal anchors/TOC MUST match its own (localized) section headings; the language-switcher bar in every README (all 7 files) MUST reference all 7 variants correctly.
- **FR-004**: No markdown corruption: balanced code fences, well-formed tables, valid links — verified by a markdown lint or structural check.

### Key Entities *(include if feature involves data)*

- **TranslatedReadme**: one of `README_{es,fr,ja,ko,pt,zh}.md`, each mapping the English README's sections into its language.
- **SectionParity**: the completeness criterion — every English section has a localized counterpart (or a documented intentional omission).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 6 translated READMEs gain the Doctor, Self-Update, Pre-built Binaries, and Verify Installation sections (grep-checkable by section heading per language).
- **SC-002**: Grep for the localized `doctor`/`gc`/`number-format`/attestation terms returns ≥1 hit in each of the 6 files.
- **SC-003**: The language-switcher bar in all 7 READMEs links all 7 variants (link-count check per file).
- **SC-004**: A markdown structural check (balanced fenced code blocks; table row counts sane) passes for all 6 translations.

## Assumptions

- The English README is the source of truth; translations must match its current structure (278-line English with the hardening-pass sections).
- The existing translations are trustworthy for the content they already carry; only the missing sections are added (and any English-side text changed since the snapshot is reconciled).
- Translations are human-quality, not machine-verbatim: the meaning must be faithful, and the register should match each file's existing style.
- This is a docs-only change; no code, no tests to run beyond markdown checks and the full `cargo test` (which must stay green since no `src/` changes).

## Out of scope

- The English README itself (it is current and correct).
- CONTRIBUTING.md / AGENTS.md / glue-constant staleness — spec 039 (parallel, separate).
- Advertising Intel-Mac support in any README (the decision belongs to spec 035; do not claim a binary that isn't built).
- Any production code change.
