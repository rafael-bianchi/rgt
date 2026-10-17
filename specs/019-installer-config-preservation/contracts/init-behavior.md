# Contract: `rgt init` CLI Behavior (Installer Config Preservation)

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-27

## Invariants

1. `rgt init` MUST never delete, overwrite, or corrupt user-authored content in any agent config file it edits — with or without `--force`.
2. On any agent whose config cannot be safely parsed/merged, RGT MUST leave that file byte-for-byte unchanged, print an error naming the file and the problem, and (if a write was attempted) name the backup path.
3. Per-agent failures MUST NOT prevent healthy agents from being configured.
4. `rgt init` (no `--force`) on a fully configured agent MUST be a strict no-op (no file writes, no duplicate entries).

## Exit codes

| Code | Meaning | When |
|---|---|---|
| 0 | Success | All targeted agents configured or skipped; no failures. |
| 1 | Expected error | At least one agent failed (malformed/unmergeable config); report printed, healthy agents still configured. (Constitution: 1 = expected error.) |
| 2 | Invalid input | Unknown `--agent` name (existing behavior, unchanged). |

`main.rs` already maps `execute_init`'s `Err` to exit 1 and the `resolve_agent_arg` error to exit 2 — no new plumbing needed beyond returning an `Err` on any `Failed` outcome.

## Output shape

`rgt init` prints, in order:
1. The existing database-initialization line (unchanged).
2. A per-agent outcome list:
   - `✓ <Agent> (full hook|rules-file|plugin) -> <path>` for `Configured` (existing format).
   - `- <Agent> already configured (use --force to overwrite)` for `Skipped` (existing "no new ... written" messaging, per-agent).
   - `✗ <Agent>: <file>: <reason>` for `Failed`, followed by a recovery hint naming the backup if one exists.
3. If any agent failed, a summary line, e.g. `N agent(s) failed to configure — see errors above.` and the run returns exit 1.

## Acceptance linkage

- US1/SC-001 → invariants 1, 2 (byte-for-byte preservation).
- US2/FR-003, FR-008, FR-009 → invariants 2, 3 + exit codes 0/1.
- US3/FR-005, FR-006 → invariant 4 + `--force` block-replacement semantics (see `config-file-invariants.md`).
