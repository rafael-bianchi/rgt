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
    unit: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct RecordParams {
    file: String,
    content: Option<String>,
    number_format: Option<String>,
}

fn deserialize_explicit_value<'de, D>(
    deserializer: D,
) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(Some)
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct VerifyParams {
    parents: String,
    operation: String,
    expression: Option<String>,
    #[serde(default, deserialize_with = "deserialize_explicit_value")]
    result: Option<serde_json::Value>,
    unit: Option<String>,
    result_unit: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct DeriveParams {
    parents: String,
    operation: String,
    expression: Option<String>,
    #[serde(default, deserialize_with = "deserialize_explicit_value")]
    result: Option<serde_json::Value>,
    unit: Option<String>,
    result_unit: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
struct ExportGraphParams {
    format: Option<String>,
    include_absolute_paths: Option<bool>,
}

fn parse_mcp_result(result: Option<serde_json::Value>) -> Result<Option<String>, String> {
    match result {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Null) => {
            Err("invalid result: null is not allowed (must be string or number)".to_string())
        }
        Some(serde_json::Value::Bool(_)) => {
            Err("invalid result: boolean is not allowed (must be string or number)".to_string())
        }
        Some(v) => Err(format!(
            "invalid result: expected string or number, got {v}"
        )),
    }
}

/// The MCP tool surface: mapped to RGT's core operations.
#[tool_router(server_handler)]
impl RgtMcpServer {
    /// Query the full provenance lineage for a tracked value.
    #[tool]
    async fn query_provenance(
        &self,
        Parameters(QueryParams { node_id, unit }): Parameters<QueryParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let parsed_unit = unit
            .as_deref()
            .map(crate::types::DurationUnit::parse)
            .transpose()
            .map_err(|e| e.to_string())?;
        crate::query::query_provenance_with_unit(&node_id, parsed_unit)
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
        let outcome = crate::cli::record::execute_record_with_content(&file, false, content, fmt)?;
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
            unit,
            result_unit,
        }): Parameters<VerifyParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let result_str = parse_mcp_result(result)?;
        let claim_unit = if result_str.is_some() {
            result_unit.as_deref().or(unit.as_deref())
        } else {
            result_unit.as_deref()
        };
        let display = crate::verify::run_verify_core(
            &parents,
            &operation,
            expression.as_deref(),
            result_str.as_deref(),
            claim_unit,
            unit.as_deref(),
        )
        .map_err(|e| e.to_string())?;

        let mut resp = serde_json::json!({ "verified": true });
        if let Some(d) = display {
            resp["display"] = serde_json::json!(d.text);
            resp["display_details"] = serde_json::to_value(&d).unwrap_or_default();
        }
        Ok(Json(resp))
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
            unit,
            result_unit,
        }): Parameters<DeriveParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let result_str = parse_mcp_result(result)?;
        let claim_unit = if result_str.is_some() {
            result_unit.as_deref().or(unit.as_deref())
        } else {
            result_unit.as_deref()
        };

        let (node_id, display) = crate::cli::derive::execute_derive_core(
            &parents,
            &operation,
            expression.as_deref(),
            result_str.as_deref(),
            claim_unit,
            unit.as_deref(),
        )
        .map_err(|e| e.to_string())?;

        let mut res = serde_json::json!({ "node_id": node_id });
        if let Some(d) = display {
            res["display"] = serde_json::json!(d.text);
            res["display_details"] = serde_json::to_value(&d).unwrap_or_default();
        }
        Ok(Json(res))
    }

    /// Report the provenance graph state (total/active/stale nodes).
    #[tool]
    async fn get_status(&self) -> Result<Json<serde_json::Value>, String> {
        let mut db = match crate::store::DbStore::open_existing_in_project(".")
            .map_err(|e| format!("failed to open database: {}", e))?
        {
            Some(db) => db,
            None => {
                return Ok(Json(crate::cli::status::build_status_summary(&[], &[])));
            }
        };
        crate::graph::invalidation::sync_staleness(&mut db)
            .map_err(|e| format!("failed to evaluate staleness: {}", e))?;
        let conn = db.conn();
        let all = crate::store::queries::list_all_nodes(conn)
            .map_err(|e| format!("failed to list nodes: {}", e))?;
        let stale = crate::store::queries::list_stale_nodes(conn)
            .map_err(|e| format!("failed to list stale nodes: {}", e))?;
        Ok(Json(crate::cli::status::build_status_summary(&all, &stale)))
    }

    /// Export or visualize the dependency graph structure in text, mermaid, dot, or ttl format.
    #[tool]
    async fn export_graph(
        &self,
        Parameters(ExportGraphParams {
            format,
            include_absolute_paths,
        }): Parameters<ExportGraphParams>,
    ) -> Result<Json<serde_json::Value>, String> {
        let fmt = format.unwrap_or_else(|| "text".to_string());
        let abs = include_absolute_paths.unwrap_or(false);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let graph_text = crate::cli::graph::render_graph_string(&fmt, abs, deadline)?;
        Ok(Json(serde_json::json!({
            "format": fmt,
            "graph": graph_text,
        })))
    }
}

/// The MCP server handler (tools-only; `server_handler` above emits the
/// `ServerHandler` impl).
struct RgtMcpServer;
