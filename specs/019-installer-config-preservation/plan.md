# Implementation Plan: Installer Config Preservation

**Branch**: `019-installer-config-preservation` | **Date**: 2026-08-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/019-installer-config-preservation/spec.md`

**Note**: This template is filled in by the `/speckit-plan` command; its definition describes the execution workflow.

## Summary

Make `rgt init` (any of the 13 agents, with or without `--force`) **never destroy, overwrite, or corrupt existing user configuration** (§2.1–§2.4 of the findings doc). Concretely:

1. **Merge, never replace**: Claude Code `settings.json`, Cursor `hooks.json`, and Copilot Chat VS Code settings get RGT's hook entries *merged additively* into the existing `hooks` structure — all non-RGT entries (other tools' hook groups, keybindings, settings) preserved byte-for-byte. Copied from `existing["hooks"] = rgt_hooks` (whole-key overwrite) and Cursor's whole-file overwrite (findings §2.2).
2. **JSONC tolerance**: JSON-family configs containing comments/trailing commas (valid in VS Code/Claude settings) are parsed tolerantly and rewritten with comments and untouched text preserved (clarified 2026-08-27; FR-010). Genuinely unparsable files are a **loud error** — never treated as `{}` and overwritten (findings §2.1).
3. **TOML-safe plugin edits**: Hermes `config.toml` `plugins.enabled` gets `"rgt"` appended via a format-preserving TOML editor that can never emit a duplicate `[plugins]` table or malformed TOML (findings §2.3).
4. **Paired-marker block replacement**: rules/instructions files (`AGENTS.md`, `.clinerules`, `.windsurfrules`, `.agents/rules/*`, `.kilocode/rules/*`, prompts) get an explicit end marker; `--force` replaces only the bytes between open and end markers, preserving user content below the block (findings §2.4) — including Codex's `AGENTS.md`, which today is overwritten wholesale on `--force`.
5. **Loud, recoverable failures**: unparsable/unmergeable files are reported by path with a recovery hint and produce a non-zero exit; per-agent failures do not abort the other agents (FR-003/FR-008). A backup copy is written before any write that modifies an existing file (FR-004).

## Technical Context

**Language/Version**: Rust (edition 2021, stable, as in `Cargo.toml`).

**Primary Dependencies**: Existing `serde_json`, `serde`. **New:** `jsonc-parser` (MIT, pure Rust; enable `cst` feature) for JSONC-tolerant, format-preserving editing of the three JSON-family config files; `toml_edit` (MIT OR Apache-2.0, pure Rust) for Hermes/Gemini/Vibe TOML configs. No other new crates — `serde_json` alone cannot parse JSONC or preserve comments/formatting on rewrite (verified in research; see `research.md`).

**Storage**: N/A — no schema/store changes; `.rgt/store.db` and the provenance graph are untouched. Only agent config files on disk are edited.

**Testing**: `cargo test` — new unit tests for the JSONC/TOML/text-block editors, extended contract tests (`tests/contract/hook_installer.rs`) for per-writer merge/preserve/error invariants, extended integration tests (`tests/integration/test_hooks_installer.rs`), plus `cargo fmt --all -- --check` and `cargo clippy --all-targets` per constitution IX/XI.

**Target Platform**: macOS, Linux, Windows (path resolution unchanged; line-ending handling for text blocks must be cross-platform-safe).

**Project Type**: CLI tool (Rust single binary) with agent-side hook config artifacts written at `rgt init`.

**Performance Goals**: `rgt init` stays sub-second for all 13 agents; each merge is O(file size) with no unbounded allocation.

**Constraints**:
- FR-001/SC-001: non-RGT content preserved byte-for-byte, with and without `--force` (JSONC editor must splice text, not re-serialize).
- FR-003/FR-009: hard-error with file path + reason + recovery hint; never `unwrap_or_default()`-style silent fallback to empty.
- FR-004/SC-006: backup copy before any write that modifies an existing file (additive merges included); backups never auto-deleted.
- FR-005: `rgt init` (no `--force`) stays a strict no-op when already configured (no duplication).
- FR-006: `--force` replaces only RGT's own delimited block; user content below it is preserved.
- FR-007: output is always valid JSON/TOML/text; never duplicate TOML tables or unbalanced JSON.
- FR-008: per-agent outcomes (configured/skipped/failed+reason) reported; overall non-zero exit if any failure; healthy agents still configured.
- FR-010: JSONC (comments/trailing commas) tolerated and comments preserved in rewritten files.

**Scale/Scope**: Rework of the config writers in `src/hooks/installer.rs` (Claude Code, Cursor, Codex, Windsurf, Copilot, Gemini, Vibe, Hermes, Cline, Antigravity, Kilo), a new format-preserving editor module, an error-model change for `configure()` (per-agent results instead of `?` short-circuit), `rgt init` output/exit-code plumbing, 2 new dependencies, tests, docs. Plugin-tier files (OpenCode/Pi/Hermes `rgt.ts`/`plugin.py`) are wholly RGT-owned and untouched by this feature.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|---|---|---|
| I. Rust-Only Single Binary | ✅ PASS | Both new crates (`jsonc-parser`, `toml_edit`) are pure Rust, compile into the single binary, add no runtime engines. |
| II. Two-Tier Detection & BLAKE3 | ✅ PASS | Unaffected — change-detection untouched. |
| III. Graph Correctness | ✅ PASS | Unaffected — store/graph untouched. |
| IV. Rich Value Types | ✅ PASS | Unaffected. |
| V. Multi-Channel Distribution | ✅ PASS | `rgt init` behavior surface preserved; distribution unchanged. |
| VI. Dual Passive/Active CLI-First | ✅ PASS | Installer edits config only; hook runtime and CLI commands unchanged. |
| VII. Permissive OSS License | ✅ PASS | `jsonc-parser` is MIT, `toml_edit` is MIT OR Apache-2.0 — both compatible with RGT's Apache-2.0. |
| VIII. CLI-First Hook Architecture | ✅ PASS | Unaffected — hook glue unchanged. |
| IX. Test-Driven Verification | ✅ PASS | Editor unit tests + contract tests for merge/preserve/error invariants; existing 018-era byte-identical assertions updated to the new merge expectations. |
| X. Documentation Matches Behavior | ✅ PASS | README/`--help`/AGENTS.md updated for the new `rgt init` guarantees (preservation, JSONC, backups, exit codes). |
| XI. Code Quality & Idiomatic Rust | ✅ PASS | New deps justified (serde_json insufficient for JSONC + byte-for-byte preservation — see `research.md`); fmt/clippy enforced in CI; errors are user-recoverable (file path + fix). |

**Gate Result**: PASS — 11/11 principles satisfied; no amendments required.

## Project Structure

### Documentation (this feature)

```text
specs/019-installer-config-preservation/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (crate + merge-strategy research)
├── data-model.md        # Phase 1 output (installer domain model)
├── quickstart.md        # Phase 1 output (validation guide)
├── contracts/           # Phase 1 output (init CLI + config-file invariants)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
src/
├── main.rs              # MODIFIED: init failure path already exits 1; error message wording
├── cli/
│   └── init.rs          # MODIFIED: print per-agent report (configured/skipped/failed);
│                        #          return Err (exit 1) if any agent failed (FR-008)
└── hooks/
    ├── installer.rs     # MODIFIED: per-agent result model for configure(); backup helper;
    │                    #          merge-based writers (claude-code, cursor, copilot, gemini,
    │                    #          vibe, hermes); paired-marker text blocks (codex, windsurf,
    │                    #          cline, antigravity, kilocode, copilot-cli, vibe-prompt)
    ├── editor.rs        # NEW: format-preserving config editors
    │   │                #      - jsonc_merge_hook_entries(text, key, rgt_entries) -> text
    │   │                #      - toml_ensure_array_value(text, path, value) -> text
    │   │                #      - text_replace_block(text, open_marker, end_marker, block) -> text
    │   └── ...          #      (all return Result; unparsable input => Err with path context)
    └── glue/mod.rs      # MODIFIED: RGT_END_MARKER const; every block constant gains an end-marker line

tests/
├── unit/hook_editors.rs             # NEW: JSONC/TOML/text-block editor unit tests (byte-for-byte
│                                    #      preservation, JSONC tolerance, malformed-input errors)
├── contract/hook_installer.rs       # MODIFIED: per-writer merge/preserve/error/exit invariants
└── integration/test_hooks_installer.rs  # MODIFIED: end-to-end init runs against temp homes
```

**Structure Decision**: Single-project layout (existing Rust CLI). The new `src/hooks/editor.rs` owns all "edit an existing user file safely" logic (JSONC, TOML, text-block) so the writers in `installer.rs` stay thin and the preservation guarantees are testable in isolation — mirroring the existing `hooks::paths`/`glue` module pattern.

## Complexity Tracking

None — no constitution violations; the two new dependencies are justified under Principle XI in the Constitution Check above (see `research.md` for the rejected alternatives).
