#![forbid(unsafe_code)]

use schemars::schema_for;
use std::{
    env, fs,
    io::{self, Read},
};
use suture::{
    pipeline::execute_request,
    protocol::{
        Certificate, CommitGuarantee, FailureReason, Outcome, PreservationFacts, RefusalReason,
        Request, StructuralValidation, PROTOCOL_VERSION,
    },
    workspace::Workspace,
};

fn help() {
    println!("suture {PROTOCOL_VERSION} - deterministic, source-preserving file mutation\n\nUSAGE:\n  suture apply [--request FILE]\n  suture dry-run [--request FILE]\n  suture schema\n  suture doctor\n  suture --version");
}
fn empty_cert(reason: RefusalReason) -> Certificate {
    Certificate {
        protocol_version: PROTOCOL_VERSION.into(),
        outcome: Outcome::Refused,
        file_path: String::new(),
        provider: "request".into(),
        provider_version: "parser".into(),
        expected_cardinality: Default::default(),
        observed_cardinality: None,
        pre_hash: String::new(),
        post_hash: None,
        changed_ranges: Vec::new(),
        diff_summary: None,
        diff_truncated: false,
        structural_validation: StructuralValidation::NotApplicable,
        preservation: PreservationFacts::default(),
        commit: CommitGuarantee::default(),
        refusal_reason: Some(reason),
        failure_reason: None,
        diagnostics: Vec::new(),
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
    match command {
        "apply" | "dry-run" => {
            let dry = command == "dry-run";
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
        "schema" => {
            let out = serde_json::json!({"$schema":"https://json-schema.org/draft/2020-12/schema","title":"Suture v0.1 Protocol Schemas","protocol_version":PROTOCOL_VERSION,"request":schema_for!(Request),"certificate":schema_for!(Certificate)});
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        "doctor" => {
            let root = env::current_dir().unwrap_or_else(|_| ".".into());
            println!("suture doctor\nos: {}\narch: {}\nworkspace: {}\nproviders: text=ready json=strict-source-preserving toml=narrow-diff\ncommit: staged atomic replacement; metadata limits documented",env::consts::OS,env::consts::ARCH,match Workspace::new(root){Ok(_)=>"ready",Err(_)=>"unavailable"});
        }
        _ => {
            eprintln!("unknown command: {command}");
            help();
            std::process::exit(1)
        }
    }
}

#[allow(dead_code)]
fn _failure_type_is_linked(_: FailureReason) {}
