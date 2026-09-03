#![forbid(unsafe_code)]

//! The public knowledge base for Suture.  CLI discovery surfaces are views of
//! this module; they must not grow separate hand-written descriptions of the
//! protocol.

use crate::engine::compute_sha256;
use crate::lifecycle::FileOperation;
use crate::path::PathNamespace;
use crate::pattern::PatternOperation;
use crate::protocol::{
    Cardinality, EffectBudget, OperationPayload, Request, TransactionRequest, MAX_FILE_BYTES,
    MAX_REQUEST_BYTES, MAX_TRANSACTION_REQUESTS, PROTOCOL_VERSION,
};
use crate::provider::code::CodeOperation;
use crate::provider::dotenv::DotenvOperation;
use crate::provider::json::JsonOperation;
use crate::provider::patch::PatchOperation;
use crate::provider::text::TextOperation;
use crate::provider::toml::{TomlOperation, TomlValueWrapper};
use crate::provider::yaml::YamlOperation;
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OperationMetadata {
    pub name: &'static str,
    pub purpose: &'static str,
    pub required_selector: &'static str,
    pub default_cardinality: &'static str,
    pub idempotent: bool,
    pub effect: &'static str,
    pub read_only: bool,
    pub previewable: bool,
    pub transactional: bool,
    pub recoverable: bool,
    pub local_only: bool,
    pub preservation: Vec<&'static str>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProviderMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub operations: Vec<&'static str>,
    pub selectors: Vec<&'static str>,
    pub preservation: Vec<&'static str>,
    pub encodings: Vec<&'static str>,
    pub transaction_support: &'static str,
    pub durable_anchor_support: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReasonMetadata {
    pub code: &'static str,
    pub meaning: &'static str,
    pub why_refused: &'static str,
    pub recovery_category: &'static str,
    pub retry_unchanged: bool,
    pub relevant_commands: Vec<&'static str>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResourceLimits {
    pub max_request_bytes: usize,
    pub max_transaction_requests: usize,
    pub max_diagnostic_bytes: usize,
    pub max_pattern_bytes: usize,
    pub max_file_bytes: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionCapabilities {
    pub single_file: bool,
    pub multi_file: bool,
    pub rollback: bool,
    pub crash_recovery: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapabilityManifest {
    pub format_version: &'static str,
    pub protocol_versions: Vec<&'static str>,
    pub protocol_version: &'static str,
    pub suture_version: &'static str,
    pub capability_set_id: String,
    pub providers: Vec<ProviderMetadata>,
    pub operations: Vec<OperationMetadata>,
    pub selectors: Vec<&'static str>,
    pub preservation_guarantees: Vec<&'static str>,
    pub encodings: Vec<&'static str>,
    pub path_namespaces: Vec<&'static str>,
    pub code_languages: Vec<&'static str>,
    pub guard_modes: Vec<&'static str>,
    pub transaction_capabilities: TransactionCapabilities,
    pub resource_limits: ResourceLimits,
    pub effect_budget_dimensions: Vec<&'static str>,
    pub reason_codes: Vec<ReasonMetadata>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Example {
    pub topic: &'static str,
    pub intent: &'static str,
    pub request: Value,
    pub representative_response: Value,
    pub safety_property: &'static str,
}

#[derive(Serialize, Clone, Debug)]
pub struct Suggestion {
    pub provider: String,
    pub detection_basis: String,
    pub goal: Option<String>,
    pub mode: String,
    pub recommended_operation: Option<String>,
    pub rationale: String,
    pub request_template: Option<Value>,
    pub guarantees: Vec<String>,
    pub budget_defaults: EffectBudget,
    pub alternatives: Vec<Value>,
    pub blocked_reasons: Vec<String>,
    pub capability_set_id: String,
}

pub fn operation_metadata() -> Vec<OperationMetadata> {
    let common = vec![
        "unrelated_bytes",
        "utf8",
        "newline_profile",
        "final_newline",
    ];
    vec![
        op(
            "replace",
            "Replace an exact target.",
            "literal",
            false,
            "mixed",
            common.clone(),
        ),
        op(
            "insert_before",
            "Insert content before an exact target.",
            "literal",
            false,
            "additive",
            common.clone(),
        ),
        op(
            "insert_after",
            "Insert content after an exact target.",
            "literal",
            false,
            "additive",
            common.clone(),
        ),
        op(
            "delete",
            "Delete an exact target.",
            "literal",
            false,
            "destructive",
            common.clone(),
        ),
        op(
            "move",
            "Move one exact target before another.",
            "literal",
            false,
            "mixed",
            common.clone(),
        ),
        op(
            "ensure_present",
            "Ensure desired content exists; replay is safe.",
            "literal",
            true,
            "additive",
            common.clone(),
        ),
        op(
            "ensure_absent",
            "Ensure a target is absent; replay is safe.",
            "literal",
            true,
            "destructive",
            common.clone(),
        ),
        op(
            "set",
            "Set a selected value or exact target.",
            "provider_selector",
            false,
            "mixed",
            common.clone(),
        ),
        op(
            "unset",
            "Remove a selected value or exact target.",
            "provider_selector",
            true,
            "destructive",
            common.clone(),
        ),
        op(
            "rename",
            "Rename a selected key or exact target.",
            "provider_selector",
            false,
            "mixed",
            common,
        ),
        op(
            "insert",
            "Insert a structured member or array item.",
            "json_pointer",
            false,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "rename_key",
            "Rename one structured key.",
            "json_pointer",
            false,
            "mixed",
            vec!["unrelated_bytes"],
        ),
        op(
            "replace_section",
            "Replace one bounded Markdown section.",
            "heading",
            false,
            "mixed",
            vec!["unrelated_bytes"],
        ),
        op(
            "ensure_section",
            "Ensure one Markdown section exists.",
            "heading",
            true,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "delete_section",
            "Delete one bounded Markdown section.",
            "heading",
            false,
            "destructive",
            vec!["unrelated_bytes"],
        ),
        op(
            "insert_after_heading",
            "Insert content after one Markdown heading.",
            "heading",
            false,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "replace_list_item",
            "Replace one bounded Markdown list item.",
            "list_item",
            false,
            "mixed",
            vec!["unrelated_bytes"],
        ),
        op(
            "ensure_list_item",
            "Ensure one Markdown list item exists.",
            "list_item",
            true,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "delete_list_item",
            "Delete one bounded Markdown list item.",
            "list_item",
            false,
            "destructive",
            vec!["unrelated_bytes"],
        ),
        op(
            "replace_fenced_block",
            "Replace one fenced Markdown block body.",
            "fenced_region",
            false,
            "mixed",
            vec!["unrelated_bytes"],
        ),
        op(
            "replace_node",
            "Replace one parsed syntax node.",
            "syntax_node_text",
            false,
            "mixed",
            vec!["unrelated_bytes"],
        ),
        op(
            "insert_before_node",
            "Insert content before one parsed syntax node.",
            "syntax_node_text",
            false,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "insert_after_node",
            "Insert content after one parsed syntax node.",
            "syntax_node_text",
            false,
            "additive",
            vec!["unrelated_bytes"],
        ),
        op(
            "remove_node",
            "Remove one parsed syntax node.",
            "syntax_node_text",
            false,
            "destructive",
            vec!["unrelated_bytes"],
        ),
        op(
            "unified_diff",
            "Apply one exact unified diff.",
            "exact_preimage",
            false,
            "mixed",
            vec!["exact_context"],
        ),
        op(
            "create_file",
            "Create a missing file without overwriting.",
            "workspace_relative_path",
            false,
            "additive",
            vec!["identity_and_confinement"],
        ),
        op(
            "delete_file",
            "Delete a file guarded by identity.",
            "workspace_relative_path",
            false,
            "destructive",
            vec!["identity_and_confinement"],
        ),
        op(
            "rename_file",
            "Rename a file with source and destination guards.",
            "workspace_relative_path",
            false,
            "mixed",
            vec!["identity_and_confinement"],
        ),
        op(
            "move_file",
            "Move a file with source and destination guards.",
            "workspace_relative_path",
            false,
            "mixed",
            vec!["identity_and_confinement"],
        ),
    ]
}

fn op(
    name: &'static str,
    purpose: &'static str,
    selector: &'static str,
    idempotent: bool,
    effect: &'static str,
    preservation: Vec<&'static str>,
) -> OperationMetadata {
    OperationMetadata {
        name,
        purpose,
        required_selector: selector,
        default_cardinality: "exactly_one",
        idempotent,
        effect,
        read_only: false,
        previewable: true,
        transactional: true,
        recoverable: true,
        local_only: true,
        preservation,
    }
}

pub fn provider_metadata() -> Vec<ProviderMetadata> {
    let text_ops = vec![
        "replace",
        "insert_before",
        "insert_after",
        "delete",
        "move",
        "ensure_present",
        "ensure_absent",
        "set",
        "unset",
        "rename",
    ];
    vec![
        provider(
            "text",
            "text-byte-v1",
            text_ops,
            vec!["literal"],
            "utf8, BOM, newline profile",
            true,
        ),
        provider(
            "json",
            "json-source-v1",
            vec![
                "set",
                "insert",
                "delete",
                "rename_key",
                "ensure_present",
                "ensure_absent",
                "unset",
                "rename",
            ],
            vec!["json_pointer"],
            "source ranges, unrelated bytes",
            true,
        ),
        provider(
            "jsonc",
            "jsonc-source-v1",
            vec![
                "set",
                "insert",
                "delete",
                "rename_key",
                "ensure_present",
                "ensure_absent",
                "unset",
                "rename",
            ],
            vec!["json_pointer"],
            "comments and source ranges",
            true,
        ),
        provider(
            "toml",
            "toml-edit-narrow-v1",
            vec![
                "set",
                "insert",
                "delete",
                "rename_key",
                "ensure_present",
                "ensure_absent",
                "unset",
                "rename",
            ],
            vec!["dotted_key"],
            "comments, ordering where supported",
            true,
        ),
        provider(
            "yaml",
            "yaml-conservative-source-v1",
            vec!["set", "ensure_present", "delete", "ensure_absent"],
            vec!["top_level_scalar_key"],
            "comments for supported scalar forms",
            true,
        ),
        provider(
            "markdown",
            "markdown-regions-v2",
            vec![
                "replace_section",
                "ensure_section",
                "delete_section",
                "insert_after_heading",
                "replace_list_item",
                "ensure_list_item",
                "delete_list_item",
                "replace_fenced_block",
            ],
            vec!["heading", "section", "fenced_region"],
            "unrelated markdown bytes",
            true,
        ),
        provider(
            "dotenv",
            "dotenv-lines-v1",
            vec!["set", "unset", "ensure_present"],
            vec!["key"],
            "comments and unrelated lines",
            true,
        ),
        provider(
            "pattern",
            "regex-automata-bounded-v1",
            vec!["replace", "delete", "ensure_absent"],
            vec!["bounded_pattern"],
            "unrelated bytes",
            true,
        ),
        provider(
            "patch",
            "unified-diff-strict-v1",
            vec!["unified_diff"],
            vec!["exact_preimage", "exact_context"],
            "exact patch context",
            true,
        ),
        provider(
            "code",
            "tree-sitter-node-v1",
            vec![
                "replace_node",
                "insert_before_node",
                "insert_after_node",
                "remove_node",
            ],
            vec!["syntax_node_text", "syntax_node_kind"],
            "unrelated source bytes where ranges permit",
            true,
        ),
        provider(
            "filesystem",
            "lifecycle-checked-v1",
            vec!["create_file", "delete_file", "rename_file", "move_file"],
            vec!["workspace_relative_path"],
            "identity and confinement",
            true,
        ),
    ]
}

fn provider(
    name: &'static str,
    version: &'static str,
    operations: Vec<&'static str>,
    selectors: Vec<&'static str>,
    preservation: &'static str,
    durable: bool,
) -> ProviderMetadata {
    ProviderMetadata {
        name,
        version,
        operations,
        selectors,
        preservation: vec![preservation],
        encodings: vec!["utf8", "utf8_bom"],
        // Lifecycle requests are guarded and recoverable as one-shot
        // operations, but are not yet composable with the content transaction
        // journal. Do not advertise a capability that the pipeline refuses.
        transaction_support: if name == "filesystem" {
            "single_file"
        } else {
            "single_file_and_multi_file"
        },
        durable_anchor_support: durable,
    }
}

pub fn reason_metadata() -> Vec<ReasonMetadata> {
    let mut out = Vec::new();
    macro_rules! reason {
        ($code:literal, $meaning:literal, $why:literal, $cat:literal, $retry:expr, [$($cmd:literal),* $(,)?]) => {
            out.push(ReasonMetadata { code: $code, meaning: $meaning, why_refused: $why, recovery_category: $cat, retry_unchanged: $retry, relevant_commands: vec![$($cmd),*] });
        };
    }
    reason!(
        "CARDINALITY_MISMATCH",
        "The observed match count did not equal the requested count.",
        "Suture will not choose an unintended target.",
        "narrow_target",
        false,
        ["inspect", "suggest", "preview"]
    );
    reason!(
        "TARGET_NOT_FOUND",
        "No exact target was found.",
        "The requested precondition is absent.",
        "correct_selector",
        false,
        ["inspect", "suggest"]
    );
    reason!(
        "TARGET_AMBIGUOUS",
        "More than one plausible target was found.",
        "Choosing the first match would be unsafe.",
        "choose_candidate",
        false,
        ["suggest", "preview"]
    );
    reason!(
        "STALE_IDENTITY",
        "The accepted source or guarded region changed.",
        "The request no longer describes the state being mutated.",
        "refresh_guard",
        false,
        ["inspect", "preview"]
    );
    reason!(
        "EFFECT_BUDGET_EXCEEDED",
        "The prepared effect exceeds a caller limit.",
        "No bytes are written when a declared budget fails.",
        "narrow_or_confirm_budget",
        false,
        ["suggest", "preview"]
    );
    reason!(
        "RESOURCE_LIMIT_EXCEEDED",
        "The request or observed file exceeds a built-in safety limit.",
        "Suture bounds parser and memory exposure before mutation.",
        "reduce_input_or_split_work",
        false,
        ["capabilities", "inspect"]
    );
    reason!(
        "WORKSPACE_ESCAPE",
        "The path or scope leaves the workspace.",
        "Suture only mutates confined workspace state.",
        "correct_path",
        false,
        ["inspect", "suggest"]
    );
    reason!(
        "SYMLINK_ESCAPE",
        "A symlink or reparse path escapes confinement.",
        "Path spelling cannot override physical containment.",
        "correct_path",
        false,
        ["inspect"]
    );
    reason!(
        "PROVIDER_UNSUPPORTED",
        "No advertised provider capability supports the request.",
        "Suture never silently falls back to another provider.",
        "choose_supported_provider",
        false,
        ["capabilities", "suggest"]
    );
    reason!(
        "PRESERVATION_UNAVAILABLE",
        "The requested source-preservation guarantee cannot be proved.",
        "Suture refuses lossy rewriting.",
        "choose_guarantee",
        false,
        ["capabilities", "suggest"]
    );
    reason!(
        "INVALID_STRUCTURE",
        "The candidate failed provider validation.",
        "A syntactically invalid result cannot be committed.",
        "correct_request",
        false,
        ["schema", "suggest"]
    );
    reason!(
        "ENCODING_UNSUPPORTED",
        "The file encoding is not safely supported.",
        "Suture never guesses a legacy encoding.",
        "convert_explicitly",
        false,
        ["inspect"]
    );
    reason!(
        "TRANSACTION_CONFLICT",
        "A transaction member could not be prepared coherently.",
        "Partial transaction application is not implicit.",
        "split_or_correct_transaction",
        false,
        ["suggest", "preview"]
    );
    reason!(
        "OVERLAPPING_EDITS",
        "Prepared edits overlap.",
        "Overlapping byte ranges do not have an unambiguous result.",
        "split_operations",
        false,
        ["suggest", "preview"]
    );
    reason!(
        "GENERATED_FILE_REQUIRES_OPT_IN",
        "The target appears generated or marked do-not-edit.",
        "Generated state is protected by default.",
        "explicit_opt_in",
        false,
        ["suggest"]
    );
    reason!(
        "DESTINATION_EXISTS",
        "The requested file destination already exists.",
        "Lifecycle operations never overwrite silently.",
        "choose_destination",
        false,
        ["inspect", "suggest"]
    );
    reason!(
        "INVALID_INPUT",
        "The request could not be parsed or contains invalid input.",
        "A malformed request cannot be interpreted safely.",
        "correct_request",
        false,
        ["schema", "examples"]
    );
    reason!(
        "LOSSY_OPERATION_REQUIRES_OPT_IN",
        "The operation could change source details that were not authorized.",
        "Suture does not silently accept lossy rewriting.",
        "choose_guarantee",
        false,
        ["capabilities", "suggest"]
    );
    reason!(
        "OPERATION_UNSUPPORTED",
        "The selected provider does not implement this operation.",
        "Provider selection is explicit and never falls back silently.",
        "choose_supported_operation",
        false,
        ["capabilities", "schema"]
    );
    reason!(
        "PROTOCOL_UNSUPPORTED",
        "The request protocol version is not supported by this binary.",
        "Suture will not reinterpret a different contract.",
        "upgrade_or_use_matching_binary",
        false,
        ["capabilities", "schema"]
    );
    reason!(
        "REFUSED",
        "The request was refused without a more specific public reason.",
        "The request did not meet a safe execution precondition.",
        "inspect_certificate",
        false,
        ["explain", "suggest"]
    );
    reason!(
        "BINARY_INPUT",
        "The target contains binary data and is outside content-mutation scope.",
        "Suture does not guess how binary bytes should be edited.",
        "choose_text_target",
        false,
        ["inspect"]
    );
    reason!(
        "PATH_UNMAPPABLE",
        "The declared path namespace cannot be mapped to this execution environment.",
        "Suture will not guess a drive, mount, or distribution mapping.",
        "correct_path_namespace",
        false,
        ["capabilities", "inspect"]
    );
    reason!(
        "COMMIT_FAILED",
        "The prepared candidate could not be committed.",
        "The certificate reports commit or recovery state explicitly.",
        "recover",
        true,
        ["recover"]
    );
    reason!(
        "POST_COMMIT_VERIFICATION_FAILED",
        "The landed bytes differ from the verified candidate.",
        "The mutation cannot be reported as successful without matching evidence.",
        "recover",
        false,
        ["recover", "inspect"]
    );
    reason!(
        "IO_ERROR",
        "The workspace could not be read or accessed.",
        "The requested state was not safely observable.",
        "repair_workspace",
        true,
        ["doctor", "recover"]
    );
    reason!(
        "INTERNAL_INVARIANT",
        "An internal Suture invariant failed while preparing a candidate.",
        "Suture fails closed rather than committing an unverified result.",
        "report_defect",
        false,
        ["inspect", "recover"]
    );
    reason!(
        "FAILED",
        "The operation failed without a more specific public reason.",
        "The certificate contains the failure details and recovery state.",
        "inspect_certificate",
        false,
        ["recover"]
    );
    out
}

pub fn capabilities() -> CapabilityManifest {
    let providers = provider_metadata();
    let operations = operation_metadata();
    let reason_codes = reason_metadata();
    let value = json!({
        "format_version": "1.1",
        "protocol_versions": [PROTOCOL_VERSION],
        "protocol_version": PROTOCOL_VERSION,
        "suture_version": env!("CARGO_PKG_VERSION"),
        "providers": providers,
        "operations": operations,
        "selectors": ["literal", "json_pointer", "dotted_key", "top_level_scalar_key", "heading", "bounded_pattern", "syntax_node_text", "syntax_node_kind", "workspace_relative_path"],
        "preservation_guarantees": ["unrelated_bytes", "utf8", "utf8_bom", "lf", "crlf", "final_newline", "comments_where_supported"],
        "encodings": ["utf8", "utf8_bom"],
        "path_namespaces": ["native", "windows", "wsl", "posix"],
        "code_languages": ["javascript", "typescript", "jsx", "tsx", "python", "rust", "go"],
        "guard_modes": ["immediate", "strict_snapshot", "region_snapshot", "structural_snapshot"],
        "transaction_capabilities": {"single_file": true, "multi_file": true, "rollback": true, "crash_recovery": true},
        "resource_limits": {"max_request_bytes": MAX_REQUEST_BYTES, "max_transaction_requests": MAX_TRANSACTION_REQUESTS, "max_diagnostic_bytes": 4096, "max_pattern_bytes": 8192, "max_file_bytes": MAX_FILE_BYTES},
        "effect_budget_dimensions": ["max_files", "max_matches", "max_changed_regions", "max_changed_lines", "max_changed_bytes", "allowed_path_prefixes"],
        "reason_codes": reason_codes
    });
    let capability_set_id = digest_without_id(&value);
    CapabilityManifest {
        format_version: "1.1",
        protocol_versions: vec![PROTOCOL_VERSION],
        protocol_version: PROTOCOL_VERSION,
        suture_version: env!("CARGO_PKG_VERSION"),
        capability_set_id,
        providers: provider_metadata(),
        operations: operation_metadata(),
        selectors: vec![
            "literal",
            "json_pointer",
            "dotted_key",
            "top_level_scalar_key",
            "heading",
            "bounded_pattern",
            "syntax_node_text",
            "syntax_node_kind",
            "workspace_relative_path",
        ],
        preservation_guarantees: vec![
            "unrelated_bytes",
            "utf8",
            "utf8_bom",
            "lf",
            "crlf",
            "final_newline",
            "comments_where_supported",
        ],
        encodings: vec!["utf8", "utf8_bom"],
        path_namespaces: vec!["native", "windows", "wsl", "posix"],
        code_languages: vec![
            "javascript",
            "typescript",
            "jsx",
            "tsx",
            "python",
            "rust",
            "go",
        ],
        guard_modes: vec![
            "immediate",
            "strict_snapshot",
            "region_snapshot",
            "structural_snapshot",
        ],
        transaction_capabilities: TransactionCapabilities {
            single_file: true,
            multi_file: true,
            rollback: true,
            crash_recovery: true,
        },
        resource_limits: ResourceLimits {
            max_request_bytes: MAX_REQUEST_BYTES,
            max_transaction_requests: MAX_TRANSACTION_REQUESTS,
            max_diagnostic_bytes: 4_096,
            max_pattern_bytes: 8_192,
            max_file_bytes: MAX_FILE_BYTES,
        },
        effect_budget_dimensions: vec![
            "max_files",
            "max_matches",
            "max_changed_regions",
            "max_changed_lines",
            "max_changed_bytes",
            "allowed_path_prefixes",
        ],
        reason_codes: reason_metadata(),
    }
}

pub fn capability_view(selector: Option<&str>) -> Value {
    let manifest = serde_json::to_value(capabilities()).expect("capabilities serialize");
    let Some(selector) = selector else {
        return manifest;
    };
    let mut view = manifest;
    if let Some((provider, operation)) = selector.split_once('.') {
        let provider_entries: Vec<Value> = view["providers"]
            .as_array()
            .into_iter()
            .flat_map(|providers| providers.iter())
            .filter(|entry| entry["name"] == provider)
            .cloned()
            .collect();
        let supported = provider_entries.first().is_some_and(|entry| {
            entry["operations"]
                .as_array()
                .is_some_and(|operations| operations.iter().any(|value| value == operation))
        });
        view["providers"] = provider_entries
            .into_iter()
            .map(|mut entry| {
                entry["selected_operation"] = Value::String(operation.into());
                entry["operation_supported"] = Value::Bool(supported);
                entry
            })
            .collect::<Vec<_>>()
            .into();
        if supported {
            if let Some(metadata) = operation_metadata()
                .into_iter()
                .find(|entry| entry.name == operation)
            {
                view["selected_operation"] =
                    serde_json::to_value(metadata).expect("operation serializes");
            }
        } else {
            view["selection_error"] = json!({
                "provider": provider,
                "operation": operation,
                "reason": "operation is not advertised for this provider"
            });
        }
    } else {
        view["providers"] = view["providers"]
            .as_array()
            .into_iter()
            .flat_map(|providers| providers.iter())
            .filter(|entry| entry["name"] == selector)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        view["selected_provider"] = Value::String(selector.into());
    }
    view
}

pub fn capabilities_for(path: &str, bytes: Option<&[u8]>) -> Value {
    let mut value = serde_json::to_value(capabilities()).expect("capabilities serialize");
    let (provider, basis, candidates) = detect_provider(path, bytes);
    value["target"] = json!({"path": path, "provider": provider, "detection_basis": basis, "candidates": candidates});
    if provider != "ambiguous" {
        value["providers"] = value["providers"]
            .as_array()
            .into_iter()
            .flat_map(|providers| providers.iter())
            .filter(|entry| entry["name"] == provider)
            .cloned()
            .collect::<Vec<_>>()
            .into();
    }
    value
}

fn digest_without_id(value: &Value) -> String {
    compute_sha256(
        serde_json::to_string(value)
            .expect("metadata serializes")
            .as_bytes(),
    )[..24]
        .into()
}

pub fn schema(scope: Option<&str>) -> Value {
    let mut document = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Suture 1.1 Protocol Schemas",
        "protocol_version": PROTOCOL_VERSION,
        "scope": scope.unwrap_or("all"),
        "request": schema_for!(Request),
        "response": schema_for!(crate::protocol::Certificate),
        "certificate": schema_for!(crate::protocol::Certificate),
        "transaction_request": schema_for!(TransactionRequest),
        "transaction_certificate": schema_for!(crate::protocol::TransactionCertificate),
        "metadata": capabilities(),
    });
    let schema_id = digest_without_id(&document);
    document["schema_id"] = Value::String(schema_id);
    document
}

pub fn examples(topic: Option<&str>) -> Vec<Example> {
    let all = vec![
        example(
            "exact-text-replacement",
            "Replace one exact token.",
            text_request(
                "src/config.txt",
                TextOperation::Replace {
                    target: "old".into(),
                    replacement: "new".into(),
                },
            ),
            "APPLIED",
            "Exactly-one cardinality prevents a wrong duplicate edit.",
        ),
        example(
            "idempotent-ensure-present",
            "Ensure a line exists and make replay safe.",
            text_request(
                "README.md",
                TextOperation::EnsurePresent {
                    content: "managed line".into(),
                },
            ),
            "NO_CHANGE or APPLIED",
            "Desired-state operations are safe to replay.",
        ),
        example(
            "json-structural-set",
            "Set a JSON value without reserializing the document.",
            json_request(
                "config.json",
                JsonOperation::Set {
                    path: "$.name".into(),
                    value: json!("new"),
                },
            ),
            "APPLIED",
            "The JSON provider targets a source range.",
        ),
        example(
            "toml-structural-set",
            "Set a TOML key while preserving supported source details.",
            toml_request(
                "Cargo.toml",
                TomlOperation::Set {
                    path: "package.name".into(),
                    value: TomlValueWrapper::String("suture".into()),
                },
            ),
            "APPLIED",
            "The provider validates the candidate before commit.",
        ),
        example(
            "preview",
            "Inspect a prepared mutation without writing.",
            text_request(
                "x.txt",
                TextOperation::Replace {
                    target: "a".into(),
                    replacement: "b".into(),
                },
            ),
            "APPLIED with dry_run commit",
            "Preview produces the same certificate shape without a write.",
        ),
        example(
            "safe-file-creation",
            "Create a file only when the destination is absent.",
            file_request(
                "new.txt",
                FileOperation::CreateFile {
                    expected_absent: true,
                    content: b"hello\n".to_vec(),
                },
            ),
            "APPLIED",
            "Creation uses an explicit no-overwrite precondition.",
        ),
        example(
            "safe-deletion",
            "Delete a file whose identity is known.",
            file_request(
                "old.txt",
                FileOperation::DeleteFile {
                    expected_hash: "SHA256_OF_CURRENT_FILE".into(),
                },
            ),
            "APPLIED",
            "Deletion is guarded by content identity.",
        ),
        example(
            "multi-operation-transaction",
            "Apply coherent operations to one file.",
            transaction_request(vec![
                text_request(
                    "x.txt",
                    TextOperation::Replace {
                        target: "one".into(),
                        replacement: "first".into(),
                    },
                ),
                text_request(
                    "x.txt",
                    TextOperation::Replace {
                        target: "two".into(),
                        replacement: "second".into(),
                    },
                ),
            ]),
            "APPLIED",
            "All operations resolve against one in-memory candidate.",
        ),
        example(
            "multi-file-transaction",
            "Stage several files before commit.",
            transaction_request(vec![
                text_request(
                    "a.txt",
                    TextOperation::Replace {
                        target: "a".into(),
                        replacement: "b".into(),
                    },
                ),
                text_request(
                    "b.txt",
                    TextOperation::EnsurePresent {
                        content: "managed".into(),
                    },
                ),
            ]),
            "APPLIED",
            "Preparation completes before any member is written.",
        ),
        example(
            "ambiguous-refusal",
            "Refuse a duplicate exact target.",
            text_request(
                "x.txt",
                TextOperation::Replace {
                    target: "duplicate".into(),
                    replacement: "new".into(),
                },
            ),
            "REFUSED / TARGET_AMBIGUOUS",
            "Suture returns candidates instead of choosing one.",
        ),
        example(
            "refusal-recovery",
            "Feed a refusal certificate back to discovery.",
            text_request(
                "x.txt",
                TextOperation::Replace {
                    target: "missing".into(),
                    replacement: "new".into(),
                },
            ),
            "REFUSED / TARGET_NOT_FOUND",
            "Use suggest --from-refusal to obtain corrected skeletons.",
        ),
        example(
            "effect-budget-refusal",
            "Reject a candidate that exceeds a caller limit.",
            text_request(
                "x.txt",
                TextOperation::Replace {
                    target: "old".into(),
                    replacement: "new".into(),
                },
            ),
            "REFUSED / EFFECT_BUDGET_EXCEEDED",
            "Budgets are checked before commit.",
        ),
        example(
            "strict-patch",
            "Apply an exact unified diff or refuse.",
            patch_request("x.txt"),
            "APPLIED",
            "Patch context is exact; fuzzy relocation is not used.",
        ),
    ];
    match topic {
        Some(wanted) => all
            .into_iter()
            .filter(|e| topic_matches(e.topic, wanted))
            .collect(),
        None => all,
    }
}

fn topic_matches(topic: &str, wanted: &str) -> bool {
    topic == wanted || topic.replace('-', "_") == wanted || topic.contains(wanted)
}

fn example<T: Serialize>(
    topic: &'static str,
    intent: &'static str,
    request: T,
    outcome: &'static str,
    safety: &'static str,
) -> Example {
    Example {
        topic,
        intent,
        request: serde_json::to_value(request).expect("example request serializes"),
        representative_response: json!({"outcome": outcome, "protocol_version": PROTOCOL_VERSION}),
        safety_property: safety,
    }
}

fn base_request(path: &str, operation: OperationPayload) -> Request {
    Request {
        version: PROTOCOL_VERSION.into(),
        request_id: format!("example-{path}"),
        allow_generated: false,
        file_path: path.into(),
        namespace: PathNamespace::Native,
        expected_pre_hash: None,
        region_guard: None,
        cardinality: Cardinality::ExactlyOne,
        budget: EffectBudget {
            max_files: Some(1),
            max_matches: Some(1),
            ..Default::default()
        },
        operation,
    }
}
fn text_request(path: &str, operation: TextOperation) -> Request {
    base_request(path, OperationPayload::Text(operation))
}
fn json_request(path: &str, operation: JsonOperation) -> Request {
    base_request(path, OperationPayload::Json(operation))
}
fn toml_request(path: &str, operation: TomlOperation) -> Request {
    base_request(path, OperationPayload::Toml(operation))
}
fn file_request(path: &str, operation: FileOperation) -> Request {
    base_request(path, OperationPayload::File(operation))
}
fn patch_request(path: &str) -> Request {
    base_request(
        path,
        OperationPayload::Patch(PatchOperation::UnifiedDiff {
            patch: "--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n-old\n+new\n".into(),
        }),
    )
}
fn transaction_request(requests: Vec<Request>) -> TransactionRequest {
    TransactionRequest {
        version: PROTOCOL_VERSION.into(),
        transaction_id: "example-transaction".into(),
        requests,
        budget: EffectBudget {
            max_files: Some(2),
            max_matches: Some(2),
            ..Default::default()
        },
    }
}

pub fn commands() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "capabilities",
            "Discover providers, operations, guarantees and limits.",
        ),
        (
            "examples",
            "See small, current, validated request examples.",
        ),
        (
            "schema",
            "Inspect the exact local protocol and schema fingerprint.",
        ),
        (
            "explain",
            "Understand a stable refusal or failure reason code.",
        ),
        (
            "inspect",
            "Read target identity and preservation facts without mutation.",
        ),
        ("preview", "Prepare and certify a mutation without writing."),
        (
            "mutate",
            "Prepare, verify, commit and certify one mutation.",
        ),
        ("transact", "Prepare and commit a guarded transaction."),
        (
            "recover",
            "Inspect and deterministically recover interrupted commits.",
        ),
        ("suggest", "Generate a safe request skeleton for a target."),
    ]
}

