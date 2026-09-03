#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
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
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(CodeError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one syntax node".into(),
            actual: 1,
        }));
    }
    let (language_name, target, replacement, before, after, node_kind) = match op {
        CodeOperation::ReplaceNode {
            language,
            target,
            replacement,
            node_kind,
        } => (
            language,
            target,
            replacement.as_bytes(),
            false,
            false,
            node_kind,
        ),
        CodeOperation::InsertBeforeNode {
            language,
            target,
            content,
            node_kind,
        } => (language, target, content.as_bytes(), true, false, node_kind),
        CodeOperation::InsertAfterNode {
            language,
            target,
            content,
            node_kind,
        } => (language, target, content.as_bytes(), false, true, node_kind),
        CodeOperation::RemoveNode {
            language,
            target,
            node_kind,
        } => (language, target, &b""[..], false, false, node_kind),
    };
    let language = language(language_name)?;
    let mut parser = Parser::new();
    parser.set_language(&language).map_err(|_| {
        CodeError::Refused(RefusalReason::ProviderCapabilityMissing {
            provider: "code".into(),
            capability: language_name.to_string(),
        })
    })?;
    let tree = parser.parse(content, None).ok_or_else(|| {
        CodeError::Refused(RefusalReason::MalformedInput {
            details: "parser returned no syntax tree".into(),
        })
    })?;
    if tree.root_node().has_error() {
        return Err(CodeError::Refused(RefusalReason::MalformedInput {
            details: format!("{language_name} source contains syntax errors"),
        }));
    }
    let mut found = Vec::new();
    collect_nodes(
        tree.root_node(),
        content,
        target.as_bytes(),
        node_kind.as_deref(),
        &mut found,
    );
    if found.len() != 1 {
        return Err(CodeError::Refused(if found.is_empty() {
            RefusalReason::MissingTarget {
                target: format!("no {language_name} syntax node matched"),
            }
        } else {
            RefusalReason::DuplicateTarget {
                target: target.into(),
                count: found.len(),
                candidates: found
                    .iter()
                    .take(8)
                    .map(|(start, end, _)| crate::protocol::Candidate {
                        offset: *start,
                        line: content[..*start].iter().filter(|b| **b == b'\n').count() + 1,
                        context: String::from_utf8_lossy(
                            &content[start.saturating_sub(24)..(*end + 24).min(content.len())],
                        )
                        .into(),
                        anchor_sha256: crate::engine::compute_sha256(&content[*start..*end]),
                    })
                    .collect(),
            }
        }));
    }
    let (start, end, _) = found[0];
    let replacement = if before {
        [replacement, &content[start..end]].concat()
    } else if after {
        [&content[start..end], replacement].concat()
    } else {
        replacement.to_vec()
    };
    Ok(vec![ByteEdit {
        start,
        end,
        replacement,
    }])
}

pub fn validate(content: &[u8], language_name: &str) -> Result<(), CodeError> {
    let language = language(language_name)?;
    let mut parser = Parser::new();
    parser.set_language(&language).map_err(|_| {
        CodeError::Refused(RefusalReason::ProviderCapabilityMissing {
            provider: "code".into(),
            capability: language_name.into(),
        })
    })?;
    let tree = parser.parse(content, None).ok_or_else(|| {
        CodeError::Refused(RefusalReason::MalformedInput {
            details: "parser returned no syntax tree".into(),
        })
    })?;
    if tree.root_node().has_error() {
        Err(CodeError::Refused(RefusalReason::MalformedInput {
            details: format!("{language_name} source contains syntax errors"),
        }))
    } else {
        Ok(())
    }
}

fn language(name: &str) -> Result<Language, CodeError> {
    match name.to_ascii_lowercase().as_str() {
        "javascript" | "js" | "jsx" => Ok(tree_sitter_javascript::LANGUAGE.into()),
        "typescript" | "ts" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Ok(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "python" | "py" => Ok(tree_sitter_python::LANGUAGE.into()),
        "rust" | "rs" => Ok(tree_sitter_rust::LANGUAGE.into()),
        "go" | "golang" => Ok(tree_sitter_go::LANGUAGE.into()),
        other => Err(CodeError::Refused(
            RefusalReason::ProviderCapabilityMissing {
                provider: "code".into(),
                capability: format!("language grammar: {other}"),
            },
        )),
    }
}

fn collect_nodes(
    node: Node<'_>,
    content: &[u8],
    target: &[u8],
    kind: Option<&str>,
    out: &mut Vec<(usize, usize, String)>,
) {
    let start = node.start_byte();
    let end = node.end_byte();
    if &content[start..end] == target && kind.is_none_or(|wanted| wanted == node.kind()) {
        out.push((start, end, node.kind().into()));
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes(child, content, target, kind, out);
    }
}
