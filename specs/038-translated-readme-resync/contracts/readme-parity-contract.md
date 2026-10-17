# Contract: Translated README Parity

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `scripts/check-readme-parity.sh`.

## I. Feature-set coverage (FR-001, SC-001, SC-002)

For each of `README_{es,fr,ja,ko,pt,zh}.md`, the localized equivalents of the following MUST be present (heading or term):

- Doctor section
- Update section (checksums + attestation + semver no-downgrade)
- `gc` in the Provenance command list
- `--number-format` in init/record
- non-text-source capture note

## II. Preservation (FR-002)

- Pre-existing translated passages are unchanged (no gratuitous re-translation).

## III. Anchors & language bar (FR-003, SC-003)

- Each file's TOC anchors match its own localized headings.
- The language-switcher bar in all 7 README files references all 7 variants.

## IV. Markdown integrity (FR-004, SC-004)

- Balanced fenced code blocks in each file.
- Tables parse (consistent column counts per row).

## V. No scope creep

- No Intel-Mac binary claims in any README (spec 035 owns that).
- `cargo test` stays green (no `src/` changes).
