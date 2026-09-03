#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::Cardinality;
use crate::provider::json::{JsonOperation, JsonProvider, JsonProviderError};

/// JSONC uses the strict JSON source-range planner after comments are masked
/// with equal-length spaces. Offsets therefore remain valid and comments are
/// never part of a replacement range.
pub struct JsoncProvider;

impl JsoncProvider {
    pub fn plan(
        content: &[u8],
        op: &JsonOperation,
        cardinality: &Cardinality,
    ) -> Result<Vec<ByteEdit>, JsonProviderError> {
        let masked = mask_comments(content)?;
        JsonProvider::plan(&masked, op, cardinality)
    }

    pub fn validate(content: &[u8]) -> Result<(), JsonProviderError> {
        let masked = mask_comments(content)?;
        JsonProvider::validate(&masked)
    }
}

fn mask_comments(content: &[u8]) -> Result<Vec<u8>, JsonProviderError> {
    let mut out = content.to_vec();
    let mut i = 0;
    let mut string = false;
    let mut escaped = false;
    while i < content.len() {
        let b = content[i];
        if string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                string = false;
            }
            i += 1;
            continue;
        }
        if b == b'"' {
            string = true;
            i += 1;
            continue;
        }
        if b == b'/' && content.get(i + 1) == Some(&b'/') {
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            while i < content.len() && content[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        }
        if b == b'/' && content.get(i + 1) == Some(&b'*') {
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            let mut closed = false;
            while i + 1 < content.len() {
                if content[i] == b'*' && content[i + 1] == b'/' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    closed = true;
                    break;
                }
                if content[i] != b'\r' && content[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            if !closed {
                return Err(JsonProviderError::Error {
                    message: "unterminated JSONC block comment".into(),
                });
            }
            continue;
        }
        i += 1;
    }
    // JSONC permits trailing commas. Mask only commas whose next significant
    // byte closes an object/array; offsets remain unchanged.
    let mut i = 0;
    let mut string = false;
    let mut escaped = false;
    while i < out.len() {
        if string {
            if escaped {
                escaped = false;
            } else if out[i] == b'\\' {
                escaped = true;
            } else if out[i] == b'"' {
                string = false;
            }
            i += 1;
            continue;
        }
        if out[i] == b'"' {
            string = true;
            i += 1;
            continue;
        }
        if out[i] == b',' {
            let mut next = i + 1;
            while next < out.len() && out[next].is_ascii_whitespace() {
                next += 1;
            }
            if matches!(out.get(next), Some(b'}') | Some(b']')) {
                out[i] = b' ';
            }
        }
        i += 1;
    }
    Ok(out)
}
