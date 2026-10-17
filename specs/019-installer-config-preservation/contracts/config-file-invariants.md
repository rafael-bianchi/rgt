# Contract: Config-File Merge Invariants (Installer Config Preservation)

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-27

Guarantees per config-file kind, referenced by editor unit tests and installer contract tests. "Preserved byte-for-byte" means the bytes of `existing_content` outside RGT-managed regions are verbatim in `edited_content`.

## JSON-family files (Claude Code `settings.json`, Cursor `hooks.json`, Copilot Chat VS Code settings)

| Invariant | Detail |
|---|---|
| **JSONC tolerance** | Files with comments and trailing commas parse successfully (FR-010); genuinely unparsable JSON is an error (file untouched). |
| **Additive merge** | RGT's hook entries are added into the existing `hooks` structure; all other hook groups, keys, and settings are preserved byte-for-byte (FR-001/FR-002). |
| **Replace own entries** | Pre-existing RGT entries (command starting with `rgt hook`) are replaced in place by fresh ones; never duplicated. |
| **Comment preservation** | Comments in the file survive the rewrite (FR-010, SC-005). |
| **No whole-key replacement** | RGT never assigns `existing["hooks"] = <rgt-only>`; the `hooks` subtree keeps every non-RGT group (fixes §2.2). |
| **No whole-file replacement** | Cursor `hooks.json` is merged, never rewritten from scratch (fixes §2.2). |
| **Unexpected schema shape** | A valid-JSON file whose `hooks` subtree has an unexpected shape (e.g. object instead of array, or the target section missing) is treated as unmergeable → error, file untouched (FR-003/FR-007). |

### Per-agent hook shape

| Agent | Structure edited | RGT entry |
|---|---|---|
| Claude Code | `hooks.PostToolUse[]` / `hooks.PreToolUse[]` of `{matcher, hooks:[{type:command, command}]}` | `{matcher:"", hooks:[{type:"command", command:"rgt hook post"}]}` (post) / `rgt hook pre` (pre) |
| Cursor | `hooks.postToolUse[]` / `hooks.preToolUse[]` of `{command, matcher}` | `{command:"rgt hook pre", matcher:"Shell"}` / `rgt hook post` |
| Copilot Chat | `github.copilot.chat.hooks.PostToolUse[]` of `{matcher, hooks:[{type:command, command}]}` | `{matcher:".*", hooks:[{type:"command", command:"rgt hook post --agent copilot"}]}` |

## TOML files (Hermes `config.toml`, Gemini `hooks.toml`, Vibe `hooks.toml`)

| Invariant | Detail |
|---|---|
| **Format preservation** | Comments, spacing, and relative order preserved via document editing (findings §2.3). |
| **No duplicate tables** | `[plugins]` is created only if absent (`or_insert`); never a second `[plugins]` (fixes §2.3 invalid-TOML). |
| **Additive array** | `"rgt"` appended to `plugins.enabled` only if absent. |
| **Malformed TOML is an error** | Unparsable TOML → file untouched, error named, non-zero exit (FR-003). |

### Per-agent TOML shape

| Agent | Structure edited | RGT entry |
|---|---|---|
| Hermes | `plugins.enabled` array | ensure `"rgt"` ∈ `plugins.enabled` |
| Gemini | `[PostToolUse]` table | ensure `command` contains `rgt hook post --agent gemini`; if the table exists with a conflicting user command, treat as unmergeable → error (cannot duplicate the table) |
| Vibe | `[[pre_tool]]` array of tables | ensure a table with `command` containing `rgt hook post --agent vibe` exists; append if absent |

## Text/rules files (AGENTS.md, `.clinerules`, `.windsurfrules`, `.agents/rules/*`, `.kilocode/rules/*`, prompts)

| Invariant | Detail |
|---|---|
| **Paired markers** | Every block written is delimited by `open_marker` (`glue::RGT_MARKER`) and `end_marker` (`glue::RGT_END_MARKER` = `<!-- /RGT Integration -->`). |
| **`--force` replaces only the block** | Content above the open marker and below the end marker is preserved byte-for-byte (FR-006; fixes §2.4). |
| **No `--force` → no-op** | Open marker present → `Skipped`, no write (FR-005). |
| **Legacy block safety** | Open marker present but no end marker → `Failed` with a manual-remediation message; file untouched (never guesses a boundary). |
| **Codex AGENTS.md** | RGT's section is merged/replaced; the whole file is never overwritten on `--force` (fixes §2.4 sibling bug). |

## Cross-cutting

- **Backup** (FR-004): any write that modifies an existing file is preceded by a `<path>.rgt.bak` copy; errors name the backup path.
- **Valid output** (FR-007): every `edited_content` re-parses in its target format.
- **Idempotency** (FR-005): second run without `--force` writes nothing.
