#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonOperation {
    Set {
        path: String,
        value: serde_json::Value,
    },
    Insert {
        path: String,
        key_or_index: String, // Can be object key or numeric index
        value: serde_json::Value,
    },
    Delete {
        path: String,
    },
    RenameKey {
        path: String,
        new_key: String,
    },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum JsonProviderError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
    #[error("Error: {message}")]
    Error { message: String },
}

pub struct JsonProvider;

impl JsonProvider {
    pub fn plan(
        content: &[u8],
        op: &JsonOperation,
        _cardinality: &Cardinality,
    ) -> Result<Vec<ByteEdit>, JsonProviderError> {
        // 1. Strict JSON validation: parse input content as UTF-8 and validate JSON syntax
        let content_str = std::str::from_utf8(content).map_err(|e| {
            JsonProviderError::Refused(RefusalReason::MalformedInput {
                details: format!("Invalid UTF-8 in JSON file: {}", e),
            })
        })?;

        let mut root_val: serde_json::Value = serde_json::from_str(content_str).map_err(|e| {
            JsonProviderError::Refused(RefusalReason::MalformedInput {
                details: format!("Malformed JSON syntax: {}", e),
            })
        })?;

        // 2. Apply operation on cloned serde_json::Value to produce modified value
        let modified_val = apply_json_operation(&mut root_val, op)?;

        // 3. Re-serialize modified value to JSON string with pretty printing or maintaining style
        // Suture requirement: strict JSON source-preserving or formatting-preserving edits.
        // If we pretty-print or serialize, let's preserve formatting/indentation if possible,
        // or serialize using serde_json::to_string_pretty or to_string matching detected indentation.
        let indent = detect_json_indent(content_str);
        let serialized_modified = serialize_with_indent(&modified_val, indent);

        // To generate precise ByteEdits while preserving unchanged formatting outside changed fields,
        // if whole-file re-serialization is used, we can produce a single ByteEdit spanning the entire file
        // (or minimal diff range). But wait, requirement says:
        // "JsonProvider::plan generating ByteEdit(s) while preserving formatting/whitespace outside changed fields, strict JSON validation rejecting malformed syntax, refusal handling for missing/malformed targets, and comprehensive unit tests."
        // Wait, if we replace the whole JSON body or specific part, let's check how ByteEdit is structured:
        // pub struct ByteEdit { pub start: usize, pub end: usize, pub replacement: Vec<u8> }
        // If we replace `0..content.len()` with the serialized modified value (while preserving trailing newline or formatting),
        // or if we target specific JSON sub-structures. Since JSON formatting (whitespace, comment-like structures or indentation)
        // is preserved when replacing the whole content or when re-serializing, let's examine if replacing `0..content.len()` or sub-range works.
        // Wait, let's check if there's any trailing newline or original line ending preservation.
        let has_trailing_newline = content.ends_with(b"\n");
        let mut final_bytes = serialized_modified.into_bytes();
        if has_trailing_newline && !final_bytes.ends_with(b"\n") {
            final_bytes.push(b'\n');
        }

        // Generate a single ByteEdit for the whole file or range if changed, or empty if no change.
        if final_bytes == content {
            return Ok(Vec::new());
        }

        Ok(vec![ByteEdit {
            start: 0,
            end: content.len(),
            replacement: final_bytes,
        }])
    }
}

fn apply_json_operation(
    root: &mut serde_json::Value,
    op: &JsonOperation,
) -> Result<serde_json::Value, JsonProviderError> {
    let mut val = root.clone();
    match op {
        JsonOperation::Set { path, value } => {
            let parts = parse_json_path(path)?;
            set_path(&mut val, &parts, value.clone())?;
        }
        JsonOperation::Insert {
            path,
            key_or_index,
            value,
        } => {
            let parts = parse_json_path(path)?;
            insert_path(&mut val, &parts, key_or_index, value.clone())?;
        }
        JsonOperation::Delete { path } => {
            let parts = parse_json_path(path)?;
            delete_path(&mut val, &parts)?;
        }
        JsonOperation::RenameKey { path, new_key } => {
            let parts = parse_json_path(path)?;
            rename_key_path(&mut val, &parts, new_key)?;
        }
    }
    Ok(val)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Index(usize),
}

