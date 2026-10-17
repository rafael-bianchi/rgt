# Data Model: MCP Server Mode

**Feature**: [spec.md](./spec.md) | **Date**: 2026-08-31

## 1. McpTool

One of the five exposed MCP tools, mapping 1:1 to a refactored pure function:

| Tool | Input | Output |
|------|-------|--------|
| `query_provenance` | `node_id: String` | lineage JSON (`node_id`, `is_stale`, `total_ancestors`, `lineage_steps`) |
| `get_status` | — | graph-state summary JSON (total/active/stale nodes + stale details) |
| `record_values` | `file: String`, `content: Option<String>`, `number_format: Option<String>` | `RecordOutcome { count, skipped_limit, skipped_malformed }` |
| `verify_only` | `parents: String`, `operation: String`, `expression: Option<String>`, `result: f64` | `Ok(())` or `VerifyDeriveError` (never writes) |
| `derive_and_verify` | same as verify_only | derived node id (on success) or `VerifyDeriveError` |

## 2. VerifyDeriveError

`InvalidInput(String)` → CLI exit 2; `VerificationFailed(String)` → exit 1; `Internal(String)` → exit 1. Separates "bad input" from "math didn't check out."

## 3. RecordOutcome

`{ count: usize, skipped_limit: bool, skipped_malformed: usize }` — the structured result of `record_values`, replacing print-only reporting.

## 4. StatusSummary

The existing `json!({...})` graph-state value (status.rs:57-62), returned as a value instead of `println!`-only.

## 5. RgtMcpServer

The `#[tool]`-annotated handler struct implementing `rmcp::ServerHandler`, served over stdio (`transport-io`). Long-lived: tool errors return to the client; the process never exits from tool logic.
