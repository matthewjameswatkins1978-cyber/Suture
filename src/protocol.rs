use crate::path::PathNamespace;
use crate::provider::json::JsonOperation;
use crate::provider::text::TextOperation;
use crate::provider::toml::TomlOperation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "0.1.0";

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
    Toml(TomlOperation),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: String,
    pub file_path: String,
    #[serde(default)]
    pub namespace: PathNamespace,
    pub expected_pre_hash: Option<String>,
    #[serde(default)]
    pub cardinality: Cardinality,
    pub operation: OperationPayload,
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
    pub outcome: Outcome,
    pub file_path: String,
    pub provider: String,
    pub provider_version: String,
    pub expected_cardinality: Cardinality,
    pub observed_cardinality: Option<usize>,
    pub pre_hash: String,
    pub post_hash: Option<String>,
    pub changed_ranges: Vec<ByteRange>,
    pub diff_summary: Option<String>,
    pub diff_truncated: bool,
    pub structural_validation: StructuralValidation,
    pub preservation: PreservationFacts,
    pub commit: CommitGuarantee,
    pub refusal_reason: Option<RefusalReason>,
    pub failure_reason: Option<FailureReason>,
    pub diagnostics: Vec<String>,
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
