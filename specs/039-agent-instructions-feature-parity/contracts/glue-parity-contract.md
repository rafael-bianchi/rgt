# Contract: Glue-Instructions Feature Parity

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/contract/hook_installer.rs` (and grep checks).

## I. Feature-set coverage (FR-001, FR-002, SC-001)

For each of cline, antigravity, kilocode, vibe, copilot (CLI instructions), codex (AGENTS.md), and windsurf, the written artifact MUST mention:

- `rgt doctor` (or `doctor`)
- `rgt gc` (or `gc`)
- `--number-format`

## II. Existing content preserved (FR-005, SC-003)

- `rgt record <file>`, `rgt derive --parents`, `rgt status`, `rgt query`, `rgt graph` remain present in each artifact.
- The `with_instruction` non-text-source block is unchanged (still ends with `<!-- /RGT Integration -->`).

## III. CONTRIBUTING.md current (FR-004, SC-002)

- The `src/cli/` tree lists `doctor.rs` and `gc.rs`.
- Test examples reference real current test files (`hook_editors.rs`, `store_integrity.rs`, `test_update_attestation.rs`, `test_update_semver.rs`, `test_doctor_contract.rs`, `test_store_maintenance_contract.rs`).

## IV. Non-regression (Constitution IX, XI)

- Full `cargo test` green; `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
