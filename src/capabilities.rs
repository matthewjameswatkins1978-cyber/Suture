#![forbid(unsafe_code)]

use crate::protocol::PROTOCOL_VERSION;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Capabilities {
    pub protocol_versions: Vec<&'static str>,
    pub providers: Vec<ProviderCapability>,
    pub operations: Vec<&'static str>,
    pub selectors: Vec<&'static str>,
    pub preservation_guarantees: Vec<&'static str>,
    pub encodings: Vec<&'static str>,
    pub path_namespaces: Vec<&'static str>,
    pub code_languages: Vec<&'static str>,
    pub transaction_capabilities: TransactionCapabilities,
    pub resource_limits: ResourceLimits,
    pub effect_budget_dimensions: Vec<&'static str>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProviderCapability {
    pub name: &'static str,
    pub version: &'static str,
    pub operations: Vec<&'static str>,
    pub selectors: Vec<&'static str>,
}

#[derive(Serialize, Clone, Debug)]
pub struct TransactionCapabilities {
    pub single_file: bool,
    pub multi_file: bool,
    pub rollback: bool,
    pub crash_recovery: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResourceLimits {
    pub max_request_bytes: usize,
    pub max_diagnostic_bytes: usize,
    pub max_pattern_bytes: usize,
    pub max_file_bytes: usize,
}

pub fn current() -> Capabilities {
    let operations = vec![
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
    Capabilities {
        protocol_versions: vec![PROTOCOL_VERSION],
        providers: vec![
            ProviderCapability {
                name: "text",
                version: "text-byte-v1",
                operations: operations.clone(),
                selectors: vec!["literal"],
            },
            ProviderCapability {
                name: "json",
                version: "json-source-v1",
                operations: vec![
                    "set",
                    "insert",
                    "delete",
                    "rename_key",
                    "ensure_present",
                    "ensure_absent",
                    "unset",
                    "rename",
                ],
                selectors: vec!["json_pointer"],
            },
            ProviderCapability {
                name: "jsonc",
                version: "jsonc-source-v1",
                operations: vec![
                    "set",
                    "insert",
                    "delete",
                    "rename_key",
                    "ensure_present",
                    "ensure_absent",
                    "unset",
                    "rename",
                ],
                selectors: vec!["json_pointer"],
            },
            ProviderCapability {
                name: "toml",
                version: "toml-edit-narrow-v1",
                operations: vec!["set", "insert", "delete", "rename_key"],
                selectors: vec!["dotted_key"],
            },
            ProviderCapability {
                name: "pattern",
                version: "bounded-literal-v1",
                operations: vec!["replace", "delete", "ensure_present", "ensure_absent"],
                selectors: vec!["bounded_pattern"],
            },
            ProviderCapability {
                name: "markdown",
                version: "markdown-regions-v1",
                operations: vec![
                    "replace_section",
                    "ensure_section",
                    "delete_section",
                    "insert_after_heading",
                ],
                selectors: vec!["heading", "section", "fenced_region"],
            },
            ProviderCapability {
                name: "yaml",
                version: "yaml-conservative-source-v1",
                operations: vec!["set", "ensure_present", "delete", "ensure_absent"],
                selectors: vec!["top_level_scalar_key"],
            },
            ProviderCapability {
                name: "filesystem",
                version: "lifecycle-checked-v1",
                operations: vec!["create_file", "delete_file", "rename_file", "move_file"],
                selectors: vec!["workspace_relative_path"],
            },
            ProviderCapability {
                name: "code",
                version: "tree-sitter-node-v1",
                operations: vec![
                    "replace_node",
                    "insert_before_node",
                    "insert_after_node",
                    "remove_node",
                ],
                selectors: vec!["syntax_node_text", "syntax_node_kind"],
            },
            ProviderCapability {
                name: "dotenv",
                version: "dotenv-lines-v1",
                operations: vec!["set", "unset", "ensure_present"],
                selectors: vec!["key"],
            },
            ProviderCapability {
                name: "patch",
                version: "unified-diff-strict-v1",
                operations: vec!["unified_diff"],
                selectors: vec!["exact_preimage", "exact_context"],
            },
        ],
        operations,
        selectors: vec![
            "literal",
            "json_pointer",
            "dotted_key",
            "bounded_pattern",
            "syntax_node_text",
            "syntax_node_kind",
        ],
        preservation_guarantees: vec![
            "unrelated_bytes",
            "utf8",
            "utf8_bom",
            "lf",
            "crlf",
            "final_newline",
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
        transaction_capabilities: TransactionCapabilities {
            single_file: true,
            multi_file: true,
            rollback: true,
            crash_recovery: true,
        },
        resource_limits: ResourceLimits {
            max_request_bytes: 1_048_576,
            max_diagnostic_bytes: 4_096,
            max_pattern_bytes: 8_192,
            max_file_bytes: 64 * 1024 * 1024,
        },
        effect_budget_dimensions: vec![
            "max_files",
            "max_matches",
            "max_changed_regions",
            "max_changed_lines",
            "max_changed_bytes",
            "allowed_path_prefixes",
        ],
    }
}
