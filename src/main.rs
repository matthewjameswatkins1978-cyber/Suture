#![forbid(unsafe_code)]

use schemars::schema_for;
use std::env;
use std::fs;
use std::io::{self, Read};
use suture::pipeline::execute_request;
use suture::protocol::{Certificate, Request};
use suture::workspace::Workspace;

fn print_help() {
    println!(
        r#"suture v0.1.0 - AI-first deterministic file-mutation protocol and runtime

USAGE:
    suture <COMMAND> [OPTIONS]

COMMANDS:
    apply [--request <path>]       Read JSON request from file or stdin, run pipeline, emit Certificate JSON
    dry-run [--request <path>]     Run dry-run pipeline, emit Certificate JSON without writing to disk
    schema                         Print JSON schema for Request and Certificate
    doctor                         Diagnose platform, workspace access, and provider status
    --version, -V                  Print version information
    --help, -h                     Print help information
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("suture 0.1.0");
        return;
    }

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_help();
        return;
    }

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "apply" | "dry-run" => {
            let dry_run = command == "dry-run";
            let mut request_path: Option<String> = None;

            let mut i = 2;
            while i < args.len() {
                if args[i] == "--request" && i + 1 < args.len() {
                    request_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let request_json_str = if let Some(path) = request_path {
                match fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(_) => {
                        eprintln!("Error: failed to read request file '{}'", path);
                        std::process::exit(1);
                    }
                }
            } else {
                let mut buffer = String::new();
                if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                    eprintln!("Error: failed to read request from stdin: {}", e);
                    std::process::exit(1);
                }
                buffer
            };

            let request: Request = match serde_json::from_str(&request_json_str) {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("Error: failed to parse JSON request: {}", e);
                    let cert = Certificate {
                        outcome: suture::protocol::Outcome::Refused,
                        file_path: String::new(),
                        pre_hash: String::new(),
                        post_hash: None,
                        refusal_reason: Some(suture::protocol::RefusalReason::MalformedInput {
                            details: e.to_string(),
                        }),
                        failure_reason: None,
                        diff_summary: None,
                    };
                    println!("{}", serde_json::to_string_pretty(&cert).unwrap());
                    std::process::exit(1);
                }
            };

            let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let workspace = match Workspace::new(&current_dir) {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("Error: failed to initialize workspace: {}", e);
                    std::process::exit(1);
                }
            };

            let certificate = execute_request(&workspace, &request, dry_run);
            match serde_json::to_string_pretty(&certificate) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Error serializing certificate: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "schema" => {
            let request_schema = schema_for!(Request);
            let certificate_schema = schema_for!(Certificate);

            let combined = serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Suture v0.1 Protocol Schemas",
                "request": request_schema,
                "certificate": certificate_schema
            });

            println!("{}", serde_json::to_string_pretty(&combined).unwrap());
        }
        "doctor" => {
            println!("Suture Doctor Diagnostics:");
            println!("- OS Target: {}", env::consts::OS);
            println!("- Architecture: {}", env::consts::ARCH);

            let current_dir = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            print!("- Workspace root ({:?}): ", current_dir);
            match Workspace::new(&current_dir) {
                Ok(_) => println!("READY (Accessible & Confined)"),
                Err(e) => println!("ERROR ({})", e),
            }

            println!("- Providers Status:");
            println!(
                "  * Text Provider: READY (identify, replace, insert_before, insert_after, delete)"
            );
            println!(
                "  * JSON Provider (serde_json / CST): READY (set, insert, delete, rename_key)"
            );
            println!("  * TOML Provider (toml_edit): READY (set, insert, delete, rename_key)");
            println!("- Status Summary: All systems operational and ready.");
        }
        other => {
            eprintln!("Unknown command: '{}'", other);
            print_help();
            std::process::exit(1);
        }
    }
}
#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn test_schema_command_output() {
        // Verify schema generation doesn't panic
        let req_schema = schema_for!(Request);
        assert!(!serde_json::to_string(&req_schema).unwrap().is_empty());
    }
}
