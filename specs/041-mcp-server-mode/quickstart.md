# Quickstart: MCP Server Mode

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

Runnable end-to-end validation. See [contracts/mcp-tool-contract.md](./contracts/mcp-tool-contract.md) for the authoritative invariants.

## Prerequisites

- Built binary: `cargo build`

## Scenario 1 — an MCP client handshake and query

```bash
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | target/debug/rgt mcp
```

Expected: an `initialize` result and a `tools/list` result naming `query_provenance`, `get_status`, `record_values`, `verify_only`, `derive_and_verify`.

## Scenario 2 — server survives an invalid tool call

Seed a store, then call `derive_and_verify` with an unknown operation; follow with another `tools/list`. Expected: the tool returns an error result and the following `tools/list` still succeeds (the server did not exit).

## Scenario 3 — CLI exit codes unchanged

```bash
target/debug/rgt derive --parents x --operation bogus --result 1; echo "exit=$?"   # 2
target/debug/rgt verify --parents x --operation EXPRESSION --result 1; echo "exit=$?"  # 2 (missing expression)
```

## Automated checks

```bash
cargo test --test test_mcp_contract
cargo test --test test_cli_record_derive_contract
cargo fmt --all -- --check
cargo clippy --all-targets
cargo test
```
