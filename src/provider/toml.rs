#![forbid(unsafe_code)]

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Value};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TomlOperation {
    Set {
        path: String,
        value: TomlValueWrapper,
    },
    Insert {
        path: String,
        key: String,
        value: TomlValueWrapper,
    },
    Delete {
        path: String,
    },
    RenameKey {
        path: String,
        new_key: String,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TomlValueWrapper {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<TomlValueWrapper>),
    Table(std::collections::BTreeMap<String, TomlValueWrapper>),
}

impl TomlValueWrapper {
    fn to_toml_value(&self) -> Value {
        match self {
            TomlValueWrapper::String(s) => Value::from(s.as_str()),
            TomlValueWrapper::Integer(i) => Value::from(*i),
            TomlValueWrapper::Float(f) => Value::from(*f),
            TomlValueWrapper::Boolean(b) => Value::from(*b),
            TomlValueWrapper::Array(arr) => {
                let mut t_arr = toml_edit::Array::new();
                for item in arr {
                    t_arr.push(item.to_toml_value());
                }
                Value::Array(t_arr)
            }
            TomlValueWrapper::Table(tbl) => {
                let mut t_tbl = toml_edit::InlineTable::new();
                for (k, v) in tbl {
                    t_tbl.insert(k.as_str(), v.to_toml_value());
                }
                Value::InlineTable(t_tbl)
            }
        }
    }

    fn to_toml_item(&self) -> Item {
        match self {
            TomlValueWrapper::Table(tbl) => {
                let mut table = toml_edit::Table::new();
                for (k, v) in tbl {
                    table.insert(k.as_str(), v.to_toml_item());
                }
                Item::Table(table)
            }
            _ => Item::Value(self.to_toml_value()),
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TomlProviderError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
    #[error("Error: {message}")]
    Error { message: String },
}

pub struct TomlProvider;

impl TomlProvider {
    pub fn plan(
        content: &[u8],
        op: &TomlOperation,
        cardinality: &Cardinality,
    ) -> Result<Vec<ByteEdit>, TomlProviderError> {
        if !matches!(cardinality, Cardinality::ExactlyOne) {
            return Err(TomlProviderError::Refused(
                RefusalReason::CardinalityMismatch {
                    expected: "exactly_one (structured paths are unique)".into(),
                    actual: 1,
                },
            ));
        }
        let content_str = std::str::from_utf8(content).map_err(|e| {
            TomlProviderError::Refused(RefusalReason::MalformedInput {
                details: format!("Invalid UTF-8 in TOML file: {}", e),
            })
        })?;

        let mut doc = content_str.parse::<DocumentMut>().map_err(|e| {
            TomlProviderError::Refused(RefusalReason::MalformedInput {
                details: format!("Malformed TOML syntax: {}", e),
            })
        })?;

        apply_toml_operation(&mut doc, op)?;

        let serialized_modified = doc.to_string();
        let has_trailing_newline = content.ends_with(b"\n");
        let mut final_bytes = serialized_modified.into_bytes();
        if has_trailing_newline && !final_bytes.ends_with(b"\n") {
            final_bytes.push(b'\n');
        } else if !has_trailing_newline && final_bytes.ends_with(b"\n") && !content.ends_with(b"\n")
        {
            // preserve trailing newline state of original content
            // if original didn't have trailing newline, trim trailing newlines from final_bytes if appropriate,
            // or keep toml_edit's standard formatting. But wait, FR-05 says "preserve existing trailing newline state".
            while final_bytes.ends_with(b"\n") && !content.ends_with(b"\n") {
                final_bytes.pop();
            }
        }

        if final_bytes == content {
            return Ok(Vec::new());
        }
        if has_changed_encoding_or_newline(content, &final_bytes) {
            return Err(TomlProviderError::Refused(
                RefusalReason::PreservationUnavailable {
                    details: "toml_edit changed BOM, line endings, or final-newline state".into(),
                },
            ));
        }
        let (start, end, replacement) = narrow_diff(content, &final_bytes);
        Ok(vec![ByteEdit {
            start,
            end,
            replacement,
        }])
    }
}

fn has_changed_encoding_or_newline(original: &[u8], modified: &[u8]) -> bool {
    original.starts_with(&[0xef, 0xbb, 0xbf]) != modified.starts_with(&[0xef, 0xbb, 0xbf])
        || original.ends_with(b"\n") != modified.ends_with(b"\n")
        || (original.contains(&b'\r') != modified.contains(&b'\r'))
}

fn narrow_diff(original: &[u8], modified: &[u8]) -> (usize, usize, Vec<u8>) {
    let prefix = original
        .iter()
        .zip(modified)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = original[prefix..]
        .iter()
        .rev()
        .zip(modified[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let end = original.len().saturating_sub(suffix);
    let modified_end = modified.len().saturating_sub(suffix);
    (prefix, end, modified[prefix..modified_end].to_vec())
}

fn apply_toml_operation(
    doc: &mut DocumentMut,
    op: &TomlOperation,
) -> Result<(), TomlProviderError> {
    match op {
        TomlOperation::Set { path, value } => {
            let segments = parse_toml_path(path)?;
            set_toml_path(doc.as_table_mut(), &segments, value)?;
        }
        TomlOperation::Insert { path, key, value } => {
            let segments = parse_toml_path(path)?;
            insert_toml_path(doc.as_table_mut(), &segments, key, value)?;
        }
        TomlOperation::Delete { path } => {
            let segments = parse_toml_path(path)?;
            delete_toml_path(doc.as_table_mut(), &segments)?;
        }
        TomlOperation::RenameKey { path, new_key } => {
            let segments = parse_toml_path(path)?;
            rename_toml_path(doc.as_table_mut(), &segments, new_key)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TomlPathSegment {
    Key(String),
    Index(usize),
}

fn parse_toml_path(path: &str) -> Result<Vec<TomlPathSegment>, TomlProviderError> {
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
    let parts: Vec<&str> = path_str.split('.').collect();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // Handle array indexing like items[0] or [0]
        if part.contains('[') && part.ends_with(']') {
            let idx_start = part.find('[').unwrap();
            let key_part = &part[..idx_start];
            if !key_part.is_empty() {
                segments.push(TomlPathSegment::Key(key_part.to_string()));
            }
            let idx_str = &part[idx_start + 1..part.len() - 1];
            if let Ok(idx) = idx_str.trim().parse::<usize>() {
                segments.push(TomlPathSegment::Index(idx));
            } else {
                return Err(TomlProviderError::Refused(RefusalReason::MalformedInput {
                    details: format!("Invalid array index in TOML path: {}", path),
                }));
            }
        } else {
            segments.push(TomlPathSegment::Key(part.to_string()));
        }
    }

    Ok(segments)
}

fn set_toml_path(
    table: &mut toml_edit::Table,
    segments: &[TomlPathSegment],
    value: &TomlValueWrapper,
) -> Result<(), TomlProviderError> {
    if segments.is_empty() {
        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
            target: "Cannot set root table directly via empty path".to_string(),
        }));
    }

    let (head, tail) = segments.split_first().unwrap();

    match head {
        TomlPathSegment::Key(key) => {
            if tail.is_empty() {
                if !table.contains_key(key) {
                    return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for setting", key),
                    }));
                }
                table.insert(key.as_str(), value.to_toml_item());
            } else {
                let item = table.get_mut(key).ok_or_else(|| {
                    TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                match item {
                    Item::Table(sub_table) => {
                        set_toml_path(sub_table, tail, value)?;
                    }
                    Item::Value(Value::InlineTable(inline_table)) => {
                        // convert inline table access or navigate
                        // For simplicity, or if tail points into inline table
                        set_toml_inline_path(inline_table, tail, value)?;
                    }
                    _ => {
                        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                            target: format!("Target at key '{}' is not a table", key),
                        }));
                    }
                }
            }
        }
        TomlPathSegment::Index(idx) => {
            return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                target: format!("Cannot set array index {} directly on table", idx),
            }));
        }
    }
    Ok(())
}

