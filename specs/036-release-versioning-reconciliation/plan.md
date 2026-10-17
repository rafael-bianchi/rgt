# Implementation Plan: Release Versioning Reconciliation

**Branch**: `feat/036-release-versioning-reconciliation` | **Date**: 2026-08-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/036-release-versioning-reconciliation/spec.md`

## Summary

Reconcile the version-bearing state of the repo to 0.5.0 (which `Cargo.toml`, the `v0.5.0` tag, and the live GitHub release already use): fix `.release-please-manifest.json` (set the operative `.` key to 0.5.0, remove the stale `rgt` key), backfill `CHANGELOG.md` with a 0.5.0 entry covering specs 019–029, and bump `Formula/rgt.rb` to 0.5.0 with the real v0.5.0 `checksums.txt` SHAs. Zero production-code changes.

## Technical Context

**Language/Version**: N/A (state/config files only — JSON manifest, Markdown changelog, Ruby formula).

**Primary Dependencies**: None.

**Storage**: N/A.

**Testing**: A consistency-check script (shell or a small Rust test) asserting the version-bearing files agree; SHA verification against the live v0.5.0 `checksums.txt`.

**Target Platform**: N/A (repo state).

**Project Type**: CLI distribution metadata.

**Performance Goals**: N/A.

**Constraints**: Constitution X (docs match reality — this fixes exactly that). No `src/` changes. Intel-Mac branch deferred to spec 035.

**Scale/Scope**: `.release-please-manifest.json`, `CHANGELOG.md`, `Formula/rgt.rb`, plus a verification script/test.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-only, zero runtime deps | PASS | State-file change; no deps. |
| II. Two-tier detection + BLAKE3 | PASS | Unaffected. |
| III. Graph correctness | PASS | Unaffected. |
| IV. Rich value types | PASS | Unaffected. |
| V. Distribution / RTK UX | PASS | Homebrew formula corrected — direct distribution-surface fix. |
| VI. Dual passive/active surface | PASS | Unaffected. |
| VII. Open-source governance | PASS | Unaffected. |
| VIII. Passive hooks, thin delegates | PASS | Unaffected. |
| IX. Test-driven verification | PASS | Consistency-check script + SHA verification. |
| X. Documentation matches behavior | PASS | Changelog backfill + manifest/formula sync restore doc-reality alignment. |
| XI. Idiomatic Rust, exit codes | PASS | No code change. |

**Post-design re-check**: No violations.

## Project Structure

### Documentation (this feature)

```text
specs/036-release-versioning-reconciliation/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── version-state-contract.md
└── tasks.md             # Phase 2 output (/speckit-tasks)
```

### Source Code (repository root)

```text
.release-please-manifest.json   # reconciled (0.5.0, no rgt key)
CHANGELOG.md                     # 0.5.0 entry added
Formula/rgt.rb                   # version + SHAs bumped
scripts/check-version-state.sh   # NEW: consistency + SHA verification
```

**Structure Decision**: A single small shell script (`scripts/check-version-state.sh`) is the verification artifact (matches the existing `scripts/` presence), runnable in CI or locally. It checks the four version fields agree and diffs the formula SHAs against `checksums.txt` when `RGT_CHECKSUMS_URL` is provided (offline-safe: the SHA check is skipped with a warning when the URL is unreachable, so `cargo test` never depends on the network — Constitution IX).

## Complexity Tracking

> None — no constitution violations requiring justification.
