# Data Model: Translated README Resync

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

No persisted data. The entity is the translated-document set and its parity with the English source.

## 1. TranslatedReadme

One of `README_{es,fr,ja,ko,pt,zh}.md`. Each must be a complete, structure-parity mapping of the English `README.md` (278 lines) into its language.

## 2. SectionParity (the completeness contract)

Every English section has a localized counterpart:

| English section | Required in every translation |
|---|---|
| What RGT Does | existing (preserved) |
| Why Provenance Tracking Matters | existing (preserved) |
| Installation → Quick Install | existing (preserved) |
| Installation → Homebrew | existing (preserved) |
| Installation → Cargo | existing (preserved) |
| Installation → Pre-built Binaries (+ "published with each release" note) | add note |
| Installation → Self-Update | existing (preserved) |
| Installation → Verify Installation | existing (preserved) |
| Quick Start | existing (preserved) |
| How It Works (+ non-text-source callout) | add callout |
| Commands → Init & Hooks (+ `--number-format`, config-preservation, absolute-path prose) | add |
| Commands → Doctor | add |
| Commands → Update (checksums + attestations + semver) | add |
| Commands → Provenance (+ `gc`, `--number-format`, expanded ops prose) | add |
| Supported AI Tools | existing (preserved) |
| Data & Storage | existing (preserved) |
| Documentation / Acknowledgments / Contributing / License | existing (preserved) |

## 3. AnchorSlug

The localized `#slug` used in the TOC/links, derived per-file from its own heading convention (preserving diacritics or CJK as that file already does).
