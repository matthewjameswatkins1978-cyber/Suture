use crate::lifecycle::FileOperation;
use crate::path::PathNamespace;
use crate::pattern::PatternOperation;
use crate::provider::code::CodeOperation;
use crate::provider::dotenv::DotenvOperation;
use crate::provider::json::JsonOperation;
use crate::provider::markdown::MarkdownOperation;
use crate::provider::patch::PatchOperation;
use crate::provider::text::TextOperation;
use crate::provider::toml::TomlOperation;
use crate::provider::yaml::YamlOperation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1.1.0";

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(
    tag = "provider",
    content = "operation",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OperationPayload {
    Text(TextOperation),
    Json(JsonOperation),
    Jsonc(JsonOperation),
    Toml(TomlOperation),
    Pattern(PatternOperation),
    Markdown(MarkdownOperation),
    Yaml(YamlOperation),
    File(FileOperation),
    Code(CodeOperation),
    Dotenv(DotenvOperation),
    Patch(PatchOperation),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: String,
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub allow_generated: bool,
    pub file_path: String,
    #[serde(default)]
    pub namespace: PathNamespace,
    pub expected_pre_hash: Option<String>,
    #[serde(default)]
    pub region_guard: Option<RegionGuard>,
    #[serde(default)]
    pub cardinality: Cardinality,
    #[serde(default)]
    pub budget: EffectBudget,
    pub operation: OperationPayload,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegionGuard {
    pub anchor: String,
    pub target_sha256: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionRequest {
    pub version: String,
    pub transaction_id: String,
    pub requests: Vec<Request>,
    #[serde(default)]
    pub budget: EffectBudget,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TransactionCertificate {
    pub protocol_version: String,
    pub transaction_id: String,
    pub outcome: Outcome,
    pub certificates: Vec<Certificate>,
    pub rollback_state: String,
    pub transaction_guarantee: String,
    pub refusal_reason: Option<RefusalReason>,
    pub failure_reason: Option<FailureReason>,
    pub reason_code: Option<String>,
}

/// A hard upper bound on the mutation's prepared effect. `None` means that
/// particular dimension is unbounded; Suture still applies its own safety
/// limits for pathological requests.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct EffectBudget {
    pub max_files: Option<usize>,
    pub max_matches: Option<usize>,
    pub max_changed_regions: Option<usize>,
    pub max_changed_lines: Option<usize>,
    pub max_changed_bytes: Option<usize>,
    pub allowed_path_prefixes: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EffectUsage {
    pub files: usize,
    pub matches: usize,
    pub changed_regions: usize,
    pub changed_lines: usize,
    pub changed_bytes: usize,
    pub passed: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Outcome {
    Applied,
    NoChange,
    Refused,
    Failed,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RefusalReason {
    CardinalityMismatch {
        expected: String,
        actual: usize,
    },
    CardinalityAmbiguous {
        path: String,
        count: usize,
    },
    StaleIdentity {
        expected_hash: String,
        actual_hash: String,
    },
    WorkspaceTraversal {
        path: String,
    },
    SymlinkEscape {
        path: String,
    },
    MissingTarget {
        target: String,
    },
    DuplicateTarget {
        target: String,
        count: usize,
        candidates: Vec<Candidate>,
    },
    UnsupportedEncoding {
        details: String,
    },
    MalformedInput {
        details: String,
    },
    ProviderCapabilityMissing {
        provider: String,
        capability: String,
    },
    PreservationUnavailable {
        details: String,
    },
    LossyOperationRequiresOptIn {
        operation: String,
    },
    UnsupportedOperation {
        operation: String,
    },
    UnsupportedProtocolVersion {
        requested: String,
        supported: String,
    },
    Custom {
        message: String,
    },
    EffectBudgetExceeded {
        dimension: String,
        limit: usize,
        actual: usize,
    },
    GeneratedFileRequiresOptIn {
        marker: String,
    },
    BinaryInput,
    DestinationExists {
        path: String,
    },
}

impl RefusalReason {
    /// Stable machine identifier used by certificates and `suture explain`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CardinalityMismatch { .. } => "CARDINALITY_MISMATCH",
            Self::CardinalityAmbiguous { .. } | Self::DuplicateTarget { .. } => "TARGET_AMBIGUOUS",
            Self::StaleIdentity { .. } => "STALE_IDENTITY",
            Self::WorkspaceTraversal { .. } => "WORKSPACE_ESCAPE",
            Self::SymlinkEscape { .. } => "SYMLINK_ESCAPE",
            Self::MissingTarget { .. } => "TARGET_NOT_FOUND",
            Self::UnsupportedEncoding { .. } => "ENCODING_UNSUPPORTED",
            Self::MalformedInput { .. } => "INVALID_INPUT",
            Self::ProviderCapabilityMissing { .. } => "PROVIDER_UNSUPPORTED",
            Self::PreservationUnavailable { .. } => "PRESERVATION_UNAVAILABLE",
            Self::LossyOperationRequiresOptIn { .. } => "LOSSY_OPERATION_REQUIRES_OPT_IN",
            Self::UnsupportedOperation { .. } => "OPERATION_UNSUPPORTED",
            Self::UnsupportedProtocolVersion { .. } => "PROTOCOL_UNSUPPORTED",
            Self::Custom { .. } => "REFUSED",
            Self::EffectBudgetExceeded { .. } => "EFFECT_BUDGET_EXCEEDED",
            Self::GeneratedFileRequiresOptIn { .. } => "GENERATED_FILE_REQUIRES_OPT_IN",
            Self::BinaryInput => "BINARY_INPUT",
            Self::DestinationExists { .. } => "DESTINATION_EXISTS",
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub offset: usize,
    pub line: usize,
    pub context: String,
    pub anchor_sha256: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FailureReason {
    IoError {
        message: String,
    },
    ProviderError {
        details: String,
    },
    InternalInvariant {
        details: String,
    },
    CommitFailure {
        message: String,
    },
    PostCommitVerificationFailure {
        expected_hash: String,
        actual_hash: String,
    },
    ParseError {
        details: String,
    },
    WriteError {
        message: String,
    },
    Custom {
        message: String,
    },
}

impl FailureReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::IoError { .. } => "IO_ERROR",
            Self::ProviderError { .. } | Self::ParseError { .. } => "INVALID_STRUCTURE",
            Self::InternalInvariant { .. } => "INTERNAL_INVARIANT",
            Self::CommitFailure { .. } | Self::WriteError { .. } => "COMMIT_FAILED",
            Self::PostCommitVerificationFailure { .. } => "POST_COMMIT_VERIFICATION_FAILED",
            Self::Custom { .. } => "FAILED",
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, Default)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Cardinality {
    #[default]
    ExactlyOne,
    #[serde(rename = "exactly")]
    Exactly(usize),
    All,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct ByteEdit {
    pub offset: usize,
    pub delete_len: usize,
    pub replacement: Vec<u8>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct MutationPlan {
    pub version: String,
    pub file_path: String,
    pub expected_pre_hash: String,
    pub edits: Vec<ByteEdit>,
    #[serde(default)]
    pub cardinality: Cardinality,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum StructuralValidation {
    #[default]
    NotApplicable,
    Valid {
        format: String,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct PreservationFacts {
    pub unrelated_bytes_changed: bool,
    pub line_endings_changed: bool,
    pub bom_changed: bool,
    pub final_newline_changed: bool,
    pub comments_preserved: Option<bool>,
    pub metadata: String,
    pub original_newline_profile: String,
    pub result_newline_profile: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
pub struct CommitGuarantee {
    pub mode: String,
    pub content_replacement: String,
    pub permissions: String,
    pub timestamps: String,
    pub acl_xattr: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    pub protocol_version: String,
    pub request_id: String,
    pub outcome: Outcome,
    pub file_path: String,
    pub provider: String,
    pub provider_version: String,
    pub expected_cardinality: Cardinality,
    pub observed_cardinality: Option<usize>,
    pub pre_hash: String,
    pub post_hash: Option<String>,
    pub changed_ranges: Vec<ByteRange>,
    pub changed_line_ranges: Vec<ByteRange>,
    pub diff_summary: Option<String>,
    pub diff_truncated: bool,
    pub structural_validation: StructuralValidation,
    pub preservation: PreservationFacts,
    pub commit: CommitGuarantee,
    pub refusal_reason: Option<RefusalReason>,
    pub failure_reason: Option<FailureReason>,
    pub reason_code: Option<String>,
    pub diagnostics: Vec<String>,
    pub budget: EffectBudget,
    pub effect: EffectUsage,
    pub transaction_guarantee: String,
    pub recovery_state: String,
}

impl Default for PreservationFacts {
    fn default() -> Self {
        Self {
            unrelated_bytes_changed: false,
            line_endings_changed: false,
            bom_changed: false,
            final_newline_changed: false,
            comments_preserved: None,
            metadata: "not_verified".into(),
            original_newline_profile: "unknown".into(),
            result_newline_profile: "unknown".into(),
        }
    }
}
impl Default for CommitGuarantee {
    fn default() -> Self {
        Self {
            mode: "not_committed".into(),
            content_replacement: "not_applicable".into(),
            permissions: "not_verified".into(),
            timestamps: "not_preserved_by_replacement".into(),
            acl_xattr: "unknown".into(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;
    #[test]
    fn outcome_serializes() {
        assert_eq!(
            serde_json::to_string(&Outcome::Applied).unwrap(),
            "\"APPLIED\""
        );
    }
    #[test]
    fn cardinality_serializes() {
        assert_eq!(
            serde_json::to_string(&Cardinality::ExactlyOne).unwrap(),
            "{\"type\":\"exactly_one\"}"
        );
        assert_eq!(
            serde_json::to_string(&Cardinality::Exactly(5)).unwrap(),
            "{\"type\":\"exactly\",\"value\":5}"
        );
    }
    #[test]
    fn unknown_request_fields_are_rejected() {
        let json = r#"{"version":"0.1.0","file_path":"a","expected_pre_hash":null,"operation":{"type":"text","bogus":1}}"#;
        assert!(serde_json::from_str::<Request>(json).is_err());
    }
    #[test]
    fn schema_generation_runs() {
        assert!(!serde_json::to_string(&schema_for!(Certificate))
            .unwrap()
            .is_empty());
    }
}
