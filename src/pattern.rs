#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason, MAX_FILE_BYTES};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_PATTERN_BYTES: usize = 8_192;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PatternOperation {
    Replace {
        pattern: String,
        replacement: String,
    },
    Delete {
        pattern: String,
    },
    EnsureAbsent {
        pattern: String,
    },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PatternError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
    #[error("pattern error: {message}")]
    Error { message: String },
}

pub fn plan(
    content: &[u8],
    op: &PatternOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, PatternError> {
    if content.len() > MAX_FILE_BYTES {
        return Err(PatternError::Refused(
            RefusalReason::ResourceLimitExceeded {
                dimension: "max_file_bytes".into(),
                limit: MAX_FILE_BYTES,
                actual: content.len(),
            },
        ));
    }
    let pattern = match op {
        PatternOperation::Replace { pattern, .. }
        | PatternOperation::Delete { pattern }
        | PatternOperation::EnsureAbsent { pattern } => pattern,
    };
    if pattern.is_empty() || pattern.len() > MAX_PATTERN_BYTES {
        return Err(PatternError::Refused(if pattern.is_empty() {
            RefusalReason::MissingTarget {
                target: "empty pattern".into(),
            }
        } else {
            RefusalReason::ResourceLimitExceeded {
                dimension: "max_pattern_bytes".into(),
                limit: MAX_PATTERN_BYTES,
                actual: pattern.len(),
            }
        }));
    }
    let re = Regex::new(pattern).map_err(|e| {
        PatternError::Refused(RefusalReason::MalformedInput {
            details: format!("invalid bounded regex: {e}"),
        })
    })?;
    let text = std::str::from_utf8(content).map_err(|_| {
        PatternError::Refused(RefusalReason::UnsupportedEncoding {
            details: "pattern provider requires UTF-8".into(),
        })
    })?;
    let matches: Vec<_> = re.find_iter(text).take(1025).collect();
    if matches.len() > 1024 {
        return Err(PatternError::Refused(RefusalReason::EffectBudgetExceeded {
            dimension: "pattern_matches".into(),
            limit: 1024,
            actual: matches.len(),
        }));
    }
    if matches.is_empty() && matches!(op, PatternOperation::EnsureAbsent { .. }) {
        return Ok(Vec::new());
    }
    enforce_cardinality(matches.len(), cardinality)?;
    let replacement = match op {
        PatternOperation::Replace { replacement, .. } => replacement.as_bytes(),
        PatternOperation::Delete { .. } | PatternOperation::EnsureAbsent { .. } => b"",
    };
    Ok(matches
        .into_iter()
        .map(|m| ByteEdit {
            start: m.start(),
            end: m.end(),
            replacement: replacement.to_vec(),
        })
        .collect())
}

fn enforce_cardinality(actual: usize, cardinality: &Cardinality) -> Result<(), PatternError> {
    let valid = match cardinality {
        Cardinality::ExactlyOne => actual == 1,
        Cardinality::Exactly(n) => actual == *n,
        Cardinality::All => actual > 0,
    };
    if valid {
        return Ok(());
    }
    Err(PatternError::Refused(if actual == 0 {
        RefusalReason::MissingTarget {
            target: "pattern produced no match".into(),
        }
    } else {
        RefusalReason::CardinalityMismatch {
            expected: format!("{cardinality:?}"),
            actual,
        }
    }))
}
