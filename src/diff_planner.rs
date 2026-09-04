#![forbid(unsafe_code)]

use crate::engine::{apply_byte_edits, ByteEdit, EngineError};
use thiserror::Error;

/// Planning is deliberately bounded before any diff tokenisation occurs.
pub const MAX_PLANNER_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_PLANNER_REGIONS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPlan {
    pub edits: Vec<ByteEdit>,
    pub derived_region_count: usize,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DiffPlannerError {
    #[error("planner resource limit: {dimension} {actual} exceeds {limit}")]
    ResourceLimit {
        dimension: &'static str,
        limit: usize,
        actual: usize,
    },
    #[error("planner invariant failure: {0}")]
    Invariant(String),
}

pub fn plan(observed: &[u8], desired: &[u8]) -> Result<DiffPlan, DiffPlannerError> {
    let input_size =
        observed
            .len()
            .checked_add(desired.len())
            .ok_or(DiffPlannerError::ResourceLimit {
                dimension: "planner_input_bytes",
                limit: MAX_PLANNER_INPUT_BYTES,
                actual: usize::MAX,
            })?;
    if input_size > MAX_PLANNER_INPUT_BYTES {
        return Err(DiffPlannerError::ResourceLimit {
            dimension: "planner_input_bytes",
            limit: MAX_PLANNER_INPUT_BYTES,
            actual: input_size,
        });
    }
    if observed == desired {
        return Ok(DiffPlan {
            edits: Vec::new(),
            derived_region_count: 0,
        });
    }

    let old_lines = split_inclusive_lines(observed);
    let new_lines = split_inclusive_lines(desired);
    let old_offsets = offsets(&old_lines);
    let new_offsets = offsets(&new_lines);
    let old_refs: Vec<&[u8]> = old_lines.to_vec();
    let new_refs: Vec<&[u8]> = new_lines.to_vec();
    let line_diff = similar::TextDiff::configure()
        .algorithm(similar::Algorithm::Myers)
        .diff_slices(&old_refs, &new_refs);

    let mut edits = Vec::new();
    for op in line_diff.ops() {
        if matches!(op, similar::DiffOp::Equal { .. }) {
            continue;
        }
        let old_start = old_offsets[op.old_range().start];
        let old_end = old_offsets[op.old_range().end];
        let new_start = new_offsets[op.new_range().start];
        let new_end = new_offsets[op.new_range().end];
        refine_region(
            observed, desired, old_start, old_end, new_start, new_end, &mut edits,
        )?;
        if edits.len() > MAX_PLANNER_REGIONS {
            return Err(DiffPlannerError::ResourceLimit {
                dimension: "derived_regions",
                limit: MAX_PLANNER_REGIONS,
                actual: edits.len(),
            });
        }
    }

    edits.sort_by_key(|edit| (edit.start, edit.end));
    if edits.windows(2).any(|pair| pair[0].end > pair[1].start) {
        return Err(DiffPlannerError::Invariant(
            "refinement produced overlapping edits".into(),
        ));
    }
    let candidate = apply_byte_edits(observed, &edits)
        .map_err(|error| DiffPlannerError::Invariant(format!("cannot apply plan: {error}")))?;
    if candidate != desired {
        return Err(DiffPlannerError::Invariant(
            "derived edits do not produce desired bytes".into(),
        ));
    }
    Ok(DiffPlan {
        derived_region_count: edits.len(),
        edits,
    })
}

fn split_inclusive_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            lines.push(&bytes[start..=index]);
            start = index + 1;
        }
    }
    if start < bytes.len() {
        lines.push(&bytes[start..]);
    }
    lines
}

fn offsets(lines: &[&[u8]]) -> Vec<usize> {
    let mut result = Vec::with_capacity(lines.len() + 1);
    result.push(0);
    for line in lines {
        result.push(result.last().copied().unwrap_or_default() + line.len());
    }
    result
}

fn refine_region(
    observed: &[u8],
    desired: &[u8],
    old_start: usize,
    old_end: usize,
    new_start: usize,
    new_end: usize,
    edits: &mut Vec<ByteEdit>,
) -> Result<(), DiffPlannerError> {
    let old = &observed[old_start..old_end];
    let new = &desired[new_start..new_end];
    let old_tokens: Vec<&[u8]> = old
        .iter()
        .enumerate()
        .map(|(i, _)| &old[i..i + 1])
        .collect();
    let new_tokens: Vec<&[u8]> = new
        .iter()
        .enumerate()
        .map(|(i, _)| &new[i..i + 1])
        .collect();
    let byte_diff = similar::TextDiff::configure()
        .algorithm(similar::Algorithm::Myers)
        .diff_slices(&old_tokens, &new_tokens);
    for op in byte_diff.ops() {
        if matches!(op, similar::DiffOp::Equal { .. }) {
            continue;
        }
        let local_old_start = op.old_range().start;
        let local_old_end = op.old_range().end;
        let local_new_start = op.new_range().start;
        let local_new_end = op.new_range().end;
        edits.push(ByteEdit {
            start: old_start + local_old_start,
            end: old_start + local_old_end,
            replacement: new[local_new_start..local_new_end].to_vec(),
        });
    }
    Ok(())
}

#[allow(dead_code)]
fn _engine_error_is_exhaustive(error: EngineError) -> DiffPlannerError {
    DiffPlannerError::Invariant(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_op_has_no_regions() {
        let plan = plan(b"same\r\n", b"same\r\n").unwrap();
        assert!(plan.edits.is_empty());
        assert_eq!(plan.derived_region_count, 0);
    }

    #[test]
    fn long_line_distant_changes_stay_narrow_and_separate() {
        let observed = format!(
            "{}LEFT{}MIDDLE{}RIGHT",
            "a".repeat(400),
            "b".repeat(400),
            "c".repeat(400)
        );
        let desired = observed.replace("LEFT", "LFT").replace("RIGHT", "RITE");
        let plan = plan(observed.as_bytes(), desired.as_bytes()).unwrap();
        assert!(plan.derived_region_count >= 2);
        assert!(plan.edits.iter().all(|edit| edit.end - edit.start <= 5));
    }

    #[test]
    fn repeated_bytes_are_deterministic() {
        let observed = b"aaXXaaYYaa";
        let desired = b"aa11aa22aa";
        assert_eq!(plan(observed, desired), plan(observed, desired));
        assert_eq!(plan(observed, desired).unwrap().derived_region_count, 2);
    }

    #[test]
    fn empty_and_crlf_states_are_exact() {
        let inserted = plan(b"", b"a\r\nb\r\n").unwrap();
        assert_eq!(
            apply_byte_edits(b"", &inserted.edits).unwrap(),
            b"a\r\nb\r\n"
        );
        let removed = plan(b"a\r\nb\r\n", b"").unwrap();
        assert_eq!(
            apply_byte_edits(b"a\r\nb\r\n", &removed.edits).unwrap(),
            b""
        );
    }
}
