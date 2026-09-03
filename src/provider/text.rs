#![forbid(unsafe_code)]

use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TextOperation {
    Replace { target: String, replacement: String },
    InsertBefore { target: String, content: String },
    InsertAfter { target: String, content: String },
    Delete { target: String },
    Move { target: String, before: String },
    EnsurePresent { content: String },
    EnsureAbsent { target: String },
    Set { target: String, replacement: String },
    Unset { target: String },
    Rename { target: String, replacement: String },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TextProviderError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
    #[error("Error: {message}")]
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextProviderResult {
    pub edits: Vec<crate::engine::ByteEdit>,
    pub original_bytes: Vec<u8>,
    pub modified_bytes: Vec<u8>,
}

pub struct TextProvider;

impl TextProvider {
    pub fn plan(
        content: &[u8],
        op: &TextOperation,
        cardinality: &Cardinality,
    ) -> Result<Vec<crate::engine::ByteEdit>, TextProviderError> {
        if let TextOperation::EnsurePresent { content: wanted } = op {
            if wanted.is_empty() {
                return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                    target: "empty ensure_present content".into(),
                }));
            }
            if content
                .windows(wanted.len())
                .any(|w| w == wanted.as_bytes())
            {
                return Ok(Vec::new());
            }
            let newline = if content.windows(2).any(|w| w == b"\r\n") {
                b"\r\n".as_slice()
            } else {
                b"\n".as_slice()
            };
            let mut replacement = Vec::new();
            if !content.is_empty() && !content.ends_with(b"\n") {
                replacement.extend_from_slice(newline);
            }
            replacement.extend_from_slice(wanted.as_bytes());
            if content.ends_with(b"\n") {
                replacement.extend_from_slice(newline);
            }
            return Ok(vec![crate::engine::ByteEdit {
                start: content.len(),
                end: content.len(),
                replacement,
            }]);
        }
        if let TextOperation::Move { target, before } = op {
            return plan_move(content, target, before, cardinality);
        }
        let (target, _is_delete, replacement_bytes, is_insert_before, is_insert_after) = match op {
            TextOperation::Replace {
                target,
                replacement,
            } => (
                target.as_bytes(),
                false,
                replacement.as_bytes().to_vec(),
                false,
                false,
            ),
            TextOperation::Delete { target } => (target.as_bytes(), true, Vec::new(), false, false),
            TextOperation::InsertBefore { target, content } => (
                target.as_bytes(),
                false,
                content.as_bytes().to_vec(),
                true,
                false,
            ),
            TextOperation::InsertAfter { target, content } => (
                target.as_bytes(),
                false,
                content.as_bytes().to_vec(),
                false,
                true,
            ),
            TextOperation::EnsureAbsent { target } | TextOperation::Unset { target } => {
                (target.as_bytes(), true, Vec::new(), false, false)
            }
            TextOperation::Set {
                target,
                replacement,
            }
            | TextOperation::Rename {
                target,
                replacement,
            } => (
                target.as_bytes(),
                false,
                replacement.as_bytes().to_vec(),
                false,
                false,
            ),
            TextOperation::Move { .. } | TextOperation::EnsurePresent { .. } => unreachable!(),
        };

        if target.is_empty() {
            return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                target: String::new(),
            }));
        }

        // Find all non-overlapping matches of target in content
        let mut matches = Vec::new();
        let mut search_idx = 0;
        while search_idx <= content.len() {
            if search_idx + target.len() > content.len() {
                break;
            }
            if &content[search_idx..search_idx + target.len()] == target {
                matches.push(search_idx);
                search_idx += target.len(); // non-overlapping advance
            } else {
                search_idx += 1;
            }
        }

        let match_count = matches.len();

        if matches!(
            op,
            TextOperation::EnsureAbsent { .. } | TextOperation::Unset { .. }
        ) && match_count == 0
        {
            return Ok(Vec::new());
        }
        if matches!(op, TextOperation::Set { .. } | TextOperation::Rename { .. })
            && match_count == 0
        {
            let replacement_present = content
                .windows(replacement_bytes.len())
                .any(|w| w == replacement_bytes.as_slice());
            if replacement_present {
                return Ok(Vec::new());
            }
        }

        // Enforce cardinality
        match cardinality {
            Cardinality::ExactlyOne => {
                if matches!(
                    op,
                    TextOperation::EnsureAbsent { .. } | TextOperation::Unset { .. }
                ) {
                    // Desired-state absence is deliberately idempotent and removes every match.
                } else {
                    if match_count == 0 {
                        let near_miss = diagnose_near_miss(content, target);
                        return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                            target: near_miss,
                        }));
                    } else if match_count > 1 {
                        return Err(TextProviderError::Refused(RefusalReason::DuplicateTarget {
                            target: String::from_utf8_lossy(target).to_string(),
                            count: match_count,
                            candidates: candidate_diagnostics(content, target, &matches),
                        }));
                    }
                }
            }
            Cardinality::Exactly(n) => {
                if matches!(
                    op,
                    TextOperation::EnsureAbsent { .. } | TextOperation::Unset { .. }
                ) {
                    // Desired-state absence is deliberately idempotent and removes every match.
                } else {
                    if match_count < *n {
                        let near_miss = diagnose_near_miss(content, target);
                        return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                            target: near_miss,
                        }));
                    } else if match_count > *n {
                        return Err(TextProviderError::Refused(RefusalReason::DuplicateTarget {
                            target: String::from_utf8_lossy(target).to_string(),
                            count: match_count,
                            candidates: candidate_diagnostics(content, target, &matches),
                        }));
                    }
                }
            }
            Cardinality::All => {
                if match_count == 0 {
                    let near_miss = diagnose_near_miss(content, target);
                    return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                        target: near_miss,
                    }));
                }
            }
        }

        // Generate ByteEdit slices
        let mut edits = Vec::new();
        for &start in &matches {
            let end = start + target.len();
            let final_replacement = if is_insert_before {
                let mut rep = replacement_bytes.clone();
                rep.extend_from_slice(target);
                rep
            } else if is_insert_after {
                let mut rep = target.to_vec();
                rep.extend_from_slice(&replacement_bytes);
                rep
            } else {
                replacement_bytes.clone()
            };

            edits.push(crate::engine::ByteEdit {
                start,
                end,
                replacement: final_replacement,
            });
        }

        // Ensure edits are sorted (matches are naturally sorted by start offset)
        edits.sort_by_key(|e| e.start);

        Ok(edits)
    }
}

