#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DotenvOperation {
    Set { key: String, value: String },
    Unset { key: String },
    EnsurePresent { key: String, value: String },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DotenvError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

pub fn plan(
    content: &[u8],
    operation: &DotenvOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, DotenvError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(DotenvError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one dotenv key".into(),
            actual: 1,
        }));
    }
    let text = std::str::from_utf8(content).map_err(|_| {
        DotenvError::Refused(RefusalReason::UnsupportedEncoding {
            details: "dotenv provider requires UTF-8".into(),
        })
    })?;
    let key = match operation {
        DotenvOperation::Set { key, .. }
        | DotenvOperation::Unset { key }
        | DotenvOperation::EnsurePresent { key, .. } => key,
    };
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err(DotenvError::Refused(RefusalReason::MissingTarget {
            target: "invalid dotenv key".into(),
        }));
    }
    let mut matches = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let trimmed = body.trim_start();
        let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        if let Some((found_key, _)) = without_export.split_once('=') {
            if found_key.trim() == key {
                matches.push((offset, line, without_export.find('=').unwrap()));
            }
        }
        offset += line.len();
    }
    if matches.len() > 1 {
        return Err(DotenvError::Refused(RefusalReason::DuplicateTarget {
            target: key.clone(),
            count: matches.len(),
            candidates: Vec::new(),
        }));
    }
    if matches.is_empty() {
        if let DotenvOperation::EnsurePresent { value, .. } = operation {
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
                start: content.len(),
                end: content.len(),
                replacement: format!("{prefix}{key}={}{suffix}", quote_if_needed(value))
                    .into_bytes(),
            }]);
        }
        if matches!(operation, DotenvOperation::Unset { .. }) {
            return Ok(Vec::new());
        }
        return Err(DotenvError::Refused(RefusalReason::MissingTarget {
            target: key.clone(),
        }));
    }
    let (start, line, eq) = matches[0];
    let line_end = start + line.trim_end_matches(['\r', '\n']).len();
    let value_start = start + line.find('=').unwrap_or(eq) + 1;
    if matches!(operation, DotenvOperation::Unset { .. }) {
        return Ok(vec![ByteEdit {
            start,
            end: start + line.len(),
            replacement: Vec::new(),
        }]);
    }
    let value = match operation {
        DotenvOperation::Set { value, .. } | DotenvOperation::EnsurePresent { value, .. } => value,
        DotenvOperation::Unset { .. } => unreachable!(),
    };
    if value.contains(['\r', '\n', '\0']) {
        return Err(DotenvError::Refused(RefusalReason::MalformedInput {
            details: "dotenv values may not contain line breaks or NUL bytes".into(),
        }));
    }
    let comment_end = line[value_start - start..line_end - start]
        .find(" #")
        .map(|i| value_start + i)
        .unwrap_or(line_end);
    let encoded = quote_if_needed(value);
    if line[value_start - start..comment_end - start].trim() == encoded {
        return Ok(Vec::new());
    }
    Ok(vec![ByteEdit {
        start: value_start,
        end: comment_end,
        replacement: encoded.into_bytes(),
    }])
}

fn quote_if_needed(value: &str) -> String {
    if value
        .bytes()
        .all(|b| !b.is_ascii_whitespace() && b != b'#' && b != b'"' && b != b'\'')
    {
        value.into()
    } else {
        format!("\"{}\"", value.replace('"', "\\\""))
    }
}
