mod db;
mod sync;

use db::Database;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, io};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let database_path =
        env::var("ORACLE_MCP_DATABASE").unwrap_or_else(|_| "oracle-erp-mcp.sqlite".to_owned());
    let database = match Database::open(&database_path) {
        Ok(database) => database,
        Err(error) => {
            eprintln!("could not open SQLite: {error}");
            return Err(io::Error::new(io::ErrorKind::Other, error.to_string()));
        }
    };
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(&database, request),
            Err(error) => error_response(
                None,
                -32700,
                "invalid JSON",
                Some(json!({ "detail": error.to_string() })),
            ),
        };
        let encoded = serde_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"error interno"}}"#
                .to_owned()
        });
        stdout.write_all(encoded.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }
    Ok(())
}

fn handle_request(database: &Database, request: JsonRpcRequest) -> JsonRpcResponse {
    if request.jsonrpc != "2.0" {
        return error_response(request.id, -32600, "jsonrpc must be 2.0", None);
    }
    match request.method.as_str() {
        "initialize" => success(
            request.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "oracle-erp-mcp", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "notifications/initialized" => success(request.id, json!({})),
        "tools/list" => success(request.id, tool_definitions()),
        "tools/call" => match call_tool(database, &request.params) {
            Ok(result) => success(request.id, result),
            Err(error) => error_response(request.id, -32602, &error, None),
        },
        _ if request.id.is_none() => success(None, json!({})),
        _ => error_response(request.id, -32601, "method not supported", None),
    }
}

fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "listar_modulos_y_tablas",
                "description": "Lists tables from the active release, optionally filtered by module.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "module": { "type": "string" } }
                }
            },
            {
                "name": "buscar_estructura_tabla",
                "description": "Searches for an exact table or by text and returns its technical structure.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "sugerir_joins",
                "description": "Returns the exact relationships between two tables.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "table_a": { "type": "string" },
                        "table_b": { "type": "string" }
                    },
                    "required": ["table_a", "table_b"]
                }
            }
        ]
    })
}

fn call_tool(database: &Database, params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("tools/call requires name")?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match name {
        "listar_modulos_y_tablas" => {
            let module = arguments.get("module").and_then(Value::as_str);
            let tables = database
                .list_modules_and_tables(module)
                .map_err(|error| error.to_string())?;
            Ok(tool_result(
                serde_json::to_value(tables).map_err(|e| e.to_string())?,
            ))
        }
        "buscar_estructura_tabla" => {
            let query = required_string(&arguments, "name")?;
            if let Some(structure) = database
                .table_structure(&query)
                .map_err(|error| error.to_string())?
            {
                return Ok(tool_result(
                    serde_json::to_value(structure).map_err(|e| e.to_string())?,
                ));
            }
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(10)
                .clamp(1, 100) as usize;
            let matches = database
                .search_tables(&query, limit)
                .map_err(|error| error.to_string())?;
            Ok(tool_result(
                serde_json::to_value(matches).map_err(|e| e.to_string())?,
            ))
        }
        "sugerir_joins" => {
            let left = required_string(&arguments, "table_a")?;
            let right = required_string(&arguments, "table_b")?;
            let references = database
                .suggest_joins(&left, &right)
                .map_err(|error| error.to_string())?;
            Ok(tool_result(
                serde_json::to_value(references).map_err(|e| e.to_string())?,
            ))
        }
        _ => Err(format!("tool not supported: {name}")),
    }
}

fn required_string(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("required argument: {key}"))
}

fn tool_result(value: Value) -> Value {
    json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&value).unwrap_or_default() }],
        "structuredContent": value,
        "isError": false
    })
}

fn success(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(
    id: Option<Value>,
    code: i32,
    message: &str,
    data: Option<Value>,
) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_owned(),
            data,
        }),
    }
}
