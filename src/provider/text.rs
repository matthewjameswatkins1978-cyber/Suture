#![forbid(unsafe_code)]

use crate::engine::{compute_sha256, ByteEdit};
use crate::protocol::{Candidate, Cardinality, RefusalReason};
use memchr::{memchr_iter, memmem};
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
    pub edits: Vec<ByteEdit>,
    pub original_bytes: Vec<u8>,
    pub modified_bytes: Vec<u8>,
}

pub struct TextProvider;

#[derive(Clone, Copy)]
enum Placement {
    Replace,
    Before,
    After,
}

impl TextProvider {
    pub fn plan(
        content: &[u8],
        op: &TextOperation,
        cardinality: &Cardinality,
    ) -> Result<Vec<ByteEdit>, TextProviderError> {
        match op {
            TextOperation::EnsurePresent { content: wanted } => {
                return plan_ensure_present(content, wanted)
            }
            TextOperation::Move { target, before } => {
                return plan_move(content, target, before, cardinality)
            }
            _ => {}
        }

        let (target, replacement, placement, desired_absence, set_like) = match op {
            TextOperation::Replace {
                target,
                replacement,
            } => (
                target.as_bytes(),
                replacement.as_bytes(),
                Placement::Replace,
                false,
                false,
            ),
            TextOperation::Delete { target } => (
                target.as_bytes(),
                &[][..],
                Placement::Replace,
                false,
                false,
            ),
            TextOperation::InsertBefore { target, content } => (
                target.as_bytes(),
                content.as_bytes(),
                Placement::Before,
                false,
                false,
            ),
            TextOperation::InsertAfter { target, content } => (
                target.as_bytes(),
                content.as_bytes(),
                Placement::After,
                false,
                false,
            ),
            TextOperation::EnsureAbsent { target } | TextOperation::Unset { target } => (
                target.as_bytes(),
                &[][..],
                Placement::Replace,
                true,
                false,
            ),
            TextOperation::Set {
                target,
                replacement,
            }
            | TextOperation::Rename {
                target,
                replacement,
            } => (
                target.as_bytes(),
                replacement.as_bytes(),
                Placement::Replace,
                false,
                true,
            ),
            TextOperation::Move { .. } | TextOperation::EnsurePresent { .. } => unreachable!(),
        };

        if target.is_empty() {
            return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
                target: String::new(),
            }));
        }

        let matches = find_matches(content, target);
        let match_count = matches.len();

        if desired_absence && match_count == 0 {
            return Ok(Vec::new());
        }

        // For desired-state set/rename operations, an already-present non-empty
        // replacement is a safe no-op. An empty replacement is not treated as
        // "present everywhere": without the target we cannot establish the
        // requested state, so normal missing-target refusal applies.
        if set_like
            && match_count == 0
            && !replacement.is_empty()
            && memmem::find(content, replacement).is_some()
        {
            return Ok(Vec::new());
        }

        enforce_cardinality(content, target, &matches, cardinality, desired_absence)?;

        let replacement = build_replacement(target, replacement, placement);
        Ok(matches
            .into_iter()
            .map(|start| ByteEdit {
                start,
                end: start + target.len(),
                replacement: replacement.clone(),
            })
            .collect())
    }
}

fn plan_ensure_present(content: &[u8], wanted: &str) -> Result<Vec<ByteEdit>, TextProviderError> {
    if wanted.is_empty() {
        return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
            target: "empty ensure_present content".into(),
        }));
    }

    if memmem::find(content, wanted.as_bytes()).is_some() {
        return Ok(Vec::new());
    }

    let newline: &[u8] = if memmem::find(content, b"\r\n").is_some() {
        b"\r\n"
    } else {
        b"\n"
    };
    let leading_newline = !content.is_empty() && !content.ends_with(b"\n");
    let trailing_newline = content.ends_with(b"\n");
    let capacity = wanted.len()
        + usize::from(leading_newline) * newline.len()
        + usize::from(trailing_newline) * newline.len();
    let mut replacement = Vec::with_capacity(capacity);

    if leading_newline {
        replacement.extend_from_slice(newline);
    }
    replacement.extend_from_slice(wanted.as_bytes());
    if trailing_newline {
        replacement.extend_from_slice(newline);
    }

    Ok(vec![ByteEdit {
        start: content.len(),
        end: content.len(),
        replacement,
    }])
}

