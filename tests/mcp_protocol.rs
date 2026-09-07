use serde_json::{json, Value};
use std::fs;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn call_mcp(workspace: &TempDir, messages: &[Value]) -> Vec<Value> {
    let input = messages
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let output = Command::new(env!("CARGO_BIN_EXE_threadmoth"))
        .current_dir(workspace.path())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .expect("MCP server should run");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("MCP output must be JSON"))
        .collect()
}

fn call_mcp_raw(workspace: &TempDir, input: &str) -> Vec<Value> {
    let output = Command::new(env!("CARGO_BIN_EXE_threadmoth"))
        .current_dir(workspace.path())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .expect("MCP server should run");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("MCP output must be JSON"))
        .collect()
}

fn replace_request(path: &str) -> Value {
    json!({
        "version": "1.1.0",
        "request_id": "mcp-test",
        "file_path": path,
        "cardinality": {"type": "exactly_one"},
        "operation": {"provider": "text", "operation": {"type": "replace", "target": "old", "replacement": "new"}}
    })
}

#[test]
fn tools_list_exposes_preview_and_notification_has_no_response() {
    let workspace = TempDir::new().unwrap();
    let messages = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    ];
    let output = call_mcp(&workspace, &messages);

    assert_eq!(output.len(), 1);
    assert!(output[0]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "threadmoth_preview"));
    for tool_name in [
        "threadmoth_inspect",
        "threadmoth_suggest",
        "threadmoth_explain",
        "threadmoth_transact_preview",
    ] {
        assert!(output[0]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == tool_name));
    }
}

#[test]
fn read_only_discovery_tools_reuse_cli_metadata() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("config.json"), b"{\"port\": 8080}\n").unwrap();
    let output = call_mcp(
        &workspace,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"threadmoth_inspect","arguments":{"path":"config.json"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"threadmoth_suggest","arguments":{"path":"config.json","goal":"set-value","at":"$.port","mode":"safe"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"threadmoth_explain","arguments":{"code":"TARGET_AMBIGUOUS"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"threadmoth_capabilities","arguments":{"selector":"json.set","for_path":"config.json"}}}),
        ],
    );
    assert_eq!(
        output[0]["result"]["structuredContent"]["file_path"],
        "config.json"
    );
    assert_eq!(output[0]["result"]["structuredContent"]["bytes"], 15);
    assert_eq!(output[1]["result"]["structuredContent"]["provider"], "json");
    assert_eq!(
        output[2]["result"]["structuredContent"]["code"],
        "TARGET_AMBIGUOUS"
    );
    assert_eq!(
        output[3]["result"]["structuredContent"]["target"]["provider"],
        "json"
    );
    assert_eq!(
        fs::read(workspace.path().join("config.json")).unwrap(),
        b"{\"port\": 8080}\n"
    );
}

#[test]
fn transaction_preview_is_non_writing_and_matches_commit_plan() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("x.txt"), b"old\n").unwrap();
    let transaction = json!({
        "version": "1.2.0",
        "transaction_id": "mcp-preview",
        "requests": [replace_request("x.txt")]
    });
    let output = call_mcp(
        &workspace,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"threadmoth_transact_preview","arguments":transaction}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"threadmoth_transact","arguments":transaction}}),
        ],
    );
    assert_eq!(
        output[0]["result"]["structuredContent"]["outcome"],
        "APPLIED"
    );
    assert_eq!(
        output[0]["result"]["structuredContent"]["transaction_guarantee"],
        "dry_run"
    );
    assert_eq!(
        output[1]["result"]["structuredContent"]["outcome"],
        "APPLIED"
    );
    assert_eq!(
        output[0]["result"]["structuredContent"]["certificates"][0]["changed_ranges"],
        output[1]["result"]["structuredContent"]["certificates"][0]["changed_ranges"]
    );
    assert_eq!(fs::read(workspace.path().join("x.txt")).unwrap(), b"new\n");
}

#[test]
fn malformed_discovery_inputs_fail_as_tool_errors() {
    let workspace = TempDir::new().unwrap();
    let output = call_mcp(
        &workspace,
        &[json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"threadmoth_inspect","arguments":{"path":"x.txt","extra":true}}
        })],
    );
    assert_eq!(output[0]["result"]["isError"], true);
}

#[test]
fn preview_is_structured_and_does_not_write() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("x.txt"), b"old\n").unwrap();
    let output = call_mcp(
        &workspace,
        &[json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"threadmoth_preview","arguments":replace_request("x.txt")}
        })],
    );

    assert_eq!(output[0]["id"], 1);
    assert_eq!(
        output[0]["result"]["structuredContent"]["outcome"],
        "APPLIED"
    );
    assert_eq!(fs::read(workspace.path().join("x.txt")).unwrap(), b"old\n");
}

#[test]
fn mutate_still_commits_and_refusal_is_structured() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("x.txt"), b"old\n").unwrap();
    let output = call_mcp(
        &workspace,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"threadmoth_mutate","arguments":replace_request("x.txt")}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"threadmoth_preview","arguments":replace_request("missing.txt")}}),
        ],
    );

    assert_eq!(fs::read(workspace.path().join("x.txt")).unwrap(), b"new\n");
    assert_eq!(
        output[0]["result"]["structuredContent"]["outcome"],
        "APPLIED"
    );
    assert_eq!(
        output[1]["result"]["structuredContent"]["outcome"],
        "REFUSED"
    );
    assert!(output[1]["result"]["structuredContent"]["refusal_reason"].is_object());
}

#[test]
fn unknown_method_returns_json_rpc_method_not_found() {
    let workspace = TempDir::new().unwrap();
    let output = call_mcp(
        &workspace,
        &[json!({"jsonrpc":"2.0","id":"unknown","method":"nope"})],
    );

    assert_eq!(output[0]["id"], "unknown");
    assert_eq!(output[0]["error"]["code"], -32601);
}

#[test]
fn initialization_and_malformed_request_are_deterministic() {
    let workspace = TempDir::new().unwrap();
    let output = call_mcp_raw(
        &workspace,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\nnot-json\n",
    );

    assert_eq!(output.len(), 2);
    assert_eq!(output[0]["result"]["serverInfo"]["name"], "threadmoth");
    assert_eq!(output[1]["error"]["code"], -32700);
}

#[test]
fn preview_hash_is_stale_after_external_modification() {
    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("x.txt"), b"old\n").unwrap();
    let preview = call_mcp(
        &workspace,
        &[json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"threadmoth_preview","arguments":replace_request("x.txt")}
        })],
    );
    let pre_hash = preview[0]["result"]["structuredContent"]["pre_hash"]
        .as_str()
        .unwrap();
    fs::write(workspace.path().join("x.txt"), b"changed\n").unwrap();
    let mut request = replace_request("x.txt");
    request["expected_pre_hash"] = json!(pre_hash);
    let result = call_mcp(
        &workspace,
        &[json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"threadmoth_mutate","arguments":request}
        })],
    );

    assert_eq!(
        result[0]["result"]["structuredContent"]["outcome"],
        "REFUSED"
    );
    assert_eq!(
        result[0]["result"]["structuredContent"]["reason_code"],
        "STALE_IDENTITY"
    );
    assert_eq!(
        fs::read(workspace.path().join("x.txt")).unwrap(),
        b"changed\n"
    );
}
