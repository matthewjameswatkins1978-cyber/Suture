#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use crate::provider::syntax::{self, LanguageFamily, Placement, StructuralTargeting};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CodeOperation {
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
pub enum CodeError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

pub fn plan(
    content: &[u8],
    op: &CodeOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, CodeError> {
    let (language, target, replacement, placement, node_kind) = match op {
        CodeOperation::ReplaceNode {
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
        CodeOperation::InsertBeforeNode {
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
        CodeOperation::InsertAfterNode {
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
        CodeOperation::RemoveNode {
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
        LanguageFamily::Code,
        cardinality,
    )
    .map(|plan| plan.edits)
    .map_err(|error| match error {
        syntax::SyntaxError::Refused(reason) => CodeError::Refused(reason),
        syntax::SyntaxError::Engine(error) => CodeError::Refused(RefusalReason::Custom {
            message: error.to_string(),
        }),
    })
}

pub fn plan_at(
    content: &[u8],
    op: &CodeOperation,
    start: usize,
    end: usize,
) -> Result<Vec<ByteEdit>, CodeError> {
    let (language, target, replacement, placement, node_kind) = match op {
        CodeOperation::ReplaceNode {
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
        CodeOperation::InsertBeforeNode {
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
        CodeOperation::InsertAfterNode {
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
        CodeOperation::RemoveNode {
            language,
            target,
            node_kind,
        } => (language, target, &[][..], Placement::Replace, node_kind),
    };
    syntax::plan_at(
        content,
        language,
        target,
        replacement,
        placement,
        node_kind.as_deref(),
        LanguageFamily::Code,
        start,
        end,
    )
    .map(|plan| plan.edits)
    .map_err(|error| match error {
        syntax::SyntaxError::Refused(reason) => CodeError::Refused(reason),
        syntax::SyntaxError::Engine(error) => CodeError::Refused(RefusalReason::Custom {
            message: error.to_string(),
        }),
    })
}

pub fn validate(content: &[u8], language_name: &str) -> Result<(), CodeError> {
    syntax::validate(content, language_name).map_err(|error| match error {
        syntax::SyntaxError::Refused(reason) => CodeError::Refused(reason),
        syntax::SyntaxError::Engine(error) => CodeError::Refused(RefusalReason::Custom {
            message: error.to_string(),
        }),
    })
}

pub fn targeting(node_kind: Option<&str>) -> StructuralTargeting {
    if node_kind.is_some() {
        StructuralTargeting::AstTyped
    } else {
        StructuralTargeting::AstGrounded
    }
}