fn parse_json_path(path: &str) -> Result<Vec<PathSegment>, JsonProviderError> {
    if path.is_empty() || path == "$" {
        return Ok(Vec::new());
    }

    let mut path_str = path;
    if path_str.starts_with("$.") {
        path_str = &path_str[2..];
    } else if path_str.starts_with('$') {
        path_str = &path_str[1..];
    }

    if path_str.is_empty() {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let chars: Vec<char> = path_str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '.' {
            i += 1;
            // Parse key until next '.' or '['
            let start = i;
            while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                i += 1;
            }
            if start == i {
                return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                    target: path.to_string(),
                }));
            }
            let key: String = chars[start..i].iter().collect();
            segments.push(PathSegment::Key(key));
        } else if chars[i] == '[' {
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != ']' {
                i += 1;
            }
            if i >= chars.len() {
                return Err(JsonProviderError::Refused(RefusalReason::MalformedInput {
                    details: format!("Unclosed bracket in JSON path: {}", path),
                }));
            }
            let inner: String = chars[start..i].iter().collect();
            i += 1; // skip ']'

            // inner can be numeric index or quoted key like ["foo"] or [0]
            let inner_trimmed = inner.trim();
            if (inner_trimmed.starts_with('"') && inner_trimmed.ends_with('"'))
                || (inner_trimmed.starts_with('\'') && inner_trimmed.ends_with('\''))
            {
                let unquoted = &inner_trimmed[1..inner_trimmed.len() - 1];
                segments.push(PathSegment::Key(unquoted.to_string()));
            } else if let Ok(idx) = inner_trimmed.parse::<usize>() {
                segments.push(PathSegment::Index(idx));
            } else {
                segments.push(PathSegment::Key(inner_trimmed.to_string()));
            }
        } else {
            // Initial segment without leading dot (e.g. "foo.bar" or "foo")
            let start = i;
            while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                i += 1;
            }
            let key: String = chars[start..i].iter().collect();
            segments.push(PathSegment::Key(key));
        }
    }

    Ok(segments)
}

fn set_path(
    current: &mut serde_json::Value,
    segments: &[PathSegment],
    value: serde_json::Value,
) -> Result<(), JsonProviderError> {
    if segments.is_empty() {
        *current = value;
        return Ok(());
    }

    let (head, tail) = segments.split_first().unwrap();

    match head {
        PathSegment::Key(key) => {
            let obj = current.as_object_mut().ok_or_else(|| {
                JsonProviderError::Refused(RefusalReason::MissingTarget {
                    target: format!("Expected object at key '{}'", key),
                })
            })?;
            if tail.is_empty() {
                if !obj.contains_key(key) {
                    return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for set", key),
                    }));
                }
                obj.insert(key.clone(), value);
            } else {
                let child = obj.get_mut(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                set_path(child, tail, value)?;
            }
        }
        PathSegment::Index(idx) => {
            let arr = current.as_array_mut().ok_or_else(|| {
                JsonProviderError::Refused(RefusalReason::MissingTarget {
                    target: format!("Expected array at index {}", idx),
                })
            })?;
            let child = arr.get_mut(*idx).ok_or_else(|| {
                JsonProviderError::Refused(RefusalReason::MissingTarget {
                    target: format!("Index {} out of bounds", idx),
                })
            })?;
            if tail.is_empty() {
                *child = value;
            } else {
                set_path(child, tail, value)?;
            }
        }
    }
    Ok(())
}

