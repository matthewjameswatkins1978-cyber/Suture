use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn call_cli(workspace: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_threadmoth"))
        .current_dir(workspace.path())
        .args(args)
        .output()
        .expect("Threadmoth CLI should run")
}

fn call_mcp(workspace: &TempDir, messages: &[Value]) -> Vec<Value> {
    let input = messages
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let mut child = Command::new(env!("CARGO_BIN_EXE_threadmoth"))
        .current_dir(workspace.path())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("MCP server should run");
    child
        .stdin
        .take()
        .expect("MCP stdin should be available")
        .write_all(input.as_bytes())
        .expect("MCP request should be written");
    let output = child.wait_with_output().expect("MCP server should exit");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("MCP output must be JSON"))
        .collect()
}

#[test]
fn registry_classifies_special_filenames_without_shadow_tables() {
    for (path, provider) in [
        ("test.ini", "ini"),
        ("setup.cfg", "ini"),
        ("tox.ini", "ini"),
        ("pytest.ini", "ini"),
        (".editorconfig", "ini"),
        (".env", "dotenv"),
        (".env.local", "dotenv"),
        (".env.production", "dotenv"),
        ("config.json", "json"),
        ("config.jsonc", "jsonc"),
        ("config.yaml", "yaml"),
        ("config.yml", "yaml"),
        ("config.toml", "toml"),
    ] {
        let detection = threadmoth::target_registry::detect(path, None);
        assert_eq!(detection.provider.as_deref(), Some(provider), "{path}");
    }
}

#[test]
fn cli_shorthand_uses_registry_for_ini_filenames_and_explicit_strings() {
    let workspace = TempDir::new().unwrap();
    fs::write(
        workspace.path().join("setup.cfg"),
        b"[metadata]\nname = old_pkg\nversion = 0.1.0\n",
    )
    .unwrap();
    let result = call_cli(
        &workspace,
        &[
            "set-value",
            "setup.cfg",
            "metadata.version",
            "0.2.0",
            "--string",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(workspace.path().join("setup.cfg")).unwrap(),
        b"[metadata]\nname = old_pkg\nversion = 0.2.0\n"
    );

    let result = call_cli(
        &workspace,
        &["set-string", "setup.cfg", "metadata.version", "0.3.0"],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&fs::read(workspace.path().join("setup.cfg")).unwrap())
            .contains("version = 0.3.0")
    );
}

#[test]
fn strict_json_never_coerces_and_refusals_leave_bytes_unchanged() {
    let workspace = TempDir::new().unwrap();
    let original = b"[metadata]\nversion = 0.1.0\n";
    fs::write(workspace.path().join("setup.cfg"), original).unwrap();

    let result = call_cli(
        &workspace,
        &["set-value", "setup.cfg", "metadata.version", "0.2.0"],
    );
    assert!(!result.status.success());
    assert_eq!(
        fs::read(workspace.path().join("setup.cfg")).unwrap(),
        original
    );

    for value in [
        "true",
        "false",
        "null",
        "123",
        "123.45",
        "1e6",
        "001",
        "127.0.0.1",
        "hello",
        "hello world",
        "v1.9.1",
        "${HOME}",
    ] {
        let result = call_cli(
            &workspace,
            &[
                "set-value",
                "setup.cfg",
                "metadata.version",
                value,
                "--string",
            ],
        );
        assert!(
            result.status.success(),
            "value={value}: stdout={} stderr={}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn mcp_and_cli_resolve_the_same_special_filename() {
    let workspace = TempDir::new().unwrap();
    fs::write(
        workspace.path().join("setup.cfg"),
        b"[metadata]\nversion = 0.1.0\n",
    )
    .unwrap();
    let output = call_mcp(
        &workspace,
        &[
            json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"threadmoth_inspect","arguments":{"path":"setup.cfg"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"threadmoth_set_value","arguments":{"file":"setup.cfg","path":"metadata.version","value":"0.2.0"}}}),
        ],
    );
    assert_eq!(
        output[0]["result"]["structuredContent"]["detection"]["provider"],
        "ini"
    );
    assert_eq!(
        output[1]["result"]["structuredContent"]["outcome"],
        "APPLIED"
    );
    assert_eq!(
        fs::read(workspace.path().join("setup.cfg")).unwrap(),
        b"[metadata]\nversion = 0.2.0\n"
    );
}
