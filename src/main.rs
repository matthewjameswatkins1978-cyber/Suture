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
        PreservationFacts, RefusalReason, Request, StructuralValidation, TransactionRequest,
        PROTOCOL_VERSION,
    },
    workspace::Workspace,
};

fn help() {
    println!("suture {PROTOCOL_VERSION} - deterministic mutation of existing workspace state\n\nSuture changes exactly the state a request authorizes, refuses ambiguity, and returns a certificate. It is not Git, a build/test runner, a formatter, a shell, or an online service.\n\nCOMMANDS");
    for (name, description) in suture::metadata::commands() {
        println!("  {name:<14} {description}");
    }
    println!("\nOUTCOMES\n  APPLIED       verified candidate committed\n  NO_CHANGE     requested desired state already held\n  REFUSED       no bytes written; request or evidence was unsafe/ambiguous\n  FAILED        execution or verification failed; inspect recovery state\n\nSAFETY\n  Providers propose edits; Core validates identity, cardinality, preservation and budgets before writing.\n  Start with: suture capabilities, suture examples, suture schema, suture suggest PATH\n  Help for one command: suture help <command>\n  Search help: suture help --find <term>");
}

fn print_examples(topic: Option<&str>) {
    let examples = suture::metadata::examples(topic);
    if examples.is_empty() {
        eprintln!("no example topic matched");
        std::process::exit(1);
    }
    println!("{}", serde_json::to_string_pretty(&examples).unwrap());
}

fn print_explain(code: &str, json_output: bool) {
    let Some(reason) = suture::metadata::reason(code) else {
        eprintln!("unknown reason code: {code}");
        std::process::exit(1);
    };
    if json_output {
        println!("{}", serde_json::to_string_pretty(&reason).unwrap());
    } else {
        println!(
            "{} — {}\nWhy: {}\nRecovery: {}\nRetry unchanged: {}\nCommands: {}",
            reason.code,
            reason.meaning,
            reason.why_refused,
            reason.recovery_category,
            reason.retry_unchanged,
            reason.relevant_commands.join(", ")
        );
    }
}

fn read_json_argument(args: &[String]) -> String {
    if let Some(index) = args.iter().position(|arg| arg == "--request") {
        if let Some(path) = args.get(index + 1) {
            return match fs::read_to_string(path) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("request read failed: {error}");
                    std::process::exit(3);
                }
            };
        }
    }
    let mut input = String::new();
    match io::stdin().read_to_string(&mut input) {
        Ok(_) => input,
        Err(error) => {
            eprintln!("request read failed: {error}");
            std::process::exit(3);
        }
    }
}

fn bool_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn option_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}
fn empty_cert(reason: RefusalReason) -> Certificate {
    let reason_code = reason.code().into();
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
        reason_code: Some(reason_code),
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
            let selector = args
                .iter()
                .skip(2)
                .find(|arg| !arg.starts_with('-'))
                .map(String::as_str);
            let mut output = suture::metadata::capability_view(selector);
            if let Some(path) = option_value(&args, "--for") {
                let root = env::current_dir().unwrap_or_else(|_| ".".into());
                let ws = Workspace::new(root).unwrap();
                let bytes = ws.read_file(&path).ok();
                output = suture::metadata::capabilities_for(&path, bytes.as_deref());
            }
            let rendered = if bool_flag(&args, "--json") && !bool_flag(&args, "--pretty") {
                serde_json::to_string(&output).unwrap()
            } else {
                serde_json::to_string_pretty(&output).unwrap()
            };
            println!("{rendered}");
        }
        "examples" => {
            print_examples(args.get(2).map(String::as_str));
        }
        "help" => {
            if let Some(term) = option_value(&args, "--find") {
                for (name, description) in suture::metadata::find_help(&term) {
                    println!("{name}: {description}");
                }
            } else if let Some(command) = args.get(2) {
                match suture::metadata::command_help(command) {
                    Some(text) => println!("suture help {command}\n\n{text}"),
                    None => {
                        eprintln!("unknown command: {command}");
                        std::process::exit(1);
                    }
                }
            } else {
                help();
            }
        }
        "explain" => {
            let Some(code) = args.get(2) else {
                eprintln!("explain requires a reason code");
                std::process::exit(1);
            };
            print_explain(code, bool_flag(&args, "--json"));
        }
        "suggest" => {
            if args.iter().any(|arg| arg == "--from-refusal") {
                let source = option_value(&args, "--from-refusal").unwrap_or_else(|| "-".into());
                let input = if source == "-" {
                    read_json_argument(&[])
                } else {
                    fs::read_to_string(source).unwrap_or_else(|error| {
                        eprintln!("refusal certificate read failed: {error}");
                        std::process::exit(3);
                    })
                };
                let certificate: Certificate =
                    serde_json::from_str(&input).unwrap_or_else(|error| {
                        eprintln!("invalid refusal certificate: {error}");
                        std::process::exit(2);
                    });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&suture::metadata::refusal_recovery(&certificate))
                        .unwrap()
                );
            } else {
                let Some(path) = args.get(2) else {
                    eprintln!(
                        "suggest requires a workspace-relative path or --from-refusal CERTIFICATE"
                    );
                    std::process::exit(1);
                };
                let root = env::current_dir().unwrap_or_else(|_| ".".into());
                let ws = Workspace::new(root).unwrap();
                let bytes = ws.read_file(path).ok();
                let suggestion = suture::metadata::suggest(
                    path,
                    option_value(&args, "--goal").as_deref(),
                    option_value(&args, "--at").as_deref(),
                    option_value(&args, "--mode").as_deref().unwrap_or("safe"),
                    bytes.as_deref(),
                );
                println!("{}", serde_json::to_string_pretty(&suggestion).unwrap());
            }
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
            let scope = args
                .iter()
                .skip(2)
                .find(|arg| !arg.starts_with('-'))
                .map(String::as_str);
            let out = suture::metadata::schema(scope);
            if bool_flag(&args, "--json") && !bool_flag(&args, "--pretty") {
                println!("{}", serde_json::to_string(&out).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
            }
        }
        "doctor" => {
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            let providers = suture::metadata::provider_metadata()
                .into_iter()
                .map(|provider| provider.name)
                .collect::<Vec<_>>()
                .join(" ");
            println!("suture doctor\nos: {}\narch: {}\nworkspace: {}\nprotocol: {}\nproviders: {}\ntransport: stdin/stdout mcp/stdio\ncommit: staged atomic replacement; recovery journal available",env::consts::OS,env::consts::ARCH,match Workspace::new(root){Ok(_)=>"ready",Err(_)=>"unavailable"}, PROTOCOL_VERSION, providers);
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
