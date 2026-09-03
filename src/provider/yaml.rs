#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

pub fn plan(
    content: &[u8],
    op: &YamlOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, YamlError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(YamlError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one yaml key".into(),
            actual: 1,
        }));
    }
    let text = std::str::from_utf8(content).map_err(|_| {
        YamlError::Refused(RefusalReason::UnsupportedEncoding {
            details: "yaml provider requires UTF-8".into(),
        })
    })?;
    serde_yaml::from_str::<serde_yaml::Value>(text).map_err(|e| {
        YamlError::Refused(RefusalReason::MalformedInput {
            details: format!("malformed YAML: {e}"),
        })
    })?;
    if text
        .lines()
        .any(|line| line.contains('&') || line.contains('*') || line.contains(" <<:"))
    {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable { details: "anchors, aliases, and merge keys require a semantic YAML rewrite; refusing lossy formatting".into() }));
    }
    let path = match op {
        YamlOperation::Set { path, .. }
        | YamlOperation::EnsurePresent { path, .. }
        | YamlOperation::Delete { path }
        | YamlOperation::EnsureAbsent { path } => path,
    };
    if path.contains('.') || path.is_empty() {
        return Err(YamlError::Refused(
            RefusalReason::ProviderCapabilityMissing {
                provider: "yaml".into(),
                capability: "only unique top-level scalar keys are currently source-preserving"
                    .into(),
            },
        ));
    }
    let mut found = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let without_newline = line.trim_end_matches(['\r', '\n']);
        let trimmed = without_newline.trim_start();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            if let Some((key, raw)) = trimmed.split_once(':') {
                if key.trim() == path {
                    found.push((offset, line, raw));
                }
            }
        }
        offset += line.len();
    }
    if found.len() > 1 {
        return Err(YamlError::Refused(RefusalReason::DuplicateTarget {
            target: path.into(),
            count: found.len(),
            candidates: Vec::new(),
        }));
    }
    if found.is_empty() {
        if matches!(op, YamlOperation::EnsureAbsent { .. }) {
            return Ok(Vec::new());
        }
        if let YamlOperation::EnsurePresent { value, .. } = op {
            let encoded = scalar(value)?;
            let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
            let prefix = if text.is_empty() || text.ends_with('\n') || text.ends_with('\r') {
                ""
            } else {
                newline
            };
            let suffix = if text.ends_with('\n') || text.ends_with('\r') {
                newline
            } else {
                ""
            };
            return Ok(vec![ByteEdit {
                start: text.len(),
                end: text.len(),
                replacement: format!("{prefix}{path}: {encoded}{suffix}").into_bytes(),
            }]);
        }
        return Err(YamlError::Refused(RefusalReason::MissingTarget {
            target: path.into(),
        }));
    }
    let (start, line, raw) = found[0];
    let colon = line.find(':').unwrap();
    let value_start = start + colon + 1 + raw.len() - raw.trim_start().len();
    let mut value_end = start + line.trim_end_matches(['\r', '\n']).len();
    let value_text = &line[value_start - start..value_end - start];
    if let Some(comment) = value_text.find(" #") {
        value_end = value_start + comment;
    }
    if raw.trim_start().starts_with('|') || raw.trim_start().starts_with('>') {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable { details: "block or collection YAML scalar is outside the conservative source-preserving subset".into() }));
    }
    match op {
        YamlOperation::Delete { .. } | YamlOperation::EnsureAbsent { .. } => Ok(vec![ByteEdit {
            start,
            end: start + line.len(),
            replacement: Vec::new(),
        }]),
        YamlOperation::Set { value, .. } | YamlOperation::EnsurePresent { value, .. } => {
            let encoded = scalar(value)?;
            if line[value_start - start..value_end - start].trim() == encoded {
                return Ok(Vec::new());
            }
            Ok(vec![ByteEdit {
                start: value_start,
                end: value_end,
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
    serde_yaml::from_str::<serde_yaml::Value>(text).map_err(|e| {
        YamlError::Refused(RefusalReason::MalformedInput {
            details: format!("malformed YAML: {e}"),
        })
    })?;
    if text
        .lines()
        .any(|line| line.contains('&') || line.contains('*') || line.contains(" <<:"))
    {
        return Err(YamlError::Refused(RefusalReason::PreservationUnavailable {
            details: "anchors/aliases are not in the conservative provider subset".into(),
        }));
    }
    Ok(())
}

fn scalar(value: &serde_json::Value) -> Result<String, YamlError> {
    match value {
        serde_json::Value::String(s) => serde_json::to_string(s).map_err(|e| {
            YamlError::Refused(RefusalReason::MalformedInput {
                details: e.to_string(),
            })
        }),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            Ok(value.to_string())
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => serde_json::to_string(value)
            .map_err(|e| {
                YamlError::Refused(RefusalReason::MalformedInput {
                    details: e.to_string(),
                })
            }),
    }
}