fn set_toml_inline_path(
    inline_table: &mut toml_edit::InlineTable,
    segments: &[TomlPathSegment],
    value: &TomlValueWrapper,
) -> Result<(), TomlProviderError> {
    if segments.is_empty() {
        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
            target: "Empty path segment in inline table".to_string(),
        }));
    }
    let (head, tail) = segments.split_first().unwrap();
    match head {
        TomlPathSegment::Key(key) => {
            if tail.is_empty() {
                if !inline_table.contains_key(key) {
                    return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found in inline table for setting", key),
                    }));
                }
                inline_table.insert(key.as_str(), value.to_toml_value());
            } else {
                let val = inline_table.get_mut(key).ok_or_else(|| {
                    TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found in inline table", key),
                    })
                })?;
                if let Value::InlineTable(sub_inline) = val {
                    set_toml_inline_path(sub_inline, tail, value)?;
                } else {
                    return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Target at key '{}' is not an inline table", key),
                    }));
                }
            }
        }
        TomlPathSegment::Index(idx) => {
            return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                target: format!("Cannot index inline table at index {}", idx),
            }));
        }
    }
    Ok(())
}

fn insert_toml_path(
    table: &mut toml_edit::Table,
    segments: &[TomlPathSegment],
    key: &str,
    value: &TomlValueWrapper,
) -> Result<(), TomlProviderError> {
    if segments.is_empty() {
        table.insert(key, value.to_toml_item());
        return Ok(());
    }

    let (head, tail) = segments.split_first().unwrap();
    match head {
        TomlPathSegment::Key(k) => {
            let item = table.get_mut(k).ok_or_else(|| {
                TomlProviderError::Refused(RefusalReason::MissingTarget {
                    target: format!("Key '{}' not found for insertion", k),
                })
            })?;
            match item {
                Item::Table(sub_table) => {
                    if tail.is_empty() {
                        sub_table.insert(key, value.to_toml_item());
                    } else {
                        insert_toml_path(sub_table, tail, key, value)?;
                    }
                }
                Item::Value(Value::Array(arr)) => {
                    if tail.is_empty() {
                        arr.push(value.to_toml_value());
                    } else {
                        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                            target: "Cannot insert further into array element via tail".to_string(),
                        }));
                    }
                }
                _ => {
                    return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Target at key '{}' is neither table nor array", k),
                    }));
                }
            }
        }
        TomlPathSegment::Index(idx) => {
            return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                target: format!("Cannot insert into table at index {}", idx),
            }));
        }
    }
    Ok(())
}

