#![forbid(unsafe_code)]

//! Source-preserving INI-style section/key targeting.
//!
//! This is intentionally syntax-level only. It does not interpret
//! interpolation, duplicate-key semantics, or application-specific dialects.

use crate::engine::ByteEdit;
use crate::protocol::{Cardinality, RefusalReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum IniOperation {
    Set { path: String, value: String },
    Unset { path: String },
    EnsurePresent { path: String, value: String },
    EnsureAbsent { path: String },
    RenameKey { path: String, new_key: String },
    EnsureSection { section: String },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum IniError {
    #[error("Refused: {0:?}")]
    Refused(RefusalReason),
}

#[derive(Clone, Debug)]
struct Entry {
    start: usize,
    end: usize,
    key_start: usize,
    key_end: usize,
    value_start: usize,
    value_end: usize,
    key: String,
    section: Option<String>,
}

type Section = (String, usize);
type ParsedIni = (Vec<Entry>, Vec<Section>);

pub fn plan(
    content: &[u8],
    operation: &IniOperation,
    cardinality: &Cardinality,
) -> Result<Vec<ByteEdit>, IniError> {
    if !matches!(cardinality, Cardinality::ExactlyOne) {
        return Err(IniError::Refused(RefusalReason::CardinalityMismatch {
            expected: "exactly_one ini key or section".into(),
            actual: 1,
        }));
    }
    let text = std::str::from_utf8(content).map_err(|_| {
        IniError::Refused(RefusalReason::UnsupportedEncoding {
            details: "ini provider requires UTF-8".into(),
        })
    })?;
    let (entries, sections) = parse_lines(text)?;
    if let IniOperation::EnsureSection { section } = operation {
        if sections.iter().any(|(name, _)| name == section) {
            return Ok(Vec::new());
        }
        let newline = newline(text);
        let prefix = if text.is_empty() || text.ends_with('\n') || text.ends_with('\r') {
            ""
        } else {
            newline
        };
        return Ok(vec![ByteEdit {
            start: text.len(),
            end: text.len(),
            replacement: format!("{prefix}[{section}]{newline}").into_bytes(),
        }]);
    }
    let path = operation_path(operation);
    let (wanted_section, wanted_key) = split_path(path)?;
    let matches = entries
        .iter()
        .filter(|entry| entry.section.as_deref() == wanted_section && entry.key() == wanted_key)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(IniError::Refused(RefusalReason::DuplicateTarget {
            target: path.into(),
            count: matches.len(),
            candidates: Vec::new(),
            candidates_returned: 0,
            truncated: false,
        }));
    }
    let Some(entry) = matches.first() else {
        if matches!(
            operation,
            IniOperation::EnsureAbsent { .. } | IniOperation::Unset { .. }
        ) {
            return Ok(Vec::new());
        }
        let IniOperation::EnsurePresent { value, .. } = operation else {
            return Err(IniError::Refused(RefusalReason::MissingTarget {
                target: path.into(),
            }));
        };
        return insert_entry(text, &sections, wanted_section, wanted_key, value);
    };
    match operation {
        IniOperation::Set { value, .. } | IniOperation::EnsurePresent { value, .. } => {
            check_value(value)?;
            if text[entry.value_start..entry.value_end] == *value {
                return Ok(Vec::new());
            }
            Ok(vec![ByteEdit {
                start: entry.value_start,
                end: entry.value_end,
                replacement: value.as_bytes().to_vec(),
            }])
        }
        IniOperation::Unset { .. } | IniOperation::EnsureAbsent { .. } => Ok(vec![ByteEdit {
            start: entry.start,
            end: entry.end,
            replacement: Vec::new(),
        }]),
        IniOperation::RenameKey { new_key, .. } => {
            check_key(new_key)?;
            Ok(vec![ByteEdit {
                start: entry.key_start,
                end: entry.key_end,
                replacement: new_key.as_bytes().to_vec(),
            }])
        }
        IniOperation::EnsureSection { .. } => {
            Err(IniError::Refused(RefusalReason::UnsupportedOperation {
                operation: "ensure_section was handled before key planning".into(),
            }))
        }
    }
}

pub fn validate(content: &[u8]) -> Result<(), IniError> {
    let text = std::str::from_utf8(content).map_err(|_| {
        IniError::Refused(RefusalReason::UnsupportedEncoding {
            details: "ini provider requires UTF-8".into(),
        })
    })?;
    parse_lines(text).map(|_| ())
}

fn operation_path(operation: &IniOperation) -> &str {
    match operation {
        IniOperation::Set { path, .. }
        | IniOperation::Unset { path }
        | IniOperation::EnsurePresent { path, .. }
        | IniOperation::EnsureAbsent { path }
        | IniOperation::RenameKey { path, .. } => path,
        IniOperation::EnsureSection { section } => section,
    }
}

fn split_path(path: &str) -> Result<(Option<&str>, &str), IniError> {
    if path.is_empty() {
        return Err(IniError::Refused(RefusalReason::MissingTarget {
            target: "empty ini path".into(),
        }));
    }
    if let Some((section, key)) = path.rsplit_once('.') {
        if section.is_empty() || key.is_empty() {
            return Err(IniError::Refused(RefusalReason::MissingTarget {
                target: path.into(),
            }));
        }
        Ok((Some(section), key))
    } else {
        Ok((None, path))
    }
}

fn parse_lines(text: &str) -> Result<ParsedIni, IniError> {
    let mut entries = Vec::new();
    let mut sections = Vec::new();
    let mut current: Option<String> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let trimmed = body.trim_start().trim_start_matches('\u{feff}');
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            offset += line.len();
            continue;
        }
        if trimmed.starts_with('[') {
            if !trimmed.ends_with(']') {
                return Err(IniError::Refused(RefusalReason::MalformedInput {
                    details: "unterminated INI section header".into(),
                }));
            }
            let name = trimmed[1..trimmed.len() - 1].trim();
            if name.is_empty() {
                return Err(IniError::Refused(RefusalReason::MalformedInput {
                    details: "empty INI section header".into(),
                }));
            }
            current = Some(name.into());
            sections.push((name.into(), offset));
            offset += line.len();
            continue;
        }
        let Some(delimiter) = body.find(['=', ':']) else {
            return Err(IniError::Refused(RefusalReason::MalformedInput {
                details: format!(
                    "INI line has no '=' or ':' delimiter: {}",
                    trimmed.chars().take(64).collect::<String>()
                ),
            }));
        };
        let key = body[..delimiter].trim().trim_start_matches('\u{feff}');
        let key_start = offset + body[..delimiter].find(key).unwrap_or(0);
        let key_end = key_start + key.len();
        check_key(key)?;
        let value_area = &body[delimiter + 1..];
        let value_start = offset + delimiter + 1 + value_area.len() - value_area.trim_start().len();
        let value_end = offset + delimiter + 1 + value_area.trim_end().len();
        entries.push(Entry {
            start: offset,
            end: offset + line.len(),
            key_start,
            key_end,
            value_start,
            value_end,
            key: key.into(),
            section: current.clone(),
        });
        offset += line.len();
    }
    Ok((entries, sections))
}

