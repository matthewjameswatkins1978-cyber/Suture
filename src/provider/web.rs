#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use crate::provider::syntax::{self, LanguageFamily, Placement};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WebOperation {
    ReplaceNode {
        language: String,
        target: String,
        replacement: String,
        #[serde(default)]
        node_kind: Option<String>,
    },
    InsertBeforeNode {
        language: String,
        target: String,
        content: String,
        #[serde(default)]
        node_kind: Option<String>,
    },
    InsertAfterNode {
        language: String,
        target: String,
        content: String,
        #[serde(default)]
        node_kind: Option<String>,
    },
    RemoveNode {
        language: String,
        target: String,
        #[serde(default)]
        node_kind: Option<String>,
    },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum WebError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

pub fn plan(
    content: &[u8],
    op: &WebOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, WebError> {
    let (language, target, replacement, placement, node_kind) = match op {
        WebOperation::ReplaceNode {
            language,
            target,
            replacement,
            node_kind,
        } => (
            language,
            target,
            replacement.as_bytes(),
            Placement::Replace,
            node_kind,
        ),
        WebOperation::InsertBeforeNode {
            language,
            target,
            content,
            node_kind,
        } => (
            language,
            target,
            content.as_bytes(),
            Placement::Before,
            node_kind,
        ),
        WebOperation::InsertAfterNode {
            language,
            target,
            content,
            node_kind,
        } => (
            language,
            target,
            content.as_bytes(),
            Placement::After,
            node_kind,
        ),
        WebOperation::RemoveNode {
            language,
            target,
            node_kind,
        } => (language, target, &[][..], Placement::Replace, node_kind),
    };
    syntax::plan(
        content,
        language,
        target,
        replacement,
        placement,
        node_kind.as_deref(),
        LanguageFamily::Web,
        cardinality,
    )
    .map(|plan| plan.edits)
    .map_err(|error| match error {
        syntax::SyntaxError::Refused(reason) => WebError::Refused(reason),
        syntax::SyntaxError::Engine(error) => WebError::Refused(RefusalReason::Custom {
            message: error.to_string(),
        }),
    })
}

pub fn validate(content: &[u8], language_name: &str) -> Result<(), WebError> {
    syntax::validate(content, language_name).map_err(|error| match error {
        syntax::SyntaxError::Refused(reason) => WebError::Refused(reason),
        syntax::SyntaxError::Engine(error) => WebError::Refused(RefusalReason::Custom {
            message: error.to_string(),
        }),
    })
}
