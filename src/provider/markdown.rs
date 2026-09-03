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
    ReplaceListItem { target: String, replacement: String },
    EnsureListItem { target: String, content: String },
    DeleteListItem { target: String },
    ReplaceFencedBlock { info: String, content: String },
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
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(MarkdownError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one heading".into(),
            actual: 1,
        }));
    }
    if matches!(
        op,
        MarkdownOperation::ReplaceListItem { .. }
            | MarkdownOperation::EnsureListItem { .. }
            | MarkdownOperation::DeleteListItem { .. }
    ) {
        return plan_list_item(text_from_bytes(content)?, op);
    }
    if matches!(op, MarkdownOperation::ReplaceFencedBlock { .. }) {
        return plan_fenced_block(text_from_bytes(content)?, op);
    }
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
        _ => unreachable!(),
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
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let prefix = if text.is_empty() || text.ends_with('\n') || text.ends_with('\r') {
            ""
        } else {
            newline
        };
        return Ok(vec![ByteEdit {
            start: text.len(),
            end: text.len(),
            replacement: format!("{prefix}# {heading}{newline}{content}{newline}").into_bytes(),
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
    let (line_no, level, start) = starts[0];
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
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
            if content.ends_with('\n') || content.ends_with('\r') {
                ""
            } else {
                newline
            }
        )
        .into_bytes(),
        MarkdownOperation::DeleteSection { .. } => Vec::new(),
        MarkdownOperation::InsertAfterHeading { content, .. } => format!(
            "{}{}",
            content,
            if content.ends_with('\n') || content.ends_with('\r') {
                ""
            } else {
                newline
            }
        )
        .into_bytes(),
        MarkdownOperation::EnsureSection { content, .. } => format!(
            "{}{}{}",
            &text[start..line_end],
            content,
            if content.ends_with('\n') || content.ends_with('\r') {
                ""
            } else {
                newline
            }
        )
        .into_bytes(),
        MarkdownOperation::ReplaceListItem { .. }
        | MarkdownOperation::EnsureListItem { .. }
        | MarkdownOperation::DeleteListItem { .. }
        | MarkdownOperation::ReplaceFencedBlock { .. } => unreachable!(),
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

fn text_from_bytes(content: &[u8]) -> Result<&str, MarkdownError> {
    std::str::from_utf8(content).map_err(|_| {
        MarkdownError::Refused(RefusalReason::UnsupportedEncoding {
            details: "markdown provider requires UTF-8".into(),
        })
    })
}

fn line_ending(text: &str) -> &str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn list_item_span(text: &str, target: &str) -> Vec<(usize, usize, usize, usize)> {
    let mut matches = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let leading = body.len() - body.trim_start().len();
        let trimmed = &body[leading..];
        let marker_len = if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
        {
            Some(2)
        } else {
            let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
            (digits > 0
                && trimmed
                    .as_bytes()
                    .get(digits)
                    .is_some_and(|byte| *byte == b'.' || *byte == b')')
                && trimmed.as_bytes().get(digits + 1) == Some(&b' '))
            .then_some(digits + 2)
        };
        if let Some(marker_len) = marker_len {
            let item_start = leading + marker_len;
            let item = body[item_start..].trim();
            if item == target {
                let trimmed_start =
                    item_start + body[item_start..].len() - body[item_start..].trim_start().len();
                let target_start = offset + trimmed_start;
                matches.push((
                    offset,
                    offset + line.len(),
                    target_start,
                    target_start + target.len(),
                ));
            }
        }
        offset += line.len();
    }
    matches
}

fn plan_list_item(text: &str, op: &MarkdownOperation) -> Result<Vec<ByteEdit>, MarkdownError> {
    let (target, replacement, ensure) = match op {
        MarkdownOperation::ReplaceListItem {
            target,
            replacement,
        } => (target, Some(replacement), false),
        MarkdownOperation::EnsureListItem { target, content } => (target, Some(content), true),
        MarkdownOperation::DeleteListItem { target } => (target, None, false),
        _ => unreachable!(),
    };
    if target.is_empty() {
        return Err(MarkdownError::Refused(RefusalReason::MissingTarget {
            target: "empty Markdown list item".into(),
        }));
    }
    let matches = list_item_span(text, target);
    if matches.len() > 1 {
        return Err(MarkdownError::Refused(RefusalReason::DuplicateTarget {
            target: target.clone(),
            count: matches.len(),
            candidates: Vec::new(),
        }));
    }
    if matches.is_empty() {
        if ensure {
            let newline = line_ending(text);
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
                replacement: format!("{prefix}- {}{suffix}", replacement.unwrap()).into_bytes(),
            }]);
        }
        return Err(MarkdownError::Refused(RefusalReason::MissingTarget {
            target: target.clone(),
        }));
    }
    let (start, end, target_start, target_end) = matches[0];
    if let Some(replacement) = replacement {
        let mut bytes = text.as_bytes()[start..end].to_vec();
        let local_start = target_start - start;
        let local_end = target_end - start;
        bytes.splice(
            local_start..local_end,
            replacement.as_bytes().iter().copied(),
        );
        Ok(vec![ByteEdit {
            start,
            end,
            replacement: bytes,
        }])
    } else {
        Ok(vec![ByteEdit {
            start,
            end,
            replacement: Vec::new(),
        }])
    }
}

