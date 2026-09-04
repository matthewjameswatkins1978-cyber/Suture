#![forbid(unsafe_code)]

mod cli;
mod cli_mcp;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use std::{
    env, fs,
    io::{self, Read, Write},
    path::Path,
};
use threadmoth::{
    pipeline::execute_request,
    protocol::{
        Certificate, CommitGuarantee, EffectBudget, EffectUsage, Outcome, PreservationFacts,
        RefusalReason, Request, StructuralValidation, TransactionCertificate, TransactionRequest,
        MAX_REQUEST_BYTES, PROTOCOL_VERSION,
    },
    workspace::Workspace,
};

use cli::{
    BenchmarkArgs, BenchmarkProfile, CapabilitiesArgs, Cli, Command, CompletionShell, HelpArgs,
    SchemaArgs, SuggestArgs, THREADMOTH_VERSION,
};

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Mutate(args) => run_request(args.request.as_deref(), false),
        Command::Preview(args) => run_request(args.request.as_deref(), true),
        Command::Transact(args) => run_transaction(args.request.as_deref(), args.preview),
        Command::TransactionPreview(args) => run_transaction(args.request.as_deref(), true),
        Command::Recover => run_recover(),
        Command::Capabilities(args) => run_capabilities(args),
        Command::Examples { topic } => print_examples(topic.as_deref()),
        Command::Benchmark(args) => run_benchmark(args),
        Command::Torture { json } => std::process::exit(threadmoth::torture::run(json)),
        Command::Help(args) => run_help(args),
        Command::Explain { code, json } => print_explain(&code, json),
        Command::Suggest(args) => run_suggest(args),
        Command::Inspect { path } => run_inspect(&path),
        Command::Schema(args) => run_schema(args),
        Command::Doctor => run_doctor(),
        Command::Completions { shell } => run_completions(shell),
        Command::Manpage { output } => run_manpage(output.as_deref()),
        Command::Mcp => cli_mcp::run_mcp(),
    }
}

fn run_request(request_path: Option<&Path>, dry: bool) {
    let input = match read_request_input(request_path) {
        Ok(s) => s,
        Err(RequestInputError::TooLarge(actual)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&empty_cert(
                    RefusalReason::ResourceLimitExceeded {
                        dimension: "max_request_bytes".into(),
                        limit: MAX_REQUEST_BYTES,
                        actual,
                    },
                ))
                .unwrap()
            );
            std::process::exit(2)
        }
        Err(RequestInputError::Io(error)) => {
            eprintln!("request read failed: {error}");
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
    exit_for_outcome(cert.outcome);
}

fn run_transaction(request_path: Option<&Path>, dry: bool) {
    let input = match read_request_input(request_path) {
        Ok(s) => s,
        Err(RequestInputError::TooLarge(actual)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&empty_transaction_certificate(
                    RefusalReason::ResourceLimitExceeded {
                        dimension: "max_request_bytes".into(),
                        limit: MAX_REQUEST_BYTES,
                        actual,
                    },
                ))
                .unwrap()
            );
            std::process::exit(2)
        }
        Err(RequestInputError::Io(error)) => {
            eprintln!("request read failed: {error}");
            std::process::exit(3);
        }
    };
    let transaction: TransactionRequest = match serde_json::from_str(&input) {
        Ok(x) => x,
        Err(e) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&empty_transaction_certificate(
                    RefusalReason::MalformedInput {
                        details: format!("transaction request parse failed: {e}"),
                    },
                ))
                .unwrap()
            );
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
    let certificate = threadmoth::pipeline::execute_transaction(&ws, &transaction, dry);
    println!("{}", serde_json::to_string_pretty(&certificate).unwrap());
    exit_for_outcome(certificate.outcome);
}

fn exit_for_outcome(outcome: Outcome) {
    match outcome {
        Outcome::Refused => std::process::exit(2),
        Outcome::Failed => std::process::exit(3),
        Outcome::Applied | Outcome::NoChange => {}
    }
}

fn run_recover() {
    let root = env::current_dir().unwrap_or_else(|_| ".".into());
    let ws = match Workspace::new(root) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("workspace initialization failed: {e}");
            std::process::exit(3)
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&threadmoth::recovery::recover_all(&ws)).unwrap()
    );
}

fn run_capabilities(args: CapabilitiesArgs) {
    let _ = args.all;
    let mut output = threadmoth::metadata::capability_view(args.selector.as_deref());
    if let Some(path) = args.for_path {
        let root = env::current_dir().unwrap_or_else(|_| ".".into());
        let ws = Workspace::new(root).unwrap_or_else(|e| {
            eprintln!("workspace initialization failed: {e}");
            std::process::exit(3)
        });
        let display = path.to_string_lossy();
        let bytes = ws.read_file(display.as_ref()).ok();
        output = threadmoth::metadata::capabilities_for(display.as_ref(), bytes.as_deref());
    }
    let rendered = if args.json && !args.pretty {
        serde_json::to_string(&output).unwrap()
    } else {
        serde_json::to_string_pretty(&output).unwrap()
    };
    println!("{rendered}");
}

