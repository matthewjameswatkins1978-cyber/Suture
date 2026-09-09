#![forbid(unsafe_code)]

//! Conservative, source-preserving YAML targeting.
//!
//! Tree-sitter locates the requested node; Core remains responsible for
//! applying the resulting byte edits. This provider refuses advanced
//! constructs whose local preservation cannot be proved.

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tree_sitter::{Node, Parser};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum YamlOperation {
    Set {
        path: String,
        value: serde_json::Value,
    },
    EnsurePresent {
        path: String,
        value: serde_json::Value,
    },
    Delete {
        path: String,
    },
    EnsureAbsent {
        path: String,
    },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum YamlError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

#[derive(Clone, Debug)]
struct Match<'a> {
    value: Node<'a>,
    container: Node<'a>,
    flow: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Key(String),
    Index(usize),
}

pub fn plan(
    content: &[u8],
    op: &YamlOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, YamlError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(YamlError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one yaml node".into(),
            actual: 1,
        }));
    }
    let text = std::str::from_utf8(content).map_err(|_| {
        YamlError::Refused(RefusalReason::UnsupportedEncoding {
            details: "yaml provider requires UTF-8".into(),
        })
    })?;
    let tree = parse(text)?;
    if contains_unsupported_construct(tree.root_node()) {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable { details: "anchors, aliases, tags, merge keys, or directives are outside the local YAML preservation subset".into() }));
    }
    let path = operation_path(op);
    let parts = parse_path(path)?;
    let mut matches = Vec::new();
    find_matches(
        tree.root_node(),
        text.as_bytes(),
        &parts,
        0,
        tree.root_node(),
        &mut matches,
    );
    if matches.len() > 1 {
        return Err(YamlError::Refused(RefusalReason::DuplicateTarget {
            target: path.into(),
            count: matches.len(),
            candidates: Vec::new(),
            candidates_returned: 0,
            truncated: false,
        }));
    }
    let Some(found) = matches.into_iter().next() else {
        return missing_or_insert(text, tree.root_node(), &parts, op);
    };
    if !is_scalar(found.value) {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable {
            details: "the selected YAML node is not a scalar with a provable local byte range"
                .into(),
        }));
    }
    match op {
        YamlOperation::Delete { .. } | YamlOperation::EnsureAbsent { .. } => {
            if found.flow {
                return Err(flow_refusal());
            }
            Ok(vec![delete_line(text, found.container)])
        }
        YamlOperation::Set { value, .. } | YamlOperation::EnsurePresent { value, .. } => {
            let encoded = scalar(value)?;
            if &text.as_bytes()[found.value.start_byte()..found.value.end_byte()]
                == encoded.as_bytes()
            {
                return Ok(Vec::new());
            }
            Ok(vec![ByteEdit {
                start: found.value.start_byte(),
                end: found.value.end_byte(),
                replacement: encoded.into_bytes(),
            }])
        }
    }
}

pub fn validate(content: &[u8]) -> Result<(), YamlError> {
    let text = std::str::from_utf8(content).map_err(|_| {
        YamlError::Refused(RefusalReason::UnsupportedEncoding {
            details: "yaml provider requires UTF-8".into(),
        })
    })?;
    let tree = parse(text)?;
    if contains_unsupported_construct(tree.root_node()) {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable { details: "anchors, aliases, tags, merge keys, or directives are outside the local YAML preservation subset".into() }));
    }
    Ok(())
}

fn parse(text: &str) -> Result<tree_sitter::Tree, YamlError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_yaml::LANGUAGE.into())
        .map_err(|error| {
            YamlError::Refused(RefusalReason::MalformedInput {
                details: format!("YAML grammar unavailable: {error}"),
            })
        })?;
    let tree = parser.parse(text, None).ok_or_else(|| {
        YamlError::Refused(RefusalReason::MalformedInput {
            details: "YAML parser returned no tree".into(),
        })
    })?;
    if tree.root_node().has_error() {
        return Err(YamlError::Refused(RefusalReason::MalformedInput {
            details: "malformed YAML".into(),
        }));
    }
    Ok(tree)
}

fn operation_path(op: &YamlOperation) -> &str {
    match op {
        YamlOperation::Set { path, .. }
        | YamlOperation::EnsurePresent { path, .. }
        | YamlOperation::Delete { path }
        | YamlOperation::EnsureAbsent { path } => path,
    }
}

fn parse_path(path: &str) -> Result<Vec<Part>, YamlError> {
    if path.is_empty() {
        return Err(path_refusal_reason());
    }
    let mut parts = Vec::new();
    for segment in path.split('.') {
        if segment.is_empty() {
            return Err(path_refusal_reason());
        }
        let mut rest = segment;
        if let Some(open) = rest.find('[') {
            let key = &rest[..open];
            if !key.is_empty() {
                parts.push(Part::Key(key.into()));
            }
            while !rest.is_empty() {
                let Some(open) = rest.find('[') else {
                    return Err(path_refusal_reason());
                };
                let Some(close) = rest[open + 1..].find(']') else {
                    return Err(path_refusal_reason());
                };
                let index = rest[open + 1..open + 1 + close]
                    .parse()
                    .map_err(|_| path_refusal_reason())?;
                parts.push(Part::Index(index));
                rest = &rest[open + close + 2..];
            }
        } else {
            parts.push(Part::Key(rest.into()));
        }
    }
    Ok(parts)
}

