# Contract: MCP Tool Surface & CLI Exit Codes

**Feature**: [spec.md](../spec.md) | **Date**: 2026-08-31

Executable invariants verified by `tests/contract/test_mcp_contract.rs` and `tests/contract/test_cli_record_derive_contract.rs`.

## I. Server lifecycle (FR-001, FR-005)

- `rgt mcp` starts a stdio MCP server that survives `tools/call` errors: after any tool error, a subsequent `tools/list` still succeeds.
- `initialize` and `tools/list` succeed against a fresh store.

## II. query_provenance (FR-007)

- `tools/call query_provenance(node_id)` returns the seeded lineage JSON matching `rgt query <id> --json`.
- An unknown node ID returns a structured tool error (no crash, no process exit).

## III. Refactored functions (FR-002, FR-003, FR-004)

- `rg "process::exit" src/cli/derive.rs src/verify/mod.rs` → no matches.
- `record_values` returns `RecordOutcome`; `get_status` returns the summary value.
- `verify_only` never writes to the graph; `derive_and_verify` verifies-then-records.

## IV. CLI exit-code preservation (FR-006, SC-003)

For `rgt derive` and `rgt verify`:
- unknown `--operation` → exit 2
- missing `--expression` for EXPRESSION → exit 2
- empty / >26 parents → exit 2
- missing parent node → exit 2
- verification failure → exit 1

Each asserted explicitly (not just "suite still passes").

## V. Non-regression (Constitution IX, XI)

- Full `cargo test` green; `cargo fmt --all -- --check` and `cargo clippy --all-targets` pass.
- Release binary size delta from `rmcp` measured and reported.