fn print_examples(topic: Option<&str>) {
    let examples = threadmoth::metadata::examples(topic);
    if examples.is_empty() {
        eprintln!("no example topic matched");
        std::process::exit(1);
    }
    println!("{}", serde_json::to_string_pretty(&examples).unwrap());
}

fn run_benchmark(args: BenchmarkArgs) {
    if args.torture {
        std::process::exit(threadmoth::torture::run(args.json));
    }
    let profile = if args.quick {
        BenchmarkProfile::Quick
    } else if args.tough {
        BenchmarkProfile::Tough
    } else {
        args.profile.unwrap_or(BenchmarkProfile::Standard)
    };
    let profile = match profile {
        BenchmarkProfile::Quick => threadmoth::benchmark::Profile::Quick,
        BenchmarkProfile::Standard => threadmoth::benchmark::Profile::Standard,
        BenchmarkProfile::Tough => threadmoth::benchmark::Profile::Tough,
    };
    std::process::exit(threadmoth::benchmark::run(profile, args.json));
}

fn run_help(args: HelpArgs) {
    if let Some(term) = args.find {
        let matches = threadmoth::metadata::find_help(&term);
        if matches.is_empty() {
            eprintln!("no help matched '{term}'");
            std::process::exit(1);
        }
        for (name, description) in matches {
            println!("{name}: {description}");
        }
        return;
    }

    let mut root = Cli::command();
    if let Some(command) = args.command {
        if let Some(subcommand) = root.find_subcommand_mut(&command) {
            subcommand.print_long_help().unwrap();
            println!();
            return;
        }
        if let Some(text) = threadmoth::metadata::command_help(&command) {
            println!("threadmoth help {command}\n\n{text}");
            return;
        }
        eprintln!("unknown command: {command}");
        std::process::exit(1);
    }
    root.print_long_help().unwrap();
    println!();
}

fn print_explain(code: &str, json_output: bool) {
    let Some(reason) = threadmoth::metadata::reason(code) else {
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

fn run_suggest(args: SuggestArgs) {
    if let Some(source) = args.from_refusal {
        let input = if source == "-" {
            read_json_argument(None)
        } else {
            match read_request_input(Some(Path::new(&source))) {
                Ok(input) => input,
                Err(RequestInputError::TooLarge(actual)) => {
                    eprintln!(
                        "refusal certificate exceeds {MAX_REQUEST_BYTES} bytes (actual: {actual})"
                    );
                    std::process::exit(2);
                }
                Err(RequestInputError::Io(error)) => {
                    eprintln!("refusal certificate read failed: {error}");
                    std::process::exit(3);
                }
            }
        };
        let certificate: Certificate = serde_json::from_str(&input).unwrap_or_else(|error| {
            eprintln!("invalid refusal certificate: {error}");
            std::process::exit(2);
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&threadmoth::metadata::refusal_recovery(&certificate))
                .unwrap()
        );
        return;
    }

    let path = args.path.expect("clap requires path unless --from-refusal");
    let path = path.to_string_lossy();
    let root = env::current_dir().unwrap_or_else(|_| ".".into());
    let ws = Workspace::new(root).unwrap_or_else(|e| {
        eprintln!("workspace initialization failed: {e}");
        std::process::exit(3)
    });
    let bytes = ws.read_file(path.as_ref()).ok();
    let suggestion = threadmoth::metadata::suggest(
        path.as_ref(),
        args.goal.as_deref(),
        args.at.as_deref(),
        &args.mode,
        bytes.as_deref(),
    );
    println!("{}", serde_json::to_string_pretty(&suggestion).unwrap());
}

fn run_inspect(path: &Path) {
    let path = path.to_string_lossy();
    let root = env::current_dir().unwrap_or_else(|_| ".".into());
    let ws = match Workspace::new(root) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("workspace initialization failed: {e}");
            std::process::exit(3);
        }
    };
    match ws.read_file(path.as_ref()) {
        Ok(bytes) => {
            let newline = if bytes.windows(2).any(|w| w == b"\r\n") {
                "crlf"
            } else if bytes.contains(&b'\n') {
                "lf"
            } else {
                "none"
            };
            let out = serde_json::json!({
                "protocol_version": PROTOCOL_VERSION,
                "file_path": path,
                "bytes": bytes.len(),
                "sha256": threadmoth::engine::compute_sha256(&bytes),
                "encoding": if bytes.starts_with(&[0xef, 0xbb, 0xbf]) { "utf8_bom" } else { "utf8" },
                "newline_profile": newline,
                "final_newline": bytes.ends_with(b"\n")
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        Err(e) => {
            eprintln!("inspect failed: {e}");
            std::process::exit(2);
        }
    }
}

fn run_schema(args: SchemaArgs) {
    let out = threadmoth::metadata::schema(args.scope.as_deref());
    if args.json && !args.pretty {
        println!("{}", serde_json::to_string(&out).unwrap());
    } else {
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    }
}

fn run_doctor() {
    let root = env::current_dir().unwrap_or_else(|_| ".".into());
    let workspace = if Workspace::new(root).is_ok() {
        "ready"
    } else {
        "unavailable"
    };
    let providers = threadmoth::metadata::provider_metadata()
        .into_iter()
        .map(|provider| provider.name)
        .collect::<Vec<_>>()
        .join(" ");
    let shell = detected_shell();
    let path_status = current_exe_on_path();
    println!(
        "threadmoth doctor\nversion: {THREADMOTH_VERSION}\nos: {}\narch: {}\nworkspace: {workspace}\nprotocol: {PROTOCOL_VERSION}\nproviders: {providers}\ntransport: stdin/stdout mcp/stdio\ncommit: staged atomic replacement; recovery journal available\nshell: {shell}\npath: {}\ncompletion: available (threadmoth completions {shell})\nmanpage: available (threadmoth manpage)",
        env::consts::OS,
        env::consts::ARCH,
        if path_status {
            "configured"
        } else {
            "current executable directory not detected on PATH"
        },
    );
}

fn detected_shell() -> &'static str {
    if env::var_os("PSModulePath").is_some() && env::consts::OS == "windows" {
        return "powershell";
    }
    let shell = env::var("SHELL").unwrap_or_default().to_ascii_lowercase();
    if shell.contains("zsh") {
        "zsh"
    } else if shell.contains("fish") {
        "fish"
    } else {
        "bash"
    }
}

fn current_exe_on_path() -> bool {
    let Ok(exe) = env::current_exe() else {
        return false;
    };
    let Some(parent) = exe.parent() else {
        return false;
    };
    env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).any(|entry| entry == parent))
        .unwrap_or(false)
}

