#![forbid(unsafe_code)]

use schemars::schema_for;
use std::{
    env, fs,
    io::{self, BufRead, Read},
};
use suture::{
    pipeline::execute_request,
    protocol::{
        Certificate, CommitGuarantee, EffectBudget, EffectUsage, FailureReason, Outcome,
        PreservationFacts, RefusalReason, Request, StructuralValidation, TransactionCertificate,
        TransactionRequest, PROTOCOL_VERSION,
    },
    workspace::Workspace,
};

fn help() {
    println!("suture {PROTOCOL_VERSION} - deterministic, source-preserving file mutation\n\nUSAGE:\n  suture mutate [--request FILE]\n  suture preview [--request FILE]\n  suture capabilities\n  suture inspect PATH\n  suture transact [--request FILE]\n  suture transaction-preview [--request FILE]\n  suture recover\n  suture schema\n  suture doctor\n  suture --version");
}
fn empty_cert(reason: RefusalReason) -> Certificate {
    Certificate {
        protocol_version: PROTOCOL_VERSION.into(),
        request_id: String::new(),
        outcome: Outcome::Refused,
        file_path: String::new(),
        provider: "request".into(),
        provider_version: "parser".into(),
        expected_cardinality: Default::default(),
        observed_cardinality: None,
        pre_hash: String::new(),
        post_hash: None,
        changed_ranges: Vec::new(),
        changed_line_ranges: Vec::new(),
        diff_summary: None,
        diff_truncated: false,
        structural_validation: StructuralValidation::NotApplicable,
        preservation: PreservationFacts::default(),
        commit: CommitGuarantee::default(),
        refusal_reason: Some(reason),
        failure_reason: None,
        diagnostics: Vec::new(),
        budget: EffectBudget::default(),
        effect: EffectUsage {
            files: 0,
            matches: 0,
            changed_regions: 0,
            changed_lines: 0,
            changed_bytes: 0,
            passed: true,
        },
        transaction_guarantee: "not_committed".into(),
        recovery_state: "not_required".into(),
    }
}
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|x| x == "--version" || x == "-V") {
        println!("suture {PROTOCOL_VERSION}");
        return;
    }
    if args.get(1).is_some_and(|x| x == "--help" || x == "-h") {
        help();
        return;
    }
    let Some(command) = args.get(1).map(String::as_str) else {
        help();
        std::process::exit(1)
    };
    if command == "mcp" {
        run_mcp();
        return;
    }
    match command {
        "apply" | "dry-run" | "mutate" | "preview" => {
            let dry = matches!(command, "dry-run" | "preview");
            let request_path = args
                .windows(2)
                .find(|w| w[0] == "--request")
                .map(|w| w[1].clone());
            let mut input = String::new();
            let read = if let Some(p) = request_path {
                fs::read_to_string(p)
            } else {
                io::stdin().read_to_string(&mut input).map(|_| input)
            };
            let input = match read {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("request read failed: {e}");
                    std::process::exit(3)
                }
            };
            let req: Request = match serde_json::from_str(&input) {
                Ok(r) => r,
                Err(e) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&empty_cert(RefusalReason::MalformedInput {
                            details: e.to_string()
                        }))
                        .unwrap()
                    );
                    std::process::exit(2)
                }
            };
            let root = match env::current_dir() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("cannot determine workspace: {e}");
                    std::process::exit(3)
                }
            };
            let ws = match Workspace::new(root) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("workspace initialization failed: {e}");
                    std::process::exit(3)
                }
            };
            let cert = execute_request(&ws, &req, dry);
            println!("{}", serde_json::to_string_pretty(&cert).unwrap());
            match cert.outcome {
                Outcome::Refused => std::process::exit(2),
                Outcome::Failed => std::process::exit(3),
                Outcome::Applied | Outcome::NoChange => {}
            }
        }
        "transact" | "transaction-preview" => {
            let dry = command == "transaction-preview";
            let request_path = args
                .windows(2)
                .find(|w| w[0] == "--request")
                .map(|w| w[1].clone());
            let mut input = String::new();
            let read = if let Some(p) = request_path {
                fs::read_to_string(p)
            } else {
                io::stdin().read_to_string(&mut input).map(|_| input)
            };
            let input = match read {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("request read failed: {e}");
                    std::process::exit(3);
                }
            };
            let transaction: TransactionRequest = match serde_json::from_str(&input) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("transaction request parse failed: {e}");
                    std::process::exit(2);
                }
            };
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            let ws = match Workspace::new(root) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("workspace initialization failed: {e}");
                    std::process::exit(3);
                }
            };
            let certificate = suture::pipeline::execute_transaction(&ws, &transaction, dry);
            println!("{}", serde_json::to_string_pretty(&certificate).unwrap());
            match certificate.outcome {
                Outcome::Refused => std::process::exit(2),
                Outcome::Failed => std::process::exit(3),
                Outcome::Applied | Outcome::NoChange => {}
            }
        }
        "recover" => {
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            let ws = match Workspace::new(root) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("workspace initialization failed: {e}");
                    std::process::exit(3);
                }
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&suture::recovery::recover_all(&ws)).unwrap()
            );
        }
        "capabilities" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&suture::capabilities::current()).unwrap()
            );
        }
        "inspect" => {
            let Some(path) = args.get(2) else {
                eprintln!("inspect requires a workspace-relative path");
                std::process::exit(1);
            };
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            let ws = match Workspace::new(root) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("workspace initialization failed: {e}");
                    std::process::exit(3);
                }
            };
            match ws.read_file(path) {
                Ok(bytes) => {
                    let newline = if bytes.windows(2).any(|w| w == b"\r\n") {
                        "crlf"
                    } else if bytes.contains(&b'\n') {
                        "lf"
                    } else {
                        "none"
                    };
                    let out = serde_json::json!({"protocol_version": PROTOCOL_VERSION, "file_path": path, "bytes": bytes.len(), "sha256": suture::engine::compute_sha256(&bytes), "encoding": if bytes.starts_with(&[0xef,0xbb,0xbf]) { "utf8_bom" } else { "utf8" }, "newline_profile": newline, "final_newline": bytes.ends_with(b"\n")});
                    println!("{}", serde_json::to_string_pretty(&out).unwrap());
                }
                Err(e) => {
                    eprintln!("inspect failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        "schema" => {
            let out = serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":"Suture v1.0 Protocol Schemas","protocol_version":PROTOCOL_VERSION,"request":schema_for!(Request),"certificate":schema_for!(Certificate),"transaction_request":schema_for!(TransactionRequest),"transaction_certificate":schema_for!(TransactionCertificate)});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        "doctor" => {
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            println!("suture doctor\nos: {}\narch: {}\nworkspace: {}\nprotocol: {}\nproviders: text json jsonc toml yaml markdown dotenv pattern patch code filesystem\ntransport: stdin/stdout mcp/stdio\ncommit: staged atomic replacement; recovery journal available",env::consts::OS,env::consts::ARCH,match Workspace::new(root){Ok(_)=>"ready",Err(_)=>"unavailable"}, PROTOCOL_VERSION);
        }
        _ => {
            eprintln!("unknown command: {command}");
            help();
            std::process::exit(1)
        }
    }
}

