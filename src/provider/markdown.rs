#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MarkdownOperation {
    ReplaceSection { heading: String, content: String },
    EnsureSection { heading: String, content: String },
    DeleteSection { heading: String },
    InsertAfterHeading { heading: String, content: String },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MarkdownError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

pub fn plan(
    content: &[u8],
    op: &MarkdownOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, MarkdownError> {
    let text = std::str::from_utf8(content).map_err(|_| {
        MarkdownError::Refused(RefusalReason::UnsupportedEncoding {
            details: "markdown provider requires UTF-8".into(),
        })
    })?;
    let heading = match op {
        MarkdownOperation::ReplaceSection { heading, .. }
        | MarkdownOperation::EnsureSection { heading, .. }
        | MarkdownOperation::DeleteSection { heading }
        | MarkdownOperation::InsertAfterHeading { heading, .. } => heading,
    };
    let starts: Vec<(usize, usize, usize)> = text
        .lines()
        .enumerate()
        .filter_map(|(line_no, line)| {
            let trimmed = line.trim_start();
            let level = trimmed.bytes().take_while(|b| *b == b'#').count();
            (level > 0
                && trimmed.as_bytes().get(level) == Some(&b' ')
                && trimmed[level + 1..].trim_end() == heading)
                .then_some((
                    line_no,
                    level,
                    line.as_ptr() as usize - text.as_ptr() as usize,
                ))
        })
        .collect();
    if matches!(op, MarkdownOperation::EnsureSection { .. }) && starts.is_empty() {
        let content = match op {
            MarkdownOperation::EnsureSection { content, .. } => content,
            _ => unreachable!(),
        };
        let prefix = if text.is_empty() || text.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        return Ok(vec![ByteEdit {
            start: content.len(),
            end: content.len(),
            replacement: format!("{prefix}{heading}\n{content}\n").into_bytes(),
        }]);
    }
    if starts.len() != 1 {
        return Err(MarkdownError::Refused(if starts.is_empty() {
            RefusalReason::MissingTarget {
                target: heading.clone(),
            }
        } else {
            RefusalReason::DuplicateTarget {
                target: heading.clone(),
                count: starts.len(),
                candidates: Vec::new(),
            }
        }));
    }
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(MarkdownError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one heading".into(),
            actual: starts.len(),
        }));
    }
    let (line_no, level, start) = starts[0];
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let end_line = ((line_no + 1)..text.lines().count())
        .find(|i| {
            let line = text.lines().nth(*i).unwrap_or("").trim_start();
            let next_level = line.bytes().take_while(|b| *b == b'#').count();
            next_level > 0 && next_level <= level && line.as_bytes().get(next_level) == Some(&b' ')
        })
        .unwrap_or(text.lines().count());
    let end = line_starts.get(end_line).copied().unwrap_or(text.len());
    let line_end = line_starts.get(line_no + 1).copied().unwrap_or(text.len());
    let replacement = match op {
        MarkdownOperation::ReplaceSection { content, .. } => format!(
            "{}{}{}",
            &text[start..line_end],
            content,
            if content.ends_with('\n') { "" } else { "\n" }
        )
        .into_bytes(),
        MarkdownOperation::DeleteSection { .. } => Vec::new(),
        MarkdownOperation::InsertAfterHeading { content, .. } => format!(
            "{}{}{}",
            &text[start..line_end],
            content,
            if content.ends_with('\n') { "" } else { "\n" }
        )
        .into_bytes(),
        MarkdownOperation::EnsureSection { content, .. } => format!(
            "{}{}{}",
            &text[start..line_end],
            content,
            if content.ends_with('\n') { "" } else { "\n" }
        )
        .into_bytes(),
    };
    let edit_start = if matches!(op, MarkdownOperation::InsertAfterHeading { .. }) {
        line_end
    } else {
        start
    };
    let edit_end = if matches!(op, MarkdownOperation::InsertAfterHeading { .. }) {
        line_end
    } else {
        end
    };
    Ok(vec![ByteEdit {
        start: edit_start,
        end: edit_end,
        replacement,
    }])
}
