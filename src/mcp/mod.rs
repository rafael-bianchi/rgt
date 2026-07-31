pub mod handlers;
pub mod protocol;

pub use handlers::*;
pub use protocol::*;

use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

pub async fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let response = match req.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                req.id.clone(),
                json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "rgt-mcp-server",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }),
            ),
            "tools/list" => JsonRpcResponse::success(
                req.id.clone(),
                json!({
                    "tools": [
                        {
                            "name": "record_value",
                            "description": "Record a root numeric or date value read from a source file",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "value_kind": { "type": "string", "enum": ["NUMBER", "DATE"] },
                                    "number_value": { "type": "number" },
                                    "date_value": { "type": "string" },
                                    "line_number": { "type": "integer" }
                                },
                                "required": ["file_path", "value_kind"]
                            }
                        },
                        {
                            "name": "record_derivation",
                            "description": "Record a derived value or date-diff calculation linked to parent nodes",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "parent_node_ids": { "type": "array", "items": { "type": "string" } },
                                    "value_kind": { "type": "string", "enum": ["NUMBER", "DATE", "DURATION"] },
                                    "number_value": { "type": "number" },
                                    "date_value": { "type": "string" },
                                    "duration_seconds": { "type": "integer" },
                                    "operation_type": { "type": "string" },
                                    "expression": { "type": "string" }
                                },
                                "required": ["parent_node_ids", "value_kind", "operation_type"]
                            }
                        },
                        {
                            "name": "query_provenance",
                            "description": "Query the complete derivation chain and source files for a value node ID ('Why is this value X?')",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "node_id": { "type": "string" }
                                },
                                "required": ["node_id"]
                            }
                        },
                        {
                            "name": "list_stale_values",
                            "description": "List all recorded nodes currently marked as stale",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }),
            ),
            "tools/call" => {
                let params = req.params.clone().unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let res = match tool_name {
                    "record_value" => handle_record_value(&args),
                    "record_derivation" => handle_record_derivation(&args),
                    "query_provenance" => handle_query_provenance(&args),
                    "list_stale_values" => handle_list_stale_values(),
                    _ => Err(format!("Unknown tool: {}", tool_name)),
                };

                match res {
                    Ok(val) => JsonRpcResponse::success(req.id.clone(), json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }] })),
                    Err(err) => JsonRpcResponse::error(req.id.clone(), -32603, &err),
                }
            }
            _ => JsonRpcResponse::error(req.id.clone(), -32601, "Method not found"),
        };

        let resp_json = serde_json::to_string(&response).unwrap_or_default();
        stdout.write_all(resp_json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}
