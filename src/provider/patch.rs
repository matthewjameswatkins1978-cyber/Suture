#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PatchOperation {
    UnifiedDiff { patch: String },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PatchError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

pub fn plan(
    content: &[u8],
    operation: &PatchOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, PatchError> {
    plan_inner(content, operation, cardinality, None)
}

pub fn plan_with_path(
    content: &[u8],
    operation: &PatchOperation,
    cardinality: &Cardinality,
    expected_path: &str,
) -> Result<Vec<ByteEdit>, PatchError> {
    plan_inner(content, operation, cardinality, Some(expected_path))
}

fn plan_inner(
    content: &[u8],
    operation: &PatchOperation,
    cardinality: &Cardinality,
    expected_path: Option<&str>,
) -> Result<Vec<ByteEdit>, PatchError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(PatchError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one patch document".into(),
            actual: 1,
        }));
    }
    let PatchOperation::UnifiedDiff { patch } = operation;
    let lines: Vec<&str> = patch.split_inclusive('\n').collect();
    let file_headers = lines.iter().filter(|line| line.starts_with("--- ")).count();
    if file_headers != 1 {
        return Err(PatchError::Refused(RefusalReason::Custom {
            message: "strict patch requires exactly one file".into(),
        }));
    }
    let plus = lines
        .iter()
        .position(|line| line.starts_with("+++ "))
        .ok_or_else(|| malformed("missing +++ file header"))?;
    if plus == 0 {
        return Err(malformed("missing --- file header"));
    }
    if let Some(expected_path) = expected_path {
        let source_path = header_path(lines[plus - 1], "---")?;
        let destination_path = header_path(lines[plus], "+++")?;
        let expected_path = normalize_patch_path(expected_path);
        if source_path == "/dev/null"
            || destination_path == "/dev/null"
            || source_path != destination_path
            || source_path != expected_path
        {
            return Err(malformed(&format!(
                "patch paths do not exactly target '{expected_path}'"
            )));
        }
    }
    let source = std::str::from_utf8(content).map_err(|_| {
        PatchError::Refused(RefusalReason::UnsupportedEncoding {
            details: "patch provider requires UTF-8".into(),
        })
    })?;
    let source_lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut edits = Vec::new();
    let mut i = plus + 1;
    while i < lines.len() {
        if !lines[i].starts_with("@@ ") {
            return Err(malformed("unexpected text outside unified-diff hunks"));
        }
        let (old_start, old_count) = parse_range(lines[i], '-')?;
        let (_, new_count) = parse_range(lines[i], '+')?;
        i += 1;
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();
        while i < lines.len() && !lines[i].starts_with("@@ ") && !lines[i].starts_with("--- ") {
            let line = lines[i];
            if line.trim_end_matches(['\r', '\n']) == "\\ No newline at end of file" {
                i += 1;
                continue;
            }
            let (prefix, value) = line.split_at(1);
            match prefix {
                " " => {
                    old_lines.push(value);
                    new_lines.push(value);
                }
                "-" => old_lines.push(value),
                "+" => new_lines.push(value),
                _ => return Err(malformed("invalid unified-diff hunk line")),
            }
            i += 1;
        }
        if old_lines.len() != old_count || new_lines.len() != new_count {
            return Err(malformed("unified-diff hunk counts do not match body"));
        }
        let start_index = old_start.saturating_sub(1);
        if start_index > source_lines.len()
            || source_lines
                .get(start_index..start_index + old_lines.len())
                .is_none()
        {
            return Err(RefusalReason::MissingTarget {
                target: format!("patch hunk line {old_start}"),
            }
            .into());
        }
        let actual = &source_lines[start_index..start_index + old_lines.len()];
        if actual != old_lines.as_slice() {
            return Err(RefusalReason::StaleIdentity {
                expected_hash: "exact patch preimage".into(),
                actual_hash: "current file differs from hunk context".into(),
            }
            .into());
        }
        let start = source_lines[..start_index]
            .iter()
            .map(|line| line.len())
            .sum();
        let end = start + actual.iter().map(|line| line.len()).sum::<usize>();
        if edits
            .iter()
            .any(|edit: &ByteEdit| start < edit.end || (start == edit.start && start == edit.end))
        {
            return Err(malformed("overlapping or duplicate unified-diff hunks"));
        }
        edits.push(ByteEdit {
            start,
            end,
            replacement: new_lines.concat().into_bytes(),
        });
    }
    if edits.is_empty() {
        return Err(malformed("patch contains no hunks"));
    }
    Ok(edits)
}

fn header_path(line: &str, marker: &str) -> Result<String, PatchError> {
    let path = line
        .strip_prefix(&format!("{marker} "))
        .and_then(|rest| rest.split_whitespace().next())
        .ok_or_else(|| malformed("invalid unified-diff file header"))?;
    Ok(normalize_patch_path(path))
}

fn normalize_patch_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = path.strip_prefix("a/").unwrap_or(&path);
    let path = path.strip_prefix("b/").unwrap_or(path);
    path.trim_start_matches("./").into()
}

fn parse_range(line: &str, prefix: char) -> Result<(usize, usize), PatchError> {
    let marker = format!(" {prefix}");
    let value = line
        .split_whitespace()
        .find(|part| part.starts_with(prefix))
        .ok_or_else(|| malformed("missing hunk range"))?;
    let value = value.trim_start_matches(prefix);
    let mut parts = value.split(',');
    let start = parts
        .next()
        .ok_or_else(|| malformed("missing hunk start"))?
        .parse()
        .map_err(|_| malformed("invalid hunk start"))?;
    let count = parts
        .next()
        .unwrap_or("1")
        .parse()
        .map_err(|_| malformed("invalid hunk count"))?;
    let _ = marker;
    Ok((start, count))
}
fn malformed(message: &str) -> PatchError {
    PatchError::Refused(RefusalReason::MalformedInput {
        details: message.into(),
    })
}
impl From<RefusalReason> for PatchError {
    fn from(reason: RefusalReason) -> Self {
        PatchError::Refused(reason)
    }
}