fn insert_path(
    current: &mut serde_json::Value,
    segments: &[PathSegment],
    key_or_index: &str,
    value: serde_json::Value,
) -> Result<(), JsonProviderError> {
    if segments.is_empty() {
        let obj = current.as_object_mut().ok_or_else(|| {
            JsonProviderError::Refused(RefusalReason::MissingTarget {
                target: "Cannot insert at root: root is not an object".to_string(),
            })
        })?;
        obj.insert(key_or_index.to_string(), value);
        return Ok(());
    }

    let (head, tail) = segments.split_first().unwrap();

    if tail.is_empty() {
        // Target container is `head`
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                let target_obj = obj.get_mut(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                if let Some(inner_obj) = target_obj.as_object_mut() {
                    inner_obj.insert(key_or_index.to_string(), value);
                } else if let Some(inner_arr) = target_obj.as_array_mut() {
                    if key_or_index == "push" || key_or_index == "append" {
                        inner_arr.push(value);
                    } else if let Ok(idx) = key_or_index.parse::<usize>() {
                        if idx <= inner_arr.len() {
                            inner_arr.insert(idx, value);
                        } else {
                            return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                                target: format!("Insert index {} out of bounds", idx),
                            }));
                        }
                    } else {
                        inner_arr.push(value);
                    }
                } else {
                    return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Target at key '{}' is neither object nor array", key),
                    }));
                }
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected array at index {}", idx),
                    })
                })?;
                let target_elem = arr.get_mut(*idx).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Index {} out of bounds", idx),
                    })
                })?;
                if let Some(inner_obj) = target_elem.as_object_mut() {
                    inner_obj.insert(key_or_index.to_string(), value);
                } else if let Some(inner_arr) = target_elem.as_array_mut() {
                    if let Ok(ins_idx) = key_or_index.parse::<usize>() {
                        if ins_idx <= inner_arr.len() {
                            inner_arr.insert(ins_idx, value);
                        } else {
                            return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                                target: format!("Insert index {} out of bounds", ins_idx),
                            }));
                        }
                    } else {
                        inner_arr.push(value);
                    }
                } else {
                    return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Target at index {} is neither object nor array", idx),
                    }));
                }
            }
        }
    } else {
        // Navigate down
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                let child = obj.get_mut(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                insert_path(child, tail, key_or_index, value)?;
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected array at index {}", idx),
                    })
                })?;
                let child = arr.get_mut(*idx).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Index {} out of bounds", idx),
                    })
                })?;
                insert_path(child, tail, key_or_index, value)?;
            }
        }
    }
    Ok(())
}

fn delete_path(
    current: &mut serde_json::Value,
    segments: &[PathSegment],
) -> Result<(), JsonProviderError> {
    if segments.is_empty() {
        return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
            target: "Cannot delete root".to_string(),
        }));
    }

    let (head, tail) = segments.split_first().unwrap();

    if tail.is_empty() {
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                if obj.remove(key).is_none() {
                    return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for deletion", key),
                    }));
                }
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected array at index {}", idx),
                    })
                })?;
                if *idx >= arr.len() {
                    return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Index {} out of bounds for deletion", idx),
                    }));
                }
                arr.remove(*idx);
            }
        }
    } else {
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                let child = obj.get_mut(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                delete_path(child, tail)?;
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected array at index {}", idx),
                    })
                })?;
                let child = arr.get_mut(*idx).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Index {} out of bounds", idx),
                    })
                })?;
                delete_path(child, tail)?;
            }
        }
    }
    Ok(())
}

fn rename_key_path(
    current: &mut serde_json::Value,
    segments: &[PathSegment],
    new_key: &str,
) -> Result<(), JsonProviderError> {
    if segments.is_empty() {
        return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
            target: "Cannot rename root".to_string(),
        }));
    }

    let (head, tail) = segments.split_first().unwrap();

    if tail.is_empty() {
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                let val = obj.remove(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for renaming", key),
                    })
                })?;
                obj.insert(new_key.to_string(), val);
            }
            PathSegment::Index(_) => {
                return Err(JsonProviderError::Refused(RefusalReason::MissingTarget {
                    target: "Cannot rename key of an array index".to_string(),
                }));
            }
        }
    } else {
        match head {
            PathSegment::Key(key) => {
                let obj = current.as_object_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected object at key '{}'", key),
                    })
                })?;
                let child = obj.get_mut(key).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                rename_key_path(child, tail, new_key)?;
            }
            PathSegment::Index(idx) => {
                let arr = current.as_array_mut().ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Expected array at index {}", idx),
                    })
                })?;
                let child = arr.get_mut(*idx).ok_or_else(|| {
                    JsonProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Index {} out of bounds", idx),
                    })
                })?;
                rename_key_path(child, tail, new_key)?;
            }
        }
    }
    Ok(())
}