fn plan_fenced_block(text: &str, op: &MarkdownOperation) -> Result<Vec<ByteEdit>, MarkdownError> {
    let MarkdownOperation::ReplaceFencedBlock { info, content } = op else {
        unreachable!()
    };
    if info.is_empty() {
        return Err(MarkdownError::Refused(RefusalReason::MissingTarget {
            target: "empty fenced-block info string".into(),
        }));
    }
    let newline = line_ending(text);
    let mut openings = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let trimmed = body.trim_start();
        let leading = body.len() - trimmed.len();
        let marker = trimmed.as_bytes().first().copied();
        let fence_len = marker
            .filter(|marker| matches!(marker, b'`' | b'~'))
            .map(|marker| trimmed.bytes().take_while(|byte| *byte == marker).count())
            .filter(|length| *length >= 3);
        if let Some(fence_len) = fence_len {
            if trimmed[fence_len..].trim() == info {
                let mut inner_offset = offset + line.len();
                let mut found_close = None;
                for closing in text[offset + line.len()..].split_inclusive('\n') {
                    let closing_body = closing.trim_end_matches(['\r', '\n']).trim_start();
                    let closes = closing_body.bytes().next() == Some(marker.unwrap())
                        && closing_body
                            .bytes()
                            .take_while(|byte| *byte == marker.unwrap())
                            .count()
                            >= fence_len
                        && closing_body[fence_len..].trim().is_empty();
                    if closes {
                        found_close = Some((inner_offset, inner_offset + closing.len()));
                        break;
                    }
                    inner_offset += closing.len();
                }
                if let Some((close_start, close_end)) = found_close {
                    openings.push((offset + leading, close_start, close_end));
                } else {
                    return Err(MarkdownError::Refused(RefusalReason::MalformedInput {
                        details: format!("fenced block '{info}' has no closing fence"),
                    }));
                }
            }
        }
        offset += line.len();
    }
    if openings.is_empty() {
        return Err(MarkdownError::Refused(RefusalReason::MissingTarget {
            target: info.clone(),
        }));
    }
    if openings.len() > 1 {
        return Err(MarkdownError::Refused(RefusalReason::DuplicateTarget {
            target: info.clone(),
            count: openings.len(),
            candidates: Vec::new(),
        }));
    }
    let (_, close_start, _) = openings[0];
    let opening_end = text[openings[0].0..]
        .find('\n')
        .map(|index| openings[0].0 + index + 1)
        .unwrap_or(openings[0].0);
    let mut replacement = content.clone();
    if !replacement.is_empty() && !replacement.ends_with('\n') && !replacement.ends_with('\r') {
        replacement.push_str(newline);
    }
    Ok(vec![ByteEdit {
        start: opening_end,
        end: close_start,
        replacement: replacement.into_bytes(),
    }])
}