fn build_replacement(target: &[u8], replacement: &[u8], placement: Placement) -> Vec<u8> {
    match placement {
        Placement::Replace => replacement.to_vec(),
        Placement::Before => {
            let mut bytes = Vec::with_capacity(replacement.len() + target.len());
            bytes.extend_from_slice(replacement);
            bytes.extend_from_slice(target);
            bytes
        }
        Placement::After => {
            let mut bytes = Vec::with_capacity(target.len() + replacement.len());
            bytes.extend_from_slice(target);
            bytes.extend_from_slice(replacement);
            bytes
        }
    }
}

fn enforce_cardinality(
    content: &[u8],
    target: &[u8],
    matches: &[usize],
    cardinality: &Cardinality,
    desired_absence: bool,
) -> Result<(), TextProviderError> {
    if desired_absence {
        return Ok(());
    }

    let actual = matches.len();
    let valid = match cardinality {
        Cardinality::ExactlyOne => actual == 1,
        Cardinality::Exactly(expected) => actual == *expected,
        Cardinality::All => actual > 0,
    };
    if valid {
        return Ok(());
    }

    if actual == 0 {
        return Err(TextProviderError::Refused(RefusalReason::MissingTarget {
            target: diagnose_near_miss(content, target),
        }));
    }

    Err(TextProviderError::Refused(RefusalReason::DuplicateTarget {
        target: String::from_utf8_lossy(target).into_owned(),
        count: actual,
        candidates: candidate_diagnostics(content, target, matches),
    }))
}

fn find_matches(content: &[u8], target: &[u8]) -> Vec<usize> {
    if target.is_empty() || target.len() > content.len() {
        return Vec::new();
    }
    memmem::find_iter(content, target).collect()
}

fn plan_move(
    content: &[u8],
    target: &str,
    before: &str,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, TextProviderError> {
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

    let mut edits = vec![
        ByteEdit {
            start: destination,
            end: destination,
            replacement: target.as_bytes().to_vec(),
        },
        ByteEdit {
            start: source,
            end: source_end,
            replacement: Vec::new(),
        },
    ];
    edits.sort_unstable_by_key(|edit| edit.start);
    Ok(edits)
}

fn diagnose_near_miss(content: &[u8], target: &[u8]) -> String {
    let target_str = String::from_utf8_lossy(target);
    let content_str = String::from_utf8_lossy(content);

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

    if target_str.contains('\t') && content_str.contains("    ") {
        let alt = target_str.replace('\t', "    ");
        if content_str.contains(&alt) {
            return "Target not found exact; detected Tab vs Spaces indentation discrepancy."
                .to_string();
        }
    }

    if content_str.contains('\u{200B}') {
        return "Target not found exact; content contains Zero-Width Space (U+200B).".to_string();
    }

    if content.starts_with(&[0xEF, 0xBB, 0xBF]) && !target.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return "Target not found exact; file starts with UTF-8 BOM (EF BB BF) while target does not."
            .to_string();
    }

    format!("Target not found: '{target_str}'")
}

fn candidate_diagnostics(content: &[u8], target: &[u8], matches: &[usize]) -> Vec<Candidate> {
    matches
        .iter()
        .take(8)
        .map(|&offset| {
            let context_start = offset.saturating_sub(24);
            let context_end = (offset + target.len() + 24).min(content.len());
            Candidate {
                offset,
                line: memchr_iter(b'\n', &content[..offset]).count() + 1,
                context: String::from_utf8_lossy(&content[context_start..context_end]).into_owned(),
                anchor_sha256: compute_sha256(&content[offset..offset + target.len()]),
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
    fn empty_set_replacement_does_not_panic_or_fake_a_noop() {
        let content = b"hello world";
        let op = TextOperation::Set {
            target: "missing".to_string(),
            replacement: String::new(),
        };
        let result = TextProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            result,
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
    fn exact_matching_remains_non_overlapping() {
        let content = b"aaaa";
        let op = TextOperation::Replace {
            target: "aa".into(),
            replacement: "b".into(),
        };
        let edits = TextProvider::plan(content, &op, &Cardinality::All).unwrap();
        assert_eq!(edits.iter().map(|edit| edit.start).collect::<Vec<_>>(), [0, 2]);
        assert_eq!(apply_byte_edits(content, &edits).unwrap(), b"bb");
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
