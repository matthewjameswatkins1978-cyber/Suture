#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EngineError {
    #[error("Edit out of bounds: range {start}..{end} exceeds slice length {len}")]
    OutOfBounds {
        start: usize,
        end: usize,
        len: usize,
    },

    #[error("Overlapping or unsorted edits at index {index}: range {start1}..{end1} and range {start2}..{end2}")]
    OverlappingOrUnsorted {
        index: usize,
        start1: usize,
        end1: usize,
        start2: usize,
        end2: usize,
    },

    #[error("Edited output size exceeds addressable memory")]
    OutputSizeOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ByteEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: Vec<u8>,
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    format!("{:x}", result)
}

pub fn apply_byte_edits(original: &[u8], edits: &[ByteEdit]) -> Result<Vec<u8>, EngineError> {
    for edit in edits {
        if edit.start > edit.end || edit.end > original.len() {
            return Err(EngineError::OutOfBounds {
                start: edit.start,
                end: edit.end,
                len: original.len(),
            });
        }
    }

    for (index, pair) in edits.windows(2).enumerate() {
        let previous = &pair[0];
        let edit = &pair[1];
        if previous.end > edit.start {
            return Err(EngineError::OverlappingOrUnsorted {
                index: index + 1,
                start1: previous.start,
                end1: previous.end,
                start2: edit.start,
                end2: edit.end,
            });
        }
    }

    let final_len = edits.iter().try_fold(original.len(), |len, edit| {
        len.checked_sub(edit.end - edit.start)
            .and_then(|len| len.checked_add(edit.replacement.len()))
            .ok_or(EngineError::OutputSizeOverflow)
    })?;

    let mut result = Vec::with_capacity(final_len);
    let mut last_idx = 0;

    for edit in edits {
        result.extend_from_slice(&original[last_idx..edit.start]);
        result.extend_from_slice(&edit.replacement);
        last_idx = edit.end;
    }

    result.extend_from_slice(&original[last_idx..]);
    debug_assert_eq!(result.len(), final_len);

    Ok(result)
}

pub fn generate_diff(original: &[u8], modified: &[u8]) -> String {
    let orig_str = String::from_utf8_lossy(original);
    let mod_str = String::from_utf8_lossy(modified);

    let diff = similar::TextDiff::from_lines(&orig_str, &mod_str);
    diff.unified_diff().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_apply_byte_edits_basic() {
        let original = b"Hello, World!";
        let edits = vec![
            ByteEdit {
                start: 0,
                end: 5,
                replacement: b"Greetings".to_vec(),
            },
            ByteEdit {
                start: 7,
                end: 12,
                replacement: b"Universe".to_vec(),
            },
        ];
        let res = apply_byte_edits(original, &edits).unwrap();
        assert_eq!(res, b"Greetings, Universe!");
    }

    #[test]
    fn test_apply_byte_edits_out_of_bounds() {
        let original = b"test";
        let edits = vec![ByteEdit {
            start: 0,
            end: 5,
            replacement: b"abc".to_vec(),
        }];
        let res = apply_byte_edits(original, &edits);
        assert_eq!(
            res,
            Err(EngineError::OutOfBounds {
                start: 0,
                end: 5,
                len: 4
            })
        );
    }

    #[test]
    fn test_apply_byte_edits_overlapping() {
        let original = b"abcdef";
        let edits = vec![
            ByteEdit {
                start: 1,
                end: 4,
                replacement: b"X".to_vec(),
            },
            ByteEdit {
                start: 3,
                end: 5,
                replacement: b"Y".to_vec(),
            },
        ];
        let res = apply_byte_edits(original, &edits);
        assert!(matches!(
            res,
            Err(EngineError::OverlappingOrUnsorted { .. })
        ));
    }

    #[test]
    fn test_apply_byte_edits_unsorted() {
        let original = b"abcdef";
        let edits = vec![
            ByteEdit {
                start: 4,
                end: 5,
                replacement: b"X".to_vec(),
            },
            ByteEdit {
                start: 1,
                end: 2,
                replacement: b"Y".to_vec(),
            },
        ];
        let res = apply_byte_edits(original, &edits);
        assert!(matches!(
            res,
            Err(EngineError::OverlappingOrUnsorted { .. })
        ));
    }

    #[test]
    fn apply_byte_edits_uses_exact_final_size() {
        let original = b"abcdef";
        let edits = [ByteEdit {
            start: 1,
            end: 5,
            replacement: b"123456".to_vec(),
        }];
        let result = apply_byte_edits(original, &edits).unwrap();
        assert_eq!(result, b"a123456f");
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_generate_diff() {
        let orig = b"line 1\nline 2\n";
        let mod_bytes = b"line 1\nline two\n";
        let diff = generate_diff(orig, mod_bytes);
        assert!(diff.contains("-line 2"));
        assert!(diff.contains("+line two"));
    }
}