fn delete_toml_path(
    table: &mut toml_edit::Table,
    segments: &[TomlPathSegment],
) -> Result<(), TomlProviderError> {
    if segments.is_empty() {
        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
            target: "Cannot delete root table".to_string(),
        }));
    }

    let (head, tail) = segments.split_first().unwrap();
    match head {
        TomlPathSegment::Key(key) => {
            if tail.is_empty() {
                if table.remove(key).is_none() {
                    return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for deletion", key),
                    }));
                }
            } else {
                let item = table.get_mut(key).ok_or_else(|| {
                    TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                match item {
                    Item::Table(sub_table) => {
                        delete_toml_path(sub_table, tail)?;
                    }
                    _ => {
                        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                            target: format!("Target at key '{}' is not a table", key),
                        }));
                    }
                }
            }
        }
        TomlPathSegment::Index(idx) => {
            return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                target: format!("Cannot delete table entry via index {}", idx),
            }));
        }
    }
    Ok(())
}

fn rename_toml_path(
    table: &mut toml_edit::Table,
    segments: &[TomlPathSegment],
    new_key: &str,
) -> Result<(), TomlProviderError> {
    if segments.is_empty() {
        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
            target: "Cannot rename root table".to_string(),
        }));
    }

    let (head, tail) = segments.split_first().unwrap();
    match head {
        TomlPathSegment::Key(key) => {
            if tail.is_empty() {
                let item = table.remove(key).ok_or_else(|| {
                    TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found for renaming", key),
                    })
                })?;
                table.insert(new_key, item);
            } else {
                let item = table.get_mut(key).ok_or_else(|| {
                    TomlProviderError::Refused(RefusalReason::MissingTarget {
                        target: format!("Key '{}' not found", key),
                    })
                })?;
                match item {
                    Item::Table(sub_table) => {
                        rename_toml_path(sub_table, tail, new_key)?;
                    }
                    _ => {
                        return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                            target: format!("Target at key '{}' is not a table", key),
                        }));
                    }
                }
            }
        }
        TomlPathSegment::Index(idx) => {
            return Err(TomlProviderError::Refused(RefusalReason::MissingTarget {
                target: format!("Cannot rename table entry via index {}", idx),
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply_byte_edits;

    #[test]
    fn test_toml_set_operation_with_comments() {
        let content = br#"
# Project configuration
[package]
name = "suture" # package name
version = "0.1.0"
"#;
        let op = TomlOperation::Set {
            path: "package.version".to_string(),
            value: TomlValueWrapper::String("0.2.0".to_string()),
        };
        let edits = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let mod_str = std::str::from_utf8(&modified).unwrap();

        // Verify version changed and comments/formatting preserved
        assert!(mod_str.contains("version = \"0.2.0\""));
        assert!(mod_str.contains("# Project configuration"));
        assert!(mod_str.contains("# package name"));
    }

    #[test]
    fn test_toml_insert_operation() {
        let content = br#"
[package]
name = "suture"
"#;
        let op = TomlOperation::Insert {
            path: "package".to_string(),
            key: "version".to_string(),
            value: TomlValueWrapper::String("0.1.0".to_string()),
        };
        let edits = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let mod_str = std::str::from_utf8(&modified).unwrap();
        assert!(mod_str.contains("version = \"0.1.0\""));
        assert!(mod_str.contains("name = \"suture\""));
    }

    #[test]
    fn test_toml_delete_operation() {
        let content = br#"
[package]
name = "suture"
remove_me = "bye"
"#;
        let op = TomlOperation::Delete {
            path: "package.remove_me".to_string(),
        };
        let edits = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let mod_str = std::str::from_utf8(&modified).unwrap();
        assert!(!mod_str.contains("remove_me"));
        assert!(mod_str.contains("name = \"suture\""));
    }

    #[test]
    fn test_toml_rename_key_operation() {
        let content = br#"
[package]
old_name = "suture"
"#;
        let op = TomlOperation::RenameKey {
            path: "package.old_name".to_string(),
            new_key: "name".to_string(),
        };
        let edits = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne).unwrap();
        let modified = apply_byte_edits(content, &edits).unwrap();
        let mod_str = std::str::from_utf8(&modified).unwrap();
        assert!(mod_str.contains("name = \"suture\""));
        assert!(!mod_str.contains("old_name"));
    }

    #[test]
    fn test_toml_malformed_rejection() {
        let content = b"unclosed = \"string\n[package\n";
        let op = TomlOperation::Set {
            path: "package.name".to_string(),
            value: TomlValueWrapper::String("test".to_string()),
        };
        let res = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            res,
            Err(TomlProviderError::Refused(
                RefusalReason::MalformedInput { .. }
            ))
        ));
    }

    #[test]
    fn test_toml_missing_target_refusal() {
        let content = br#"
[package]
name = "suture"
"#;
        let op = TomlOperation::Set {
            path: "package.nonexistent".to_string(),
            value: TomlValueWrapper::String("test".to_string()),
        };
        let res = TomlProvider::plan(content, &op, &Cardinality::ExactlyOne);
        assert!(matches!(
            res,
            Err(TomlProviderError::Refused(
                RefusalReason::MissingTarget { .. }
            ))
        ));
    }
}