fn find_matches<'a>(
    node: Node<'a>,
    source: &[u8],
    parts: &[Part],
    index: usize,
    root: Node<'a>,
    out: &mut Vec<Match<'a>>,
) {
    if index == parts.len() {
        if let Some(value) = editable_value(node) {
            let container = value.parent().unwrap_or(root);
            out.push(Match {
                value,
                container,
                flow: container.kind().contains("flow"),
            });
        }
        return;
    }
    match node.kind() {
        "stream"
        | "document"
        | "block_node"
        | "flow_node"
        | "block_mapping_value"
        | "block_sequence_item" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                find_matches(child, source, parts, index, root, out);
            }
        }
        "block_mapping" | "flow_mapping" => {
            let wanted = &parts[index];
            let mut cursor = node.walk();
            for pair in node.named_children(&mut cursor) {
                if !pair.kind().ends_with("mapping_pair") && pair.kind() != "flow_pair" {
                    continue;
                }
                let Some(key_node) = pair.child_by_field_name("key") else {
                    continue;
                };
                if !matches_key(key_node, source, wanted) {
                    continue;
                }
                if let Some(value) = pair.child_by_field_name("value") {
                    find_matches(value, source, parts, index + 1, root, out);
                }
            }
        }
        "block_sequence" | "flow_sequence" => {
            let Part::Index(wanted) = &parts[index] else {
                return;
            };
            let mut cursor = node.walk();
            if let Some(item) = node.named_children(&mut cursor).nth(*wanted) {
                find_matches(item, source, parts, index + 1, root, out);
            };
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                find_matches(child, source, parts, index, root, out);
            }
        }
    }
}

fn editable_value(node: Node<'_>) -> Option<Node<'_>> {
    if is_scalar(node) {
        return Some(node);
    }
    if node.kind().contains("mapping") || node.kind().contains("sequence") {
        return None;
    }
    let mut cursor = node.walk();
    let children = node.named_children(&mut cursor).collect::<Vec<_>>();
    if children.len() == 1 {
        editable_value(children[0])
    } else {
        None
    }
}

fn matches_key(node: Node<'_>, source: &[u8], wanted: &Part) -> bool {
    let Part::Key(wanted) = wanted else {
        return false;
    };
    let raw = std::str::from_utf8(&source[node.start_byte()..node.end_byte()])
        .unwrap_or("")
        .trim();
    let unquoted = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            raw.strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(raw);
    unquoted == wanted
}

fn missing_or_insert(
    text: &str,
    root: Node<'_>,
    parts: &[Part],
    op: &YamlOperation,
) -> Result<Vec<ByteEdit>, YamlError> {
    if matches!(op, YamlOperation::EnsureAbsent { .. }) {
        return Ok(Vec::new());
    }
    let YamlOperation::EnsurePresent { value, .. } = op else {
        return Err(RefusalReason::MissingTarget {
            target: operation_path(op).into(),
        }
        .into());
    };
    let encoded = scalar(value)?;
    let Some(parent) = find_container(
        root,
        text.as_bytes(),
        &parts[..parts.len().saturating_sub(1)],
        0,
    ) else {
        return Err(RefusalReason::MissingTarget {
            target: operation_path(op).into(),
        }
        .into());
    };
    let Part::Key(key) = parts.last().ok_or_else(path_refusal_reason)? else {
        return Err(flow_refusal());
    };
    if parent.kind() == "flow_mapping" {
        return Err(flow_refusal());
    }
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let prefix = if parent.end_byte() == text.len() && text.ends_with(['\n', '\r']) {
        ""
    } else {
        newline
    };
    let suffix = if parent.end_byte() == text.len() && text.ends_with(['\n', '\r']) {
        newline
    } else {
        ""
    };
    let insertion = format!(
        "{prefix}{}{key}: {encoded}{suffix}",
        " ".repeat(child_indent(text, parent)),
    );
    Ok(vec![ByteEdit {
        start: parent.end_byte(),
        end: parent.end_byte(),
        replacement: insertion.into_bytes(),
    }])
}

fn find_container<'a>(
    node: Node<'a>,
    source: &[u8],
    parts: &[Part],
    index: usize,
) -> Option<Node<'a>> {
    if index == parts.len() {
        return Some(node);
    }
    match node.kind() {
        "stream" | "document" | "block_node" | "flow_node" => {
            let mut cursor = node.walk();
            let result = node
                .named_children(&mut cursor)
                .find_map(|child| find_container(child, source, parts, index));
            result
        }
        "block_mapping" | "flow_mapping" => {
            let wanted = &parts[index];
            let mut cursor = node.walk();
            for pair in node.named_children(&mut cursor) {
                let Some(key) = pair.child_by_field_name("key") else {
                    continue;
                };
                if matches_key(key, source, wanted) {
                    if let Some(value) = pair.child_by_field_name("value") {
                        return find_container(value, source, parts, index + 1);
                    }
                }
            }
            None
        }
        "block_sequence" | "flow_sequence" => {
            let Part::Index(wanted) = &parts[index] else {
                return None;
            };
            let mut cursor = node.walk();
            let result = node
                .named_children(&mut cursor)
                .nth(*wanted)
                .and_then(|item| find_container(item, source, parts, index + 1));
            result
        }
        _ => {
            let mut cursor = node.walk();
            let result = node
                .named_children(&mut cursor)
                .find_map(|child| find_container(child, source, parts, index));
            result
        }
    }
}