pub fn command_help(command: &str) -> Option<String> {
    let text = match command {
        "mutate" | "preview" => "Reads a JSON Request from stdin or --request FILE and emits one Certificate. preview never writes; mutate commits only after validation and budget checks.",
        "capabilities" => "Use capabilities [PROVIDER] [PROVIDER.OPERATION] or --for PATH; add --json --all for the complete machine manifest.",
        "examples" => "Use examples [TOPIC] to print current, schema-valid request patterns.",
        "schema" => "Use schema [request|response|PROVIDER|OPERATION] [--json] [--pretty] to inspect the local contract and schema_id.",
        "explain" => "Use explain REASON_CODE [--json] to get meaning, evidence interpretation and safe recovery guidance.",
        "suggest" => "Use suggest PATH [--goal GOAL] [--at SELECTOR] [--mode minimal|safe|full], or suggest --from-refusal CERTIFICATE.",
        "inspect" => "Read a workspace-relative target's identity, encoding and newline profile; it never mutates.",
        "transact" => "Reads a TransactionRequest and stages every member before commit; transaction-preview prepares without writing.",
        "recover" => "Inspect local recovery journals and complete or restore interrupted transactions with evidence.",
        _ => return None,
    };
    Some(text.into())
}

pub fn find_help(term: &str) -> Vec<(&'static str, &'static str)> {
    let term = term.to_ascii_lowercase();
    commands()
        .into_iter()
        .filter(|(name, description)| {
            name.contains(&term) || description.to_ascii_lowercase().contains(&term)
        })
        .collect()
}

