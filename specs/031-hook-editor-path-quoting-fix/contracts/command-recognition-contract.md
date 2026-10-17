# Contract: Hook Command Recognition

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/unit/hook_editors.rs` and `tests/contract/hook_installer.rs`.

## I. Spaced-path recognition (FR-001, SC-001)

- `is_rgt_command("\"/Applications/My App/rgt\" hook post") == true`.
- `element_has_rgt_command` on a `hooks[].command` with a spaced quoted path == true.
- `is_rgt_command("\"/Applications/My App/rgt\" hook pre --agent claude-code") == true`.

## II. Existing forms preserved (FR-002, SC-003)

- `is_rgt_command("rgt hook post") == true`; `is_rgt_command("/usr/local/bin/rgt hook post") == true`.
- `is_rgt_command("\"C:\\Program Files\\rgt.exe\" hook pre") == true`.

## III. Bare non-RGT not misidentified (FR-003)

- `is_rgt_command("rtk hook post") == false` (bare-name discrimination preserved).
- Note: absolute paths pointing at `hook pre|post` remain classified as RGT-owned per the existing design (RGT writes absolute paths since spec 028); 031 does not change that classification.

## IV. Re-init idempotency (SC-002)

- Two successive installs with `injected_exe()` = a spaced path (e.g. `/Applications/My App/rgt`) produce exactly one RGT hook command entry in the target settings file — no duplicate.
- The second install is an idempotent update (report `Configured`/`Skipped`, never a second entry).

## V. Robustness (Constitution XI)

- A command with an unclosed/escaped quote does not panic; recognition falls back conservatively.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
