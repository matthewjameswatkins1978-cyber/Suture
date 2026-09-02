use crate::path::PathNamespace;
use crate::provider::json::JsonOperation;
use crate::provider::text::TextOperation;
use crate::provider::toml::TomlOperation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperationPayload {
    Text(TextOperation),
    Json(JsonOperation),
    Toml(TomlOperation),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
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
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    CardinalityMismatch {
        expected: String,
        actual: usize,
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
    },
    MalformedInput {
        details: String,
    },
    Custom {
        message: String,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    IoError { message: String },
    ParseError { details: String },
    WriteError { message: String },
    Custom { message: String },
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
pub struct Certificate {
    pub outcome: Outcome,
    pub file_path: String,
    pub pre_hash: String,
    pub post_hash: Option<String>,
    pub refusal_reason: Option<RefusalReason>,
    pub failure_reason: Option<FailureReason>,
    pub diff_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::schema_for;

    #[test]
    fn test_outcome_serialization() {
        let outcome = Outcome::Applied;
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(json, "\"APPLIED\"");
        let deserialized: Outcome = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, outcome);
    }

    #[test]
    fn test_refusal_reason_serialization() {
        let refusal = RefusalReason::CardinalityMismatch {
            expected: "1".to_string(),
            actual: 2,
        };
        let json = serde_json::to_string(&refusal).unwrap();
        assert!(json.contains("cardinality_mismatch"));
        let deserialized: RefusalReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, refusal);
    }

    #[test]
    fn test_failure_reason_serialization() {
        let failure = FailureReason::IoError {
            message: "disk full".to_string(),
        };
        let json = serde_json::to_string(&failure).unwrap();
        assert!(json.contains("io_error"));
        let deserialized: FailureReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, failure);
    }

    #[test]
    fn test_cardinality_serialization() {
        let card_one = Cardinality::ExactlyOne;
        let json_one = serde_json::to_string(&card_one).unwrap();
        assert_eq!(json_one, "{\"type\":\"exactly_one\"}");
        let de_one: Cardinality = serde_json::from_str(&json_one).unwrap();
        assert_eq!(de_one, card_one);

        let card_exact = Cardinality::Exactly(5);
        let json_exact = serde_json::to_string(&card_exact).unwrap();
        assert_eq!(json_exact, "{\"type\":\"exactly\",\"value\":5}");
        let de_exact: Cardinality = serde_json::from_str(&json_exact).unwrap();
        assert_eq!(de_exact, card_exact);

        let card_all = Cardinality::All;
        let json_all = serde_json::to_string(&card_all).unwrap();
        assert_eq!(json_all, "{\"type\":\"all\"}");
        let de_all: Cardinality = serde_json::from_str(&json_all).unwrap();
        assert_eq!(de_all, card_all);
    }

    #[test]
    fn test_byte_edit_serialization() {
        let edit = ByteEdit {
            offset: 10,
            delete_len: 4,
            replacement: b"test".to_vec(),
        };
        let json = serde_json::to_string(&edit).unwrap();
        let deserialized: ByteEdit = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, edit);
    }

    #[test]
    fn test_mutation_plan_serialization() {
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: "src/main.rs".to_string(),
            expected_pre_hash: "abc123hash".to_string(),
            edits: vec![ByteEdit {
                offset: 0,
                delete_len: 0,
                replacement: b"fn main() {}".to_vec(),
            }],
            cardinality: Cardinality::ExactlyOne,
        };
        let json = serde_json::to_string_pretty(&plan).unwrap();
        let deserialized: MutationPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, plan);
    }

    #[test]
    fn test_certificate_serialization() {
        let cert = Certificate {
            outcome: Outcome::Applied,
            file_path: "src/main.rs".to_string(),
            pre_hash: "hash1".to_string(),
            post_hash: Some("hash2".to_string()),
            refusal_reason: None,
            failure_reason: None,
            diff_summary: Some("+fn main() {}".to_string()),
        };
        let json = serde_json::to_string(&cert).unwrap();
        let deserialized: Certificate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cert);
    }

    #[test]
    fn test_schema_generation() {
        let schema = schema_for!(MutationPlan);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(!schema_json.is_empty());

        let cert_schema = schema_for!(Certificate);
        let cert_schema_json = serde_json::to_string_pretty(&cert_schema).unwrap();
        assert!(!cert_schema_json.is_empty());
    }
}
