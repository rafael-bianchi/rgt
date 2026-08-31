//! MCP server mode (spec 041): exposes RGT's core operations as Model Context
//! Protocol tools over stdio via the official `rmcp` SDK. Additive to the 13
//! hook/plugin integrations — one documented integration surface instead of
//! per-agent, guessed wire formats.

pub mod error;

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use serde::Deserialize;

/// Runs the stdio MCP server to completion (blocks until the client closes
/// the stream). Every tool error returns to the client; the process never
/// exits from tool logic (spec 041 FR-005).
pub async fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let handler = RgtMcpServer;
    let (read, write) = rmcp::transport::io::stdio();
    let running = rmcp::serve_server(
        handler,
        rmcp::transport::async_rw::AsyncRwTransport::new(read, write),
    )
    .await?;
    // serve_server returns a handle immediately; the service runs in a spawned
    // background task, so wait for it to finish (stdin EOF / client close)
    // before returning, keeping the process alive for the whole session.
    running
        .waiting()
        .await
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    Ok(())
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct QueryParams {
    node_id: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct RecordParams {
    file: String,
    content: Option<String>,
    number_format: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct VerifyParams {
    parents: String,
    operation: String,
    expression: Option<String>,
    result: f64,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct DeriveParams {
    parents: String,
    operation: String,
    expression: Option<String>,
    result: f64,
}

/// The MCP tool surface: five tools mapped 1:1 to RGT's core operations
/// (spec 041 §3.1). `query_provenance` shipped first as the zero-refactor
/// prototype; the rest are backed by the refactored (no-`process::exit`)
/// functions, so any invalid input returns to the client as an error and never
/// terminates the long-lived server.
#[tool_router(server_handler)]
impl RgtMcpServer {
    /// Query the full provenance lineage for a tracked value.
    #[tool]
    async fn query_provenance(
        &self,
        Parameters(QueryParams { node_id }): Parameters<QueryParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        crate::query::query_provenance(&node_id)
            .map(Json)
            .map_err(|e| e.to_string())
    }

    /// Record numeric/date values from file content into the provenance graph.
    #[tool]
    async fn record_values(
        &self,
        Parameters(RecordParams {
            file,
            content,
            number_format,
        }): Parameters<RecordParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let fmt = match number_format.as_deref() {
            None => None,
            Some(raw) => Some(raw.parse().map_err(|e: String| e)?),
        };
        let outcome = crate::cli::record::execute_record_with_outcome(&file, content.is_some(), fmt)?;
        Ok(Json(serde_json::to_value(outcome).unwrap_or_default()))
    }

    /// Verify a derived value against its parent nodes WITHOUT recording it.
    #[tool]
    async fn verify_only(
        &self,
        Parameters(VerifyParams {
            parents,
            operation,
            expression,
            result,
        }): Parameters<VerifyParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        crate::verify::run_verify(&parents, &operation, expression.as_deref(), result)
            .map_err(|e| e.to_string())?;
        Ok(Json(serde_json::json!({ "verified": true })))
    }

    /// Verify a derived value against its parent nodes, then record it.
    #[tool]
    async fn derive_and_verify(
        &self,
        Parameters(DeriveParams {
            parents,
            operation,
            expression,
            result,
        }): Parameters<DeriveParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let node_id =
            crate::cli::derive::execute_derive_mcp(&parents, &operation, expression.as_deref(), result)
                .map_err(|e| e.to_string())?;
        Ok(Json(serde_json::json!({ "node_id": node_id })))
    }

    /// Report the provenance graph state (total/active/stale nodes).
    #[tool]
    async fn get_status(&self) -> Result<Json<serde_json::Value>, String> {
        let db = crate::store::DbStore::open_in_project(".")
            .map_err(|e| format!("failed to open database: {}", e))?;
        let conn = db.conn();
        let all = crate::store::queries::list_all_nodes(conn)
            .map_err(|e| format!("failed to list nodes: {}", e))?;
        let stale = crate::store::queries::list_stale_nodes(conn)
            .map_err(|e| format!("failed to list stale nodes: {}", e))?;
        Ok(Json(crate::cli::status::build_status_summary(&all, &stale)))
    }
}

/// The MCP server handler (tools-only; `server_handler` above emits the
/// `ServerHandler` impl).
struct RgtMcpServer;