pub fn reason(code: &str) -> Option<ReasonMetadata> {
    reason_metadata()
        .into_iter()
        .find(|entry| entry.code.eq_ignore_ascii_case(code))
}

pub fn detect_provider(path: &str, bytes: Option<&[u8]>) -> (String, String, Vec<String>) {
    let lower = path.to_ascii_lowercase();
    let extension = std::path::Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let known = match extension {
        "json" => Some(("json", "file extension .json")),
        "jsonc" => Some(("jsonc", "file extension .jsonc")),
        "toml" => Some(("toml", "file extension .toml")),
        "yaml" | "yml" => Some(("yaml", "file extension .yaml/.yml")),
        "md" | "markdown" => Some(("markdown", "file extension .md/.markdown")),
        "env" => Some(("dotenv", "file extension .env")),
        "js" | "jsx" | "ts" | "tsx" | "py" | "rs" | "go" => {
            Some(("code", "recognized code-file extension"))
        }
        _ => None,
    };
    if let Some((provider, basis)) = known {
        return (provider.into(), basis.into(), Vec::new());
    }
    if let Some(content) = bytes {
        let trimmed = String::from_utf8_lossy(content).trim_start().to_string();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            return (
                "ambiguous".into(),
                "content resembles structured data without a known extension".into(),
                vec!["json".into(), "jsonc".into(), "yaml".into()],
            );
        }
    }
    (
        "text".into(),
        "no more-specific provider was established".into(),
        Vec::new(),
    )
}

