mod db;
mod sync;

use db::Database;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, io};
use sync::{synchronize, OracleExtractor, OracleModule};
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
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("sync") {
        run_sync_command(&args[1..])
            .await
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        return Ok(());
    }
    run_mcp().await
}

async fn run_sync_command(args: &[String]) -> anyhow::Result<()> {
    let (release, modules, activate) = parse_sync_args(args)?;
    let database_path =
        env::var("ORACLE_MCP_DATABASE").unwrap_or_else(|_| "oracle-erp-mcp.sqlite".to_owned());
    let database = Database::open(&database_path)?;
    let extractor = OracleExtractor::new()?;
    let mut tables = Vec::new();

    for module in modules {
        let source = sync::OracleSource::help_center(module, &release)?;
        eprintln!("downloading {} {}", module.label(), source.index_url);
        let extracted = extractor.extract(&source).await?;
        eprintln!(
            "extracted {} catalog entries from {}",
            extracted.len(),
            module.label()
        );
        tables.extend(extracted);
    }

    let version_id = synchronize(&database, &release, tables, activate)?;
    eprintln!("synchronized release {release} as version {version_id}");
    Ok(())
}

fn parse_sync_args(args: &[String]) -> anyhow::Result<(String, Vec<OracleModule>, bool)> {
    let mut release = None;
    let mut modules = vec![OracleModule::Financials, OracleModule::Scm];
    let mut activate = true;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--release" => {
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| anyhow::anyhow!("--release requires a value"))?;
                release = Some(value.to_ascii_uppercase());
            }
            "--module" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| anyhow::anyhow!("--module requires a value"))?;
                modules = match value.to_ascii_lowercase().as_str() {
                    "financials" => vec![OracleModule::Financials],
                    "scm" => vec![OracleModule::Scm],
                    "all" => vec![OracleModule::Financials, OracleModule::Scm],
                    _ => {
                        return Err(anyhow::anyhow!(
                            "unsupported module {value}; use financials, scm, or all"
                        ))
                    }
                };
            }
            "--no-activate" => activate = false,
            "-h" | "--help" => {
                eprintln!(
                    "Usage: cargo run -- sync --release RELEASE [--module financials|scm|all] [--no-activate]"
                );
                return Err(anyhow::anyhow!("help requested"));
            }
            value => return Err(anyhow::anyhow!("unsupported argument: {value}")),
        }
        index += 1;
    }

    let release = release.ok_or_else(|| anyhow::anyhow!("sync requires --release RELEASE"))?;
    Ok((release, modules, activate))
}

async fn run_mcp() -> io::Result<()> {
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
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal error"}}"#
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
                "name": "list_modules_and_tables",
                "description": "Lists tables from the active release, optionally filtered by module.",
                "inputSchema": {
                    "type": "object",
                    "properties": { "module": { "type": "string" } }
                }
            },
            {
                "name": "search_table_structure",
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
                "name": "suggest_joins",
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
        "list_modules_and_tables" => {
            let module = arguments.get("module").and_then(Value::as_str);
            let tables = database
                .list_modules_and_tables(module)
                .map_err(|error| error.to_string())?;
            Ok(tool_result(
                serde_json::to_value(tables).map_err(|e| e.to_string())?,
            ))
        }
        "search_table_structure" => {
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
        "suggest_joins" => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sync_options() {
        let args = vec![
            "--release".to_owned(),
            "26b".to_owned(),
            "--module".to_owned(),
            "scm".to_owned(),
            "--no-activate".to_owned(),
        ];
        let (release, modules, activate) = parse_sync_args(&args).expect("sync options");
        assert_eq!(release, "26B");
        assert_eq!(modules, vec![OracleModule::Scm]);
        assert!(!activate);
    }

    #[test]
    fn requires_sync_release() {
        let error = parse_sync_args(&[]).expect_err("missing release");
        assert_eq!(error.to_string(), "sync requires --release RELEASE");
    }
}