impl Entry {
    fn key(&self) -> &str {
        &self.key
    }
}

fn insert_entry(
    text: &str,
    sections: &[Section],
    section: Option<&str>,
    key: &str,
    value: &str,
) -> Result<Vec<ByteEdit>, IniError> {
    check_key(key)?;
    check_value(value)?;
    let newline = newline(text);
    if let Some(section) = section {
        let Some((_, section_start)) = sections.iter().rev().find(|(name, _)| name == section)
        else {
            return Err(IniError::Refused(RefusalReason::MissingTarget {
                target: section.into(),
            }));
        };
        let next = sections
            .iter()
            .find(|(_, start)| *start > *section_start)
            .map(|(_, start)| *start)
            .unwrap_or(text.len());
        let insertion = format!("{} = {value}{newline}", key);
        let prefix = if *section_start < next && text[..next].ends_with(['\n', '\r']) {
            ""
        } else {
            newline
        };
        return Ok(vec![ByteEdit {
            start: next,
            end: next,
            replacement: format!("{prefix}{insertion}").into_bytes(),
        }]);
    }
    let prefix = if text.is_empty() || text.ends_with(['\n', '\r']) {
        ""
    } else {
        newline
    };
    Ok(vec![ByteEdit {
        start: text.len(),
        end: text.len(),
        replacement: format!("{prefix}{key} = {value}{newline}").into_bytes(),
    }])
}

fn check_key(key: &str) -> Result<(), IniError> {
    if key.trim().is_empty() || key.contains(['\r', '\n', '\0', '=', ':']) {
        return Err(IniError::Refused(RefusalReason::MalformedInput {
            details: "INI key contains forbidden syntax".into(),
        }));
    }
    Ok(())
}

fn check_value(value: &str) -> Result<(), IniError> {
    if value.contains(['\r', '\n', '\0']) {
        return Err(IniError::Refused(RefusalReason::MalformedInput {
            details: "INI value may not contain line breaks or NUL bytes".into(),
        }));
    }
    Ok(())
}

fn newline(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

impl From<RefusalReason> for IniError {
    fn from(reason: RefusalReason) -> Self {
        Self::Refused(reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::apply_byte_edits;

    #[test]
    fn section_key_set_preserves_spacing_and_crlf() {
        let source = "[server]\r\nport : 8080  \r\n# keep\r\n";
        let edits = plan(
            source.as_bytes(),
            &IniOperation::Set {
                path: "server.port".into(),
                value: "9090".into(),
            },
            &Cardinality::ExactlyOne,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(apply_byte_edits(source.as_bytes(), &edits).unwrap()).unwrap(),
            "[server]\r\nport : 9090  \r\n# keep\r\n"
        );
    }

    #[test]
    fn duplicate_keys_refuse_and_ensure_inserts_locally() {
        let duplicate = "[x]\na=1\na=2\n";
        assert!(matches!(
            plan(
                duplicate.as_bytes(),
                &IniOperation::Set {
                    path: "x.a".into(),
                    value: "3".into()
                },
                &Cardinality::ExactlyOne
            ),
            Err(IniError::Refused(RefusalReason::DuplicateTarget { .. }))
        ));
        let source = "[x]\na=1\n";
        let edits = plan(
            source.as_bytes(),
            &IniOperation::EnsurePresent {
                path: "x.b".into(),
                value: "2".into(),
            },
            &Cardinality::ExactlyOne,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(apply_byte_edits(source.as_bytes(), &edits).unwrap()).unwrap(),
            "[x]\na=1\nb = 2\n"
        );
    }
}