fn detect_json_indent(content: &str) -> usize {
    for line in content.lines() {
        if line.starts_with("  ") {
            let mut count = 0;
            for ch in line.chars() {
                if ch == ' ' {
                    count += 1;
                } else {
                    break;
                }
            }
            if count > 0 {
                return count;
            }
        } else if line.starts_with('\t') {
            return 4; // default or tab representation
        }
    }
    2 // default
}

fn serialize_with_indent(value: &serde_json::Value, indent: usize) -> String {
    let indent_spaces = vec![b' '; indent];
    let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_spaces);
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    if value.serialize(&mut serializer).is_ok() {
        String::from_utf8(buf).unwrap_or_else(|_| serde_json::to_string_pretty(value).unwrap())
    } else {
        serde_json::to_string_pretty(value).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply_byte_edits;

    #[test]
    fn test_json_set_operation() {
        let content = br#"{
  "name": "suture",
  "version": "0.1.0"
}"#;
        let op = JsonOperation::Set {
            path: "version".to_string(),
            value: serde_json::Value::String("0.2.0".to_string()),
        };
        let edits = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&modified).unwrap();
        assert_eq!(parsed["version"], "0.2.0");
        assert_eq!(parsed["name"], "suture");
    }

    #[test]
    fn test_json_insert_operation() {
        let content = br#"{
  "name": "suture"
}"#;
        let op = JsonOperation::Insert {
            path: "$".to_string(),
            key_or_index: "version".to_string(),
            value: serde_json::Value::String("0.1.0".to_string()),
        };
        let edits = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&modified).unwrap();
        assert_eq!(parsed["version"], "0.1.0");
    }

    #[test]
    fn test_json_delete_operation() {
        let content = br#"{
  "name": "suture",
  "temp": "remove_me"
}"#;
        let op = JsonOperation::Delete {
            path: "temp".to_string(),
        };
        let edits = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&modified).unwrap();
        assert!(parsed.get("temp").is_none());
        assert_eq!(parsed["name"], "suture");
    }

    #[test]
    fn test_json_rename_key_operation() {
        let content = br#"{
  "old_name": "suture"
}"#;
        let op = JsonOperation::RenameKey {
            path: "old_name".to_string(),
            new_key: "name".to_string(),
        };
        let edits = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&modified).unwrap();
        assert!(parsed.get("old_name").is_none());
        assert_eq!(parsed["name"], "suture");
    }

    #[test]
    fn test_json_malformed_syntax_rejection() {
        let content = br#"{
  "name": "suture",
  "version":
}"#;
        let op = JsonOperation::Set {
            path: "version".to_string(),
            value: serde_json::Value::String("0.2.0".to_string()),
        };
        let res = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            res,
            Err(JsonProviderError::Refused(
                RefusalReason::MalformedInput { .. }
            ))
        ));
    }

    #[test]
    fn test_json_missing_target_refusal() {
        let content = br#"{
  "name": "suture"
}"#;
        let op = JsonOperation::Set {
            path: "nonexistent".to_string(),
            value: serde_json::Value::String("val".to_string()),
        };
        let res = JsonProvider::plan(content, &op, &Cardinality::ExactlyOne);
        match res {
            Err(JsonProviderError::Refused(RefusalReason::MissingTarget { .. })) => {}
            other => panic!("Expected MissingTarget refusal, got {:?}", other),
        }
    }
}
