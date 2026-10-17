# Research: Translated README Resync

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in a structural diff of the English README (278 lines) against all six translations (each 221 lines), plus a section-by-section heading survey.

## 1. The gap (FR-001)

**Finding**: The English README gained sections/content since the translations were frozen at the 0.4.0-era snapshot. Every translation is missing the same set:

| English content (README.md) | Missing in all 6 translations |
|---|---|
| `### Doctor` section (lines 184-195) | Yes |
| `### Update` section with checksums+attestations+semver prose (lines 197-210) | Yes (they only have the brief "Mise à jour automatique"/"自我更新" etc. self-update subsection under Installation) |
| `rgt gc [--vacuum]` in the Provenance Commands block (line 222) | Yes |
| `--number-format` in `rgt init` line (line 162), `rgt record` line (line 214), and the locale prose (lines 225-226) | Yes |
| The non-text-source callout in "How It Works" (line 154) | Yes |
| The "rgt init never destroys your existing configuration" paragraph (lines 168-182) incl. absolute-path note | Yes |
| The expanded Operations/case-sensitivity/DATE_DIFF prose (lines 229-231) | Partially — translations have the basic Operations line only |
| Pre-built binaries note "published with each release" (line 100) | Missing (they have the subsection header + list but not the note) |
| `verify installation` (line 112) | They have the subsection but the content is the pre-hardening text |

The `Doctor`, `Update`, `gc`, `--number-format`, attestation, non-text-source, and config-preservation content are entirely absent from all six files (grep-verified: zero hits for `doctor`, `gc`, `number-format`, `attestation`).

## 2. Surgical reconcile vs. re-translation (FR-002)

**Decision**: Preserve every already-correct translated passage; add only the missing English sections, translated into each file's established register and terminology. Each translation already has a consistent voice (e.g. French uses "hook"/"grafo" consistently; Japanese uses "来歴"/"ノード"; etc.) — the additions must match that voice.

**Rationale**: The existing translations were verified correct against the pre-hardening English snapshot (the 221-line versions match it); re-translating them wholesale would discard that verified work for no benefit (FR-002, the spec's explicit requirement).

**Alternatives considered**: Machine re-translation of the whole file — rejected: risks regressing already-correct passages and mixing terminology.

## 3. Anchors and TOC (FR-003)

**Finding**: Each translation's TOC uses localized anchor slugs (e.g. French `#binaires-précompilés`, Japanese `#コマンド`, etc.) that match its own headings. The English TOC slugs must NOT be copied verbatim.

**Decision**: For any new section, derive its anchor from the translated heading using the same slug convention the file already uses for similar headings (lowercased, diacritics preserved or stripped consistently per-file, CJK headings left as-is as the existing files do). The language-switcher bar (all 7 files, top of each) already lists all 7 variants and needs no change (verify only).

## 4. Structural verification (SC-001..SC-004, Constitution IX)

**Decision**: A lightweight check script (`scripts/check-readme-parity.sh`) that:
1. Asserts each translated README contains localized markers for Doctor, Update, `gc`, `--number-format`, and attestation (section-heading or term grep per language).
2. Asserts the language-switcher bar in all 7 files contains all 7 variant links.
3. Asserts balanced fenced code blocks in each file (count of ``` per file is even) and that each table row has a consistent column count (sanity).
4. Is deterministic and offline (no network).

The full `cargo test` must stay green (no `src/` changes), run as a regression check.

## 5. Out-of-scope guard (Constitution X)

**Decision**: Do not add Intel-Mac binary claims to any README — the release currently ships no `x86_64-apple-darwin` in the build matrix (that's spec 035's decision). The pre-built-binaries list stays as the existing arch list (aarch64 macOS, x86_64/aarch64 Linux, x86_64 Windows).
