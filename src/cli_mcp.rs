use schemars::schema_for;
use serde_json::{json, Value};
use std::{
    env,
    io::{self, BufRead},
};
use threadmoth::{
    pipeline::execute_request,
    protocol::{Request, TransactionRequest, MAX_REQUEST_BYTES, PROTOCOL_VERSION},
    workspace::Workspace,
};

use crate::cli::THREADMOTH_VERSION;

pub fn run_mcp() {
    let workspace = match env::current_dir() {
        Ok(path) => match Workspace::new(path) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("workspace initialization failed: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("workspace initialization failed: {e}");
            return;
        }
    };
    let mut input = io::stdin().lock();
    loop {
        let line = match read_mcp_line(&mut input) {
            Ok(Some(Ok(line))) => line,
            Ok(Some(Err(actual))) => {
                println!(
                    "{}",
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": serde_json::Value::Null,
                        "error": {
                            "code": -32600,
                            "message": format!("request exceeds {MAX_REQUEST_BYTES} bytes (actual: {actual})")
                        }
                    })
                );
                continue;
            }
            Ok(None) => break,
            Err(error) => {
                eprintln!("MCP input read failed: {error}");
                break;
            }
        };
        let request: Value = match serde_json::from_str(&line) {
            Ok(x) => x,
            Err(error) => {
                println!("{}", json_rpc_error(Value::Null, -32700, error.to_string()));
                continue;
            }
        };
        if let Some(response) = handle_mcp_message(&workspace, request) {
            println!("{}", response);
        }
    }
}

fn handle_mcp_message(workspace: &Workspace, request: Value) -> Option<Value> {
    let Some(object) = request.as_object() else {
        return Some(json_rpc_error(
            Value::Null,
            -32600,
            "invalid JSON-RPC request".into(),
        ));
    };
    let notification = !object.contains_key("id");
    let id = object.get("id").cloned().unwrap_or(Value::Null);
    let body = if object.get("jsonrpc") != Some(&Value::String("2.0".into())) {
        json_rpc_error(id.clone(), -32600, "invalid JSON-RPC request".into())
    } else {
        match object.get("method").and_then(Value::as_str) {
            Some("initialize") => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "threadmoth", "version": THREADMOTH_VERSION, "protocol_version": PROTOCOL_VERSION}}
            }),
            Some("tools/list") => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"tools": [
                    {"name": "threadmoth_mutate", "description": "Apply one typed Threadmoth mutation and return its certificate", "inputSchema": schema_for!(Request)},
                    {"name": "threadmoth_preview", "description": "Preview one typed Threadmoth mutation without writing", "inputSchema": schema_for!(Request)},
                    {"name": "threadmoth_capabilities", "description": "Return Threadmoth capabilities", "inputSchema": {"type": "object"}},
                    {"name": "threadmoth_transact", "description": "Prepare and commit a guarded transaction", "inputSchema": schema_for!(TransactionRequest)}
                ]}
            }),
            Some("tools/call") => {
                let Some(params) = object.get("params").and_then(Value::as_object) else {
                    return if notification {
                        None
                    } else {
                        Some(json_rpc_error(
                            id,
                            -32602,
                            "tools/call params must be an object".into(),
                        ))
                    };
                };
                let Some(name) = params.get("name").and_then(Value::as_str) else {
                    return if notification {
                        None
                    } else {
                        Some(json_rpc_error(
                            id,
                            -32602,
                            "tools/call requires a tool name".into(),
                        ))
                    };
                };
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let value = call_tool(workspace, name, arguments);
                let result = match value {
                    Ok(value) => {
                        json!({"content": [{"type": "text", "text": serde_json::to_string(&value).unwrap()}], "structuredContent": value})
                    }
                    Err(error) => {
                        json!({"isError": true, "content": [{"type": "text", "text": error}]})
                    }
                };
                json!({"jsonrpc": "2.0", "id": id, "result": result})
            }
            Some(_) => json_rpc_error(id, -32601, "method not found".into()),
            None => json_rpc_error(id, -32600, "method is required".into()),
        }
    };
    if notification {
        None
    } else {
        Some(body)
    }
}

fn call_tool(workspace: &Workspace, name: &str, arguments: Value) -> Result<Value, String> {
    match name {
        "threadmoth_capabilities" | "suture_capabilities" => {
            serde_json::to_value(threadmoth::capabilities::current()).map_err(|e| e.to_string())
        }
        "threadmoth_mutate" | "suture_mutate" => {
            let request =
                serde_json::from_value::<Request>(arguments).map_err(|e| e.to_string())?;
            Ok(
                serde_json::to_value(execute_request(workspace, &request, false))
                    .map_err(|e| e.to_string())?,
            )
        }
        "threadmoth_preview" | "suture_preview" => {
            let request =
                serde_json::from_value::<Request>(arguments).map_err(|e| e.to_string())?;
            Ok(
                serde_json::to_value(execute_request(workspace, &request, true))
                    .map_err(|e| e.to_string())?,
            )
        }
        "threadmoth_transact" | "suture_transact" => {
            let transaction = serde_json::from_value::<TransactionRequest>(arguments)
                .map_err(|e| e.to_string())?;
            Ok(
                serde_json::to_value(threadmoth::pipeline::execute_transaction(
                    workspace,
                    &transaction,
                    false,
                ))
                .map_err(|e| e.to_string())?,
            )
        }
        _ => Err("unknown Threadmoth tool".into()),
    }
}

fn json_rpc_error(id: Value, code: i32, message: String) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn read_mcp_line(reader: &mut impl BufRead) -> io::Result<Option<Result<String, usize>>> {
    let mut bytes = Vec::new();
    let mut actual = 0usize;
    loop {
        let (content_len, available_len) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                if actual == 0 && bytes.is_empty() {
                    return Ok(None);
                }
                break;
            }
            let content_len = available
                .iter()
                .position(|byte| *byte == b'\n')
                .unwrap_or(available.len());
            actual = actual.saturating_add(content_len);
            if bytes.len() <= MAX_REQUEST_BYTES {
                let room = MAX_REQUEST_BYTES
                    .saturating_add(1)
                    .saturating_sub(bytes.len());
                bytes.extend_from_slice(&available[..content_len.min(room)]);
            }
            (content_len, available.len())
        };
        let consumed = if content_len < available_len {
            content_len + 1
        } else {
            content_len
        };
        reader.consume(consumed);
        if content_len < available_len {
            break;
        }
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
        actual = actual.saturating_sub(1);
    }
    if actual > MAX_REQUEST_BYTES {
        return Ok(Some(Err(actual)));
    }
    String::from_utf8(bytes)
        .map(|line| Some(Ok(line)))
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