fn find_matches(content: &[u8], target: &[u8]) -> Vec<usize> {
    if target.is_empty() || target.len() > content.len() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    let mut at = 0;
    while at + target.len() <= content.len() {
        if &content[at..at + target.len()] == target {
            matches.push(at);
            at += target.len();
        } else {
            at += 1;
        }
    }
    matches
}

fn plan_move(
    content: &[u8],
    target: &str,
    before: &str,
    cardinality: &Cardinality,
) -> Result<Vec<crate::engine::ByteEdit>, TextProviderError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(TextProviderError::Refused(
            RefusalReason::CardinalityMismatch {
                expected: "exactly_one (move anchors are unique)".into(),
                actual: 1,
            },
        ));
    }
    let targets = find_matches(content, target.as_bytes());
    let destinations = find_matches(content, before.as_bytes());
    if targets.len() != 1 || destinations.len() != 1 || target.is_empty() || before.is_empty() {
        return Err(TextProviderError::Refused(
            RefusalReason::CardinalityMismatch {
                expected: "one target and one destination".into(),
                actual: targets.len().saturating_add(destinations.len()),
            },
        ));
    }
    let source = targets[0];
    let source_end = source + target.len();
    let destination = destinations[0];
    if source <= destination && destination <= source_end {
        return Ok(Vec::new());
    }
    let insertion = destination;
    let mut edits = vec![
        crate::engine::ByteEdit {
            start: insertion,
            end: insertion,
            replacement: target.as_bytes().to_vec(),
        },
        crate::engine::ByteEdit {
            start: source,
            end: source_end,
            replacement: Vec::new(),
        },
    ];
    edits.sort_by_key(|edit| edit.start);
    Ok(edits)
}

fn diagnose_near_miss(content: &[u8], target: &[u8]) -> String {
    let target_str = String::from_utf8_lossy(target);
    let content_str = String::from_utf8_lossy(content);

    // Check for common invisible / near-miss discrepancies
    // 1. NBSP (U+00A0) vs space (U+0020)
    if target_str.contains(' ') && content_str.contains('\u{00A0}') {
        let alt = target_str.replace(' ', "\u{00A0}");
        if content_str.contains(&alt) {
            return "Target not found exact; detected Non-Breaking Space (U+00A0) discrepancy. Try replacing spaces with NBSP.".to_string();
        }
    }
    if target_str.contains('\u{00A0}') && content_str.contains(' ') {
        let alt = target_str.replace('\u{00A0}', " ");
        if content_str.contains(&alt) {
            return "Target not found exact; detected space vs Non-Breaking Space (U+00A0) discrepancy.".to_string();
        }
    }

    // 2. CRLF vs LF
    if target_str.contains('\n') && !target_str.contains("\r\n") && content_str.contains("\r\n") {
        let alt = target_str.replace('\n', "\r\n");
        if content_str.contains(&alt) {
            return "Target not found exact; detected CRLF vs LF line ending discrepancy."
                .to_string();
        }
    }
    if target_str.contains("\r\n") && content_str.contains('\n') && !content_str.contains("\r\n") {
        let alt = target_str.replace("\r\n", "\n");
        if content_str.contains(&alt) {
            return "Target not found exact; detected LF vs CRLF line ending discrepancy."
                .to_string();
        }
    }

    // 3. Tabs vs spaces
    if target_str.contains('\t') && content_str.contains("    ") {
        let alt = target_str.replace('\t', "    ");
        if content_str.contains(&alt) {
            return "Target not found exact; detected Tab vs Spaces indentation discrepancy."
                .to_string();
        }
    }

    // 4. Zero-width space (U+200B)
    if content_str.contains('\u{200B}') {
        return "Target not found exact; content contains Zero-Width Space (U+200B).".to_string();
    }

    // 5. BOM (Byte Order Mark U+FEFF)
    if content.starts_with(&[0xEF, 0xBB, 0xBF]) && !target.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return "Target not found exact; file starts with UTF-8 BOM (EF BB BF) while target does not.".to_string();
    }

    format!("Target not found: '{}'", target_str)
}