fn run_completions(shell: CompletionShell) {
    let mut command = Cli::command();
    generate(shell.into(), &mut command, "threadmoth", &mut io::stdout());
}

fn run_manpage(output: Option<&Path>) {
    let man = clap_mangen::Man::new(Cli::command());
    match output {
        Some(path) => {
            let mut file = fs::File::create(path).unwrap_or_else(|e| {
                eprintln!("cannot create manpage {}: {e}", path.display());
                std::process::exit(3)
            });
            man.render(&mut file).unwrap_or_else(|e| {
                eprintln!("cannot render manpage: {e}");
                std::process::exit(3)
            });
        }
        None => {
            let mut stdout = io::stdout();
            man.render(&mut stdout).unwrap_or_else(|e| {
                eprintln!("cannot render manpage: {e}");
                std::process::exit(3)
            });
            stdout.flush().ok();
        }
    }
}

fn read_json_argument(path: Option<&Path>) -> String {
    match read_request_input(path) {
        Ok(value) => value,
        Err(RequestInputError::TooLarge(actual)) => {
            eprintln!("input exceeds {MAX_REQUEST_BYTES} bytes (actual: {actual})");
            std::process::exit(2);
        }
        Err(RequestInputError::Io(error)) => {
            eprintln!("request read failed: {error}");
            std::process::exit(3);
        }
    }
}

enum RequestInputError {
    Io(io::Error),
    TooLarge(usize),
}

fn read_request_input(path: Option<&Path>) -> Result<String, RequestInputError> {
    let mut bytes = Vec::new();
    let reader: Box<dyn Read> = match path {
        Some(path) => Box::new(fs::File::open(path).map_err(RequestInputError::Io)?),
        None => Box::new(io::stdin()),
    };
    reader
        .take((MAX_REQUEST_BYTES as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(RequestInputError::Io)?;
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(RequestInputError::TooLarge(bytes.len()));
    }
    String::from_utf8(bytes)
        .map_err(|error| RequestInputError::Io(io::Error::new(io::ErrorKind::InvalidData, error)))
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

fn empty_transaction_certificate(reason: RefusalReason) -> TransactionCertificate {
    TransactionCertificate {
        protocol_version: PROTOCOL_VERSION.into(),
        transaction_id: String::new(),
        outcome: Outcome::Refused,
        certificates: Vec::new(),
        rollback_state: "not_started".into(),
        transaction_guarantee: "not_committed".into(),
        refusal_reason: Some(reason.clone()),
        failure_reason: None,
        reason_code: Some(reason.code().into()),
    }
}
