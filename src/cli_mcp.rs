use schemars::schema_for;
use std::{env, io::{self, BufRead}};
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
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let id = request
            .get("id")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let result = match request.get("method").and_then(|x| x.as_str()).unwrap_or("") {
            "initialize" => {
                serde_json::json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "threadmoth", "version": THREADMOTH_VERSION, "protocol_version": PROTOCOL_VERSION}})
            }
            "tools/list" => serde_json::json!({"tools": [
                {"name": "threadmoth_mutate", "description": "Apply one typed Threadmoth mutation and return its certificate", "inputSchema": schema_for!(Request)},
                {"name": "threadmoth_capabilities", "description": "Return Threadmoth capabilities", "inputSchema": {"type": "object"}},
                {"name": "threadmoth_transact", "description": "Prepare and commit a guarded transaction", "inputSchema": schema_for!(TransactionRequest)}
            ]}),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or_default();
                let name = params.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or_default();
                let value = match name {
                    "threadmoth_capabilities" | "suture_capabilities" => {
                        Ok(serde_json::to_value(threadmoth::capabilities::current()).unwrap())
                    }
                    "threadmoth_mutate" | "suture_mutate" => {
                        serde_json::from_value::<Request>(arguments).map(|r| {
                            serde_json::to_value(execute_request(&workspace, &r, false)).unwrap()
                        })
                    }
                    "threadmoth_transact" | "suture_transact" => {
                        serde_json::from_value::<TransactionRequest>(arguments).map(|r| {
                            serde_json::to_value(threadmoth::pipeline::execute_transaction(
                                &workspace, &r, false,
                            ))
                            .unwrap()
                        })
                    }
                    _ => Err(serde_json::Error::io(io::Error::new(
                        io::ErrorKind::NotFound,
                        "unknown Threadmoth tool",
                    ))),
                };
                match value {
                    Ok(value) => {
                        serde_json::json!({"content": [{"type": "text", "text": serde_json::to_string(&value).unwrap()}], "structuredContent": value})
                    }
                    Err(e) => {
                        serde_json::json!({"isError": true, "content": [{"type": "text", "text": e.to_string()}]})
                    }
                }
            }
            _ => serde_json::json!({}),
        };
        println!(
            "{}",
            serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result})
        );
    }
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