pub fn suggest(
    path: &str,
    goal: Option<&str>,
    at: Option<&str>,
    mode: &str,
    bytes: Option<&[u8]>,
) -> Suggestion {
    let (detected, basis, candidates) = detect_provider(path, bytes);
    // An ambiguous content-based detection is evidence, not permission to
    // choose the first provider. Keep the request template empty until the
    // caller explicitly selects a provider through the path/operation it
    // submits.
    let selected = if candidates.is_empty() {
        Some(detected.as_str())
    } else {
        None
    };
    let requested_goal = goal.map(str::to_ascii_lowercase);
    let goal_name = requested_goal.as_deref().unwrap_or("replace-text");
    let allowed_goal = matches!(
        goal_name,
        "replace-text"
            | "set-value"
            | "add-item"
            | "remove-item"
            | "rename"
            | "ensure-present"
            | "ensure-absent"
            | "move"
            | "create-file"
            | "delete-file"
            | "apply-patch"
            | "transact"
    );
    let mode = match mode {
        "minimal" | "safe" | "full" => mode,
        _ => "safe",
    };
    let budget = if mode == "safe" {
        EffectBudget {
            max_files: Some(1),
            max_matches: Some(1),
            ..Default::default()
        }
    } else {
        EffectBudget::default()
    };
    let template = allowed_goal
        .then(|| selected.and_then(|provider| template_for(provider, path, goal_name, at, &budget)))
        .flatten();
    let mut alternatives = Vec::new();
    if selected == Some("text") {
        alternatives.push(
            serde_json::to_value(base_request(
                path,
                OperationPayload::Pattern(PatternOperation::Replace {
                    pattern: "BOUNDED_PATTERN".into(),
                    replacement: "NEW_VALUE".into(),
                }),
            ))
            .expect("suggestion serializes"),
        );
    }
    if !candidates.is_empty() {
        alternatives.extend(candidates.iter().map(|provider| json!({"provider": provider, "path": path, "next": format!("suture suggest {path} --goal {goal_name}")})));
    }
    Suggestion {
        provider: detected.clone(),
        detection_basis: basis,
        goal: goal.map(str::to_owned),
        mode: mode.into(),
        recommended_operation: template
            .as_ref()
            .and_then(|value| value.get("operation"))
            .and_then(|value| value.get("operation").or_else(|| value.get("type")))
            .and_then(|value| value.get("type").or(Some(value)))
            .and_then(Value::as_str)
            .map(str::to_owned),
        rationale: if !allowed_goal {
            "The requested goal is outside the controlled 1.1 goal set; choose one of the advertised goals.".into()
        } else if candidates.is_empty() {
            format!("Use the most specific advertised provider for {}; preview before committing when the target is unfamiliar.", selected.unwrap_or("the target"))
        } else {
            "Provider detection is ambiguous; choose one of the candidate providers explicitly."
                .into()
        },
        request_template: template,
        guarantees: vec![
            "exact cardinality is explicit".into(),
            "max_files=1 and max_matches=1 are conservative defaults".into(),
            "preview is available before commit".into(),
        ],
        budget_defaults: budget,
        alternatives,
        blocked_reasons: candidates
            .into_iter()
            .map(|candidate| format!("provider detection remains ambiguous: {candidate}"))
            .chain((!allowed_goal).then_some(format!("unsupported controlled goal: {goal_name}")))
            .collect(),
        capability_set_id: capabilities().capability_set_id,
    }
}