fn candidate_diagnostics(
    content: &[u8],
    target: &[u8],
    matches: &[usize],
) -> Vec<crate::protocol::Candidate> {
    matches
        .iter()
        .take(8)
        .map(|&offset| {
            let line = content[..offset].iter().filter(|&&b| b == b'\n').count() + 1;
            let context_start = offset.saturating_sub(24);
            let context_end = (offset + target.len() + 24).min(content.len());
            crate::protocol::Candidate {
                offset,
                line,
                context: String::from_utf8_lossy(&content[context_start..context_end]).into_owned(),
                anchor_sha256: crate::engine::compute_sha256(
                    &content[offset..offset + target.len()],
                ),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply_byte_edits;

    #[test]
    fn test_replace_single_unique() {
        let content = b"Hello world, welcome to Suture.";
        let op = TextOperation::Replace {
            target: "world".to_string(),
            replacement: "universe".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        assert_eq!(modified, b"Hello universe, welcome to Suture.");
    }

    #[test]
    fn test_insert_before() {
        let content = b"target line\n";
        let op = TextOperation::InsertBefore {
            target: "target line".to_string(),
            content: "prefix: ".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        assert_eq!(modified, b"prefix: target line\n");
    }

    #[test]
    fn test_insert_after() {
        let content = b"target line\n";
        let op = TextOperation::InsertAfter {
            target: "target line".to_string(),
            content: " suffix".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        assert_eq!(modified, b"target line suffix\n");
    }

    #[test]
    fn test_delete() {
        let content = b"remove this part carefully.\n";
        let op = TextOperation::Delete {
            target: "this part ".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        assert_eq!(modified, b"remove carefully.\n");
    }

    #[test]
    fn test_duplicate_target_refusal() {
        let content = b"foo and foo and foo";
        let op = TextOperation::Replace {
            target: "foo".to_string(),
            replacement: "bar".to_string(),
        };
        let res = TextProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            res,
            Err(TextProviderError::Refused(RefusalReason::DuplicateTarget {
                count: 3,
                ..
            }))
        ));
    }

    #[test]
    fn test_missing_target_refusal() {
        let content = b"hello world";
        let op = TextOperation::Replace {
            target: "missing".to_string(),
            replacement: "bar".to_string(),
        };
        let res = TextProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            res,
            Err(TextProviderError::Refused(
                RefusalReason::MissingTarget { .. }
            ))
        ));
    }

    #[test]
    fn test_near_miss_nbsp() {
        let content = "hello\u{00A0}world".as_bytes();
        let op = TextOperation::Replace {
            target: "hello world".to_string(),
            replacement: "hi".to_string(),
        };
        let res = TextProvider::plan(content, &op, &Cardinality::ExactlyOne);
        match res {
            Err(TextProviderError::Refused(RefusalReason::MissingTarget { target })) => {
                assert!(target.contains("Non-Breaking Space"));
            }
            _ => panic!("Expected near-miss NBSP refusal"),
        }
    }

    #[test]
    fn test_near_miss_crlf() {
        let content = b"line1\r\nline2\r\n";
        let op = TextOperation::Replace {
            target: "line1\nline2".to_string(),
            replacement: "replaced".to_string(),
        };
        let res = TextProvider::plan(content, &op, &Cardinality::ExactlyOne);
        match res {
            Err(TextProviderError::Refused(RefusalReason::MissingTarget { target })) => {
                assert!(target.contains("CRLF"));
            }
            _ => panic!("Expected near-miss CRLF refusal"),
        }
    }

    #[test]
    fn test_cardinality_all_bulk_replace() {
        let content = b"apple banana apple orange apple";
        let op = TextOperation::Replace {
            target: "apple".to_string(),
            replacement: "fruit".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::All).unwrap();
        assert_eq!(edits.len(), 3);
        let modified = apply_byte_edits(content, &edits).unwrap();
        assert_eq!(modified, b"fruit banana fruit orange fruit");
    }

    #[test]
    fn test_bom_and_crlf_preservation() {
        let content = b"\xEF\xBB\xBFline1\r\nline2\r\n";
        let op = TextOperation::Replace {
            target: "line2".to_string(),
            replacement: "modified2".to_string(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        // Check that BOM and CRLF are preserved
        assert_eq!(modified, b"\xEF\xBB\xBFline1\r\nmodified2\r\n");
    }

    #[test]
    fn moving_an_already_adjacent_target_is_no_change() {
        let content = b"A B C";
        let op = TextOperation::Move {
            target: "B ".into(),
            before: "C".into(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        assert!(edits.is_empty());
    }
}
