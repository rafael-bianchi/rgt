# Research: MCP Server Mode

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Decisions grounded in `docs/spec-041-mcp-server-mode.md` (the deep spec, read in full), the current source of the five target functions, and direct verification of `rmcp` 3.1.4 on crates.io/docs.rs.

## 1. Tool mapping and refactor scope (FR-001, FR-002, FR-007)

**Confirmed against current source** (`src/query/mod.rs:10-69`, `src/cli/status.rs:26-92`, `src/cli/record.rs:42-148`, `src/cli/derive.rs:8-134`, `src/verify/mod.rs:55-129`):

| MCP tool | Wraps | Refactor |
|---|---|---|
| `query_provenance` | `query::query_provenance(&str) -> Result<Value, String>` | **None** — pure, JSON-returning. Ships first (deep spec Risks: de-risking prototype). |
| `get_status` | `status::execute_status_with` | Extract the `json!({...})` at status.rs:57-62 into a returned value; CLI `println!` becomes a one-line wrapper. |
| `record_values` | `record::execute_record` | Introduce `RecordOutcome { count, skipped_limit, skipped_malformed }`; return it; CLI prints from it. |
| `verify_only` | `verify::run_verify_cli` | **Required**: 7 `process::exit` sites (verify/mod.rs:64,70,74,86,93,104,108,115,126) → pure `run_verify(...) -> Result<VerifyOutcome, VerifyDeriveError>`; CLI wrapper translates to exit codes. |
| `derive_and_verify` | `derive::execute_derive` | **Required**: 8 `process::exit(2)` sites (derive.rs:22,28,32,44,52,65,70,82) + exit(1) at :99 → typed `Err`; CLI wrapper translates. |

## 2. Error typing (FR-003)

**Decision**:

```rust
pub enum VerifyDeriveError {
    InvalidInput(String),      // exit 2
    VerificationFailed(String),// exit 1
    Internal(String),          // exit 1
}
```

The existing CLI flow is: `main.rs:236-241` forwards `execute_derive`'s `Result` to the generic `eprintln!` + `exit(1)` handler — but exit-2 is only distinguished today via direct `process::exit(2)`. So `main.rs`'s Derive arm must inspect the error type to decide 2 vs 1. The Verify arm currently calls `run_verify_cli` (returns `()`, exits internally); it becomes `run_verify(...)?` mapped by a wrapper that exits 0/1/2 explicitly.

**Rationale**: The deep spec §3.3 calls this "genuinely worthwhile cleanup independent of MCP — it directly fixes the 'function signature looks clean but actually exits the process' trap."

## 3. rmcp integration shape (FR-001)

**Verified on crates.io**: `rmcp = "3.1.4"` with default features `server`+`macros`, plus opt-in `transport-io` (stdio). The `#[tool]` attribute macro defines tools on an `impl` block implementing `ServerHandler`. The deep spec §3.2's illustrative shape is the target; its exact ergonomics are validated by the prototype task (T002) before the refactor, per the deep spec Risks table.

**Decision**: stdio transport only (`transport-io`); the server runs via `rmcp::serve_stdio(handler)`. Every one of RGT's 13 existing integrations already invokes it as a subprocess, so stdio matches the established model and introduces no new network-facing surface.

## 4. Server resilience (FR-005, FR-006)

**Decision**: All tool bodies call the refactored pure functions and map errors into the MCP error response. No tool body calls `process::exit` or panics. The `#[tool]` macro's async handlers return `Result<..., String>` per the documented pattern; a panic would be caught by the SDK transport, but the refactor eliminates the panic-inducing paths (garbage input to validation is already handled by `validate_expression` returning `Err`, not panicking — verified in `src/verify/expression.rs`).

## 5. Test strategy (SC-001..SC-005)

**Decision**: `tests/contract/test_mcp_contract.rs` drives the actual binary (`rgt mcp`) over stdio: write `initialize`, `tools/list`, `tools/call` JSON-RPC messages; read responses; assert `query_provenance` returns the seeded lineage and an invalid `derive_and_verify` returns an error while the server survives (second `tools/list` succeeds). Explicit exit-code assertions are added to `tests/contract/test_cli_record_derive_contract.rs` for every refactored call site (unknown op, missing expression, empty/too-many parents, missing parent node, verification failure) — the existing suite already covers several; the refactor adds the missing ones.

**Rationale**: Constitution IX; the deep spec Risks table explicitly requires "explicit exit-code assertions for every one of the 8 (derive) + 7 (verify) call sites ... not just a general 'does it still work' smoke test."

## 6. Binary-size measurement (SC-004)

**Decision**: Measure release-build `target/release/rgt` size before and after adding `rmcp` (the repo's `[profile.release]` is already lto+strip-optimized), and report the delta — per the deep spec Risks table.
