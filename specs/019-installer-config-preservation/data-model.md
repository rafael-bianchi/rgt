# Data Model: Installer Config Preservation

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-27

The domain model for the installer's config-writing surface. Existing entities (AgentDef, Tier, DetectedTool) are unchanged; the new/changed entities below carry this feature's guarantees.

## Entities

### AgentConfigFile

A user-owned file the installer may edit.

| Attribute | Type | Notes |
|---|---|---|
| `path` | `PathBuf` | Absolute or project-relative path on disk. |
| `kind` | `ConfigKind` | `Json` (Claude Code `settings.json`, Cursor `hooks.json`, Copilot Chat VS Code settings), `Toml` (Hermes `config.toml`, Gemini `hooks.toml`, Vibe `hooks.toml`), `Text` (rules/instructions/prompt files). |
| `existing_content` | `String` | Read before any edit (the byte-exact baseline for preservation). |
| `edited_content` | `String` | Output of a successful merge; differs from `existing_content` only in RGT's own regions. |

Invariants:
- `existing_content` → `edited_content` changes MUST be limited to RGT-managed regions (SC-001/SC-004). For JSON the edit is a text splice; for TOML a document edit; for text a delimited-block replacement.
- No write happens if the file is unparsable/unmergeable; the file is left byte-identical (FR-003).

### RgtManagedEntry

A hook entry, plugin reference, or block region that RGT created and can identify as its own.

| Attribute | Type | Notes |
|---|---|---|
| `kind` | `EntryKind` | `HookCommand` (command string starting with `rgt hook`), `PluginRef` (`"rgt"` in `plugins.enabled`, or an RGT plugin file path), `Block` (delimited text region). |
| `signature` | `String` | The identifier used to recognize an existing RGT entry during a merge: the `command` value for JSON hook entries, the literal `"rgt"` for the TOML array, the marker pair for text blocks. |

Identification rules (used by the merge):
- JSON hook group/entry: matched when its `command` field starts with `rgt hook` (findings §2.2: "append/replace only entries whose command starts with `rgt hook`").
- TOML: `plugins.enabled` contains `"rgt"`; Gemini `PostToolUse.command` contains `rgt hook post --agent gemini`; Vibe `pre_tool` table has `command` containing `rgt hook post --agent vibe`.
- Text: the open marker line + matching end marker line.

### RgtBlock (text files)

A delimited section in a text/rules file.

| Attribute | Type | Notes |
|---|---|---|
| `open_marker` | `String` | `glue::RGT_MARKER` (`## RGT Integration`, or `# RGT Integration` per block constant). |
| `end_marker` | `String` | New `glue::RGT_END_MARKER` (`<!-- /RGT Integration -->`), appended to every block written by this feature. |
| `content` | `String` | The RGT instructions between the markers. |

State transitions:
- `Absent` (no open marker) → `Present` (write block at end of file).
- `Present` (open + end marker) + `--force` → `Refreshed` (replace bytes strictly between markers; preserve everything outside).
- `Legacy` (open marker, no end marker) + `--force` → `Error` (refuse; loud message with manual-remediation hint; file untouched).
- `Present` + no `--force` → `Unchanged` (idempotent no-op).

### BackupCopy

A recoverable snapshot taken before any write that modifies an existing file.

| Attribute | Type | Notes |
|---|---|---|
| `original_path` | `PathBuf` | The config file being modified. |
| `backup_path` | `PathBuf` | `original_path` + `.rgt.bak`. |
| `written` | `bool` | Set only when the original file existed and is about to be modified. |

Invariants:
- Written before every write that modifies an existing file (additive merges included) (FR-004), never auto-deleted (spec assumption; SC-006).
- If the writer fails after backup, the error message MUST name the backup path so the user can restore (FR-009).

### InstallReport

The per-agent outcome set produced by `rgt init`.

| Attribute | Type | Notes |
|---|---|---|
| `outcomes` | `Vec<AgentOutcome>` | One per targeted agent. |
| `exit_non_zero` | `bool` | True iff any `AgentOutcome` is `Failed`. |

`AgentOutcome` variants:
- `Configured { agent: String, artifact: PathBuf }` — RGT entries written/updated; printed as today's "-> path" line.
- `Skipped { agent: String }` — idempotent no-op (already configured, no `--force`).
- `Failed { agent: String, reason: String }` — file untouched or backed up; reason names the file and the problem and, where applicable, the backup path (FR-003/FR-009).

## Validation rules (from FR-001..FR-010)

- **Preservation**: diff of `existing_content` vs `edited_content` touches only RGT-managed regions; for JSON/TOML/text this is asserted byte-level in editor unit tests (SC-001/SC-004).
- **No silent empty fallback**: an unparsable file never produces an "empty-object" edit (FR-003).
- **JSONC**: files parse tolerantly (comments + trailing commas); rewritten JSON retains its comments (FR-010).
- **Valid output**: every `edited_content` re-parses with the target format; TOML never contains duplicate `[plugins]` (FR-007).
- **Idempotency**: `rgt init` without `--force` on a fully configured agent produces `Skipped` and no disk write (FR-005).
- **Isolation**: one agent's `Failed` never prevents other agents from being `Configured`, and the overall run exits non-zero (FR-008).
- **Backup**: every destructive write is preceded by `BackupCopy` (FR-004).

## Out of scope

OpenCode/Pi/Hermes plugin files (`rgt.ts`, `plugin.py`) — wholly RGT-owned, written via `write_plugin_file` unchanged. `.rgt/store.db` and the provenance graph are untouched.