fn template_for(
    provider: &str,
    path: &str,
    goal: &str,
    at: Option<&str>,
    budget: &EffectBudget,
) -> Option<Value> {
    let operation = match goal {
        "create-file" => OperationPayload::File(FileOperation::CreateFile {
            expected_absent: true,
            content: Vec::new(),
        }),
        "delete-file" => OperationPayload::File(FileOperation::DeleteFile {
            expected_hash: "SHA256_OF_CURRENT_FILE".into(),
        }),
        "apply-patch" => OperationPayload::Patch(PatchOperation::UnifiedDiff {
            patch: "--- a/PATH\n+++ b/PATH\n@@ -1 +1 @@\n-OLD\n+NEW\n".into(),
        }),
        "ensure-present" => match provider {
            "json" | "jsonc" => OperationPayload::Json(JsonOperation::EnsurePresent {
                path: at.unwrap_or("$.KEY").into(),
                value: json!("VALUE"),
            }),
            "toml" => OperationPayload::Toml(TomlOperation::EnsurePresent {
                path: at.unwrap_or("key").into(),
                value: TomlValueWrapper::String("VALUE".into()),
            }),
            "yaml" => OperationPayload::Yaml(YamlOperation::EnsurePresent {
                path: at.unwrap_or("key").into(),
                value: json!("VALUE"),
            }),
            "dotenv" => OperationPayload::Dotenv(DotenvOperation::EnsurePresent {
                key: at.unwrap_or("KEY").into(),
                value: "VALUE".into(),
            }),
            _ => OperationPayload::Text(TextOperation::EnsurePresent {
                content: "CONTENT_TO_ENSURE".into(),
            }),
        },
        "ensure-absent" | "remove-item" => match provider {
            "json" | "jsonc" => OperationPayload::Json(if goal == "remove-item" {
                JsonOperation::Delete {
                    path: at.unwrap_or("$.KEY").into(),
                }
            } else {
                JsonOperation::EnsureAbsent {
                    path: at.unwrap_or("$.KEY").into(),
                }
            }),
            "toml" => OperationPayload::Toml(if goal == "remove-item" {
                TomlOperation::Delete {
                    path: at.unwrap_or("key").into(),
                }
            } else {
                TomlOperation::EnsureAbsent {
                    path: at.unwrap_or("key").into(),
                }
            }),
            "yaml" => OperationPayload::Yaml(if goal == "remove-item" {
                YamlOperation::Delete {
                    path: at.unwrap_or("key").into(),
                }
            } else {
                YamlOperation::EnsureAbsent {
                    path: at.unwrap_or("key").into(),
                }
            }),
            "dotenv" => OperationPayload::Dotenv(DotenvOperation::Unset {
                key: at.unwrap_or("KEY").into(),
            }),
            _ => OperationPayload::Text(TextOperation::EnsureAbsent {
                target: "EXACT_TARGET".into(),
            }),
        },
        "rename" => match provider {
            "json" | "jsonc" => OperationPayload::Json(JsonOperation::RenameKey {
                path: at.unwrap_or("$.OLD_KEY").into(),
                new_key: "NEW_KEY".into(),
            }),
            "toml" => OperationPayload::Toml(TomlOperation::RenameKey {
                path: at.unwrap_or("old.key").into(),
                new_key: "new_key".into(),
            }),
            "code" => OperationPayload::Code(CodeOperation::ReplaceNode {
                language: language_for_path(path),
                target: "OLD_NODE".into(),
                replacement: "NEW_NODE".into(),
                node_kind: None,
            }),
            _ => OperationPayload::Text(TextOperation::Rename {
                target: "OLD_TEXT".into(),
                replacement: "NEW_TEXT".into(),
            }),
        },
        "set-value" => match provider {
            "json" | "jsonc" => OperationPayload::Json(JsonOperation::Set {
                path: at.unwrap_or("$.KEY").into(),
                value: json!("NEW_VALUE"),
            }),
            "toml" => OperationPayload::Toml(TomlOperation::Set {
                path: at.unwrap_or("key").into(),
                value: TomlValueWrapper::String("NEW_VALUE".into()),
            }),
            "yaml" => OperationPayload::Yaml(YamlOperation::Set {
                path: at.unwrap_or("key").into(),
                value: json!("NEW_VALUE"),
            }),
            "code" => OperationPayload::Code(CodeOperation::ReplaceNode {
                language: language_for_path(path),
                target: "OLD_LITERAL".into(),
                replacement: "NEW_LITERAL".into(),
                node_kind: None,
            }),
            _ => OperationPayload::Text(TextOperation::Set {
                target: "OLD_VALUE".into(),
                replacement: "NEW_VALUE".into(),
            }),
        },
        "add-item" => match provider {
            "json" | "jsonc" => OperationPayload::Json(JsonOperation::Insert {
                path: at.unwrap_or("$").into(),
                key_or_index: "KEY_OR_INDEX".into(),
                value: json!("VALUE"),
            }),
            "toml" => OperationPayload::Toml(TomlOperation::Insert {
                path: at.unwrap_or("").into(),
                key: "KEY".into(),
                value: TomlValueWrapper::String("VALUE".into()),
            }),
            "dotenv" => OperationPayload::Dotenv(DotenvOperation::Set {
                key: at.unwrap_or("KEY").into(),
                value: "VALUE".into(),
            }),
            _ => OperationPayload::Text(TextOperation::EnsurePresent {
                content: "ITEM_TO_ADD".into(),
            }),
        },
        "move" => OperationPayload::Text(TextOperation::Move {
            target: "EXACT_TARGET".into(),
            before: "EXACT_DESTINATION".into(),
        }),
        "transact" => {
            return Some(
                serde_json::to_value(transaction_request(vec![text_request(
                    path,
                    TextOperation::Replace {
                        target: "OLD".into(),
                        replacement: "NEW".into(),
                    },
                )]))
                .expect("suggestion serializes"),
            )
        }
        "replace-text" => match provider {
            "code" => OperationPayload::Code(CodeOperation::ReplaceNode {
                language: language_for_path(path),
                target: "OLD_NODE".into(),
                replacement: "NEW_NODE".into(),
                node_kind: None,
            }),
            _ => OperationPayload::Text(TextOperation::Replace {
                target: "EXACT_TARGET".into(),
                replacement: "NEW_VALUE".into(),
            }),
        },
        _ => OperationPayload::Text(TextOperation::Replace {
            target: "EXACT_TARGET".into(),
            replacement: "NEW_VALUE".into(),
        }),
    };
    let mut request = match operation {
        OperationPayload::Json(operation) if provider == "jsonc" => {
            base_request(path, OperationPayload::Jsonc(operation))
        }
        operation => base_request(path, operation),
    };
    request.budget = budget.clone();
    let value = serde_json::to_value(request).expect("suggestion serializes");
    Some(value)
}