fn child_indent(text: &str, parent: Node<'_>) -> usize {
    let line_start = text[..parent.start_byte()]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let parent_indent = text[line_start..]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    let mut cursor = parent.walk();
    let first_child = parent.named_children(&mut cursor).next();
    first_child
        .map(|child| {
            let child_start = text[..child.start_byte()]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            text[child_start..]
                .bytes()
                .take_while(|byte| *byte == b' ')
                .count()
        })
        .unwrap_or(parent_indent + 2)
}

fn delete_line(text: &str, node: Node<'_>) -> ByteEdit {
    let start = text[..node.start_byte()]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let end = text[node.end_byte()..]
        .find('\n')
        .map_or(text.len(), |index| node.end_byte() + index + 1);
    ByteEdit {
        start,
        end,
        replacement: Vec::new(),
    }
}

fn is_scalar(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "plain_scalar"
            | "single_quote_scalar"
            | "double_quote_scalar"
            | "integer_scalar"
            | "float_scalar"
            | "boolean_scalar"
            | "null_scalar"
            | "timestamp_scalar"
            | "string_scalar"
    )
}

fn contains_unsupported_construct(node: Node<'_>) -> bool {
    if matches!(node.kind(), "anchor" | "alias" | "tag" | "directive") {
        return true;
    }
    let mut cursor = node.walk();
    let result = node
        .named_children(&mut cursor)
        .any(contains_unsupported_construct);
    result
}

fn scalar(value: &serde_json::Value) -> Result<String, YamlError> {
    match value {
        serde_json::Value::String(value) => serde_json::to_string(value).map_err(|error| {
            RefusalReason::MalformedInput {
                details: error.to_string(),
            }
            .into()
        }),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Ok(value.to_string())
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Err(RefusalReason::PreservationUnavailable {
                details: "collection replacement requires a whole-document YAML rewrite".into(),
            }
            .into())
        }
    }
}

fn path_refusal_reason() -> YamlError {
    RefusalReason::ProviderCapabilityMissing {
        provider: "yaml".into(),
        capability: "path must use dotted keys and numeric [index] selectors".into(),
    }
    .into()
}
fn flow_refusal() -> YamlError {
    RefusalReason::PreservationUnavailable {
        details: "flow-style YAML edits are refused unless a local source range can be proved"
            .into(),
    }
    .into()
}

impl From<RefusalReason> for YamlError {
    fn from(reason: RefusalReason) -> Self {
        Self::Refused(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply_byte_edits;

    fn set(source: &str, path: &str, value: serde_json::Value) -> String {
        let edits = plan(
            source.as_bytes(),
            &YamlOperation::Set {
                path: path.into(),
                value,
            },
            &Cardinality::ExactlyOne,
        )
        .unwrap();
        String::from_utf8(apply_byte_edits(source.as_bytes(), &edits).unwrap()).unwrap()
    }

    #[test]
    fn nested_mapping_and_sequence_targets_preserve_comments() {
        let source = "jobs:\n  build:\n    steps:\n      - run: cargo test # keep\n";
        assert_eq!(
            set(
                source,
                "jobs.build.steps[0].run",
                serde_json::json!("cargo check")
            ),
            "jobs:\n  build:\n    steps:\n      - run: \"cargo check\" # keep\n"
        );
    }

    #[test]
    fn crlf_and_nested_scalar_are_preserved() {
        let source = "service:\r\n  port: 8080 # local\r\n";
        assert_eq!(
            set(source, "service.port", serde_json::json!(9090)),
            "service:\r\n  port: 9090 # local\r\n"
        );
    }

    #[test]
    fn malformed_and_advanced_yaml_refuse() {
        assert!(matches!(
            validate(b"a: ["),
            Err(YamlError::Refused(RefusalReason::MalformedInput { .. }))
        ));
        assert!(matches!(
            validate(b"a: &x 1\nb: *x\n"),
            Err(YamlError::Refused(
                RefusalReason::PreservationUnavailable { .. }
            ))
        ));
    }
}