fn run_mcp() {
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
    for line in io::stdin().lock().lines().map_while(Result::ok) {
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
                serde_json::json!({"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "suture", "version": PROTOCOL_VERSION}})
            }
            "tools/list" => serde_json::json!({"tools": [
                {"name": "suture_mutate", "description": "Apply one typed Suture mutation and return its certificate", "inputSchema": schema_for!(Request)},
                {"name": "suture_capabilities", "description": "Return Suture capabilities", "inputSchema": {"type": "object"}},
                {"name": "suture_transact", "description": "Prepare and commit a guarded transaction", "inputSchema": schema_for!(TransactionRequest)}
            ]}),
            "tools/call" => {
                let params = request.get("params").cloned().unwrap_or_default();
                let name = params.get("name").and_then(|x| x.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or_default();
                let value = match name {
                    "suture_capabilities" => {
                        Ok(serde_json::to_value(suture::capabilities::current()).unwrap())
                    }
                    "suture_mutate" => serde_json::from_value::<Request>(arguments).map(|r| {
                        serde_json::to_value(execute_request(&workspace, &r, false)).unwrap()
                    }),
                    "suture_transact" => serde_json::from_value::<TransactionRequest>(arguments)
                        .map(|r| {
                            serde_json::to_value(suture::pipeline::execute_transaction(
                                &workspace, &r, false,
                            ))
                            .unwrap()
                        }),
                    _ => Err(serde_json::Error::io(io::Error::new(
                        io::ErrorKind::NotFound,
                        "unknown Suture tool",
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

#[allow(dead_code)]
fn _failure_type_is_linked(_: FailureReason) {}