fn language_for_path(path: &str) -> String {
    match std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        _ => "javascript",
    }
    .into()
}

pub fn refusal_recovery(certificate: &crate::protocol::Certificate) -> Value {
    let reason = certificate
        .refusal_reason
        .as_ref()
        .map(|value| value.code())
        .unwrap_or("UNKNOWN");
    let mut suggestions = Vec::new();
    if let Some(crate::protocol::RefusalReason::DuplicateTarget { candidates, .. }) =
        certificate.refusal_reason.as_ref()
    {
        for candidate in candidates {
            let request = base_request(
                &certificate.file_path,
                OperationPayload::Text(TextOperation::Replace {
                    target: candidate.context.clone(),
                    replacement: "REPLACEMENT".into(),
                }),
            );
            suggestions.push(json!({"file_path": certificate.file_path, "provider": certificate.provider, "selector": candidate.context, "candidate_line": candidate.line, "candidate_fingerprint": candidate.anchor_sha256, "request_template": serde_json::to_value(request).expect("recovery request serializes"), "next": "preview this skeleton and confirm the intended candidate"}));
        }
    }
    if suggestions.is_empty() {
        let request = base_request(
            &certificate.file_path,
            OperationPayload::Text(TextOperation::Replace {
                target: "EXACT_TARGET".into(),
                replacement: "REPLACEMENT".into(),
            }),
        );
        suggestions.push(json!({"file_path": certificate.file_path, "provider": certificate.provider, "request_template": serde_json::to_value(request).expect("recovery request serializes"), "next": "inspect the target, narrow the selector or explicitly correct the guard/budget"}));
    }
    json!({"reason_code": reason, "capability_set_id": capabilities().capability_set_id, "source_certificate": certificate.request_id, "suggestions": suggestions, "blocked_reasons": [format!("{reason} must be corrected before retry")], "safe_retry": false})
}
