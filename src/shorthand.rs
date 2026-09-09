#![forbid(unsafe_code)]

//! Shared construction for the small, convenience mutation commands.
//!
//! The command-line and MCP entrypoints deliberately remain thin adapters.
//! Target classification belongs to the target registry, so these two
//! surfaces cannot drift into separate extension tables.

use crate::protocol::OperationPayload;
use serde_json::Value;

/// Build the canonical structured value operation for a target path.
///
/// The registry resolves exact filenames and filename families as well as
/// extensions.  The operation-specific checks below only decide whether the
/// selected provider has a value-setting shorthand; they do not classify the
/// file independently.
pub fn set_value_operation(
    file: &str,
    path: &str,
    value: Value,
) -> Result<OperationPayload, String> {
    let detection = crate::target_registry::detect(file, None);
    match detection.provider.as_deref() {
        Some("json") => Ok(OperationPayload::Json(
            crate::provider::json::JsonOperation::Set {
                path: path.into(),
                value,
            },
        )),
        Some("jsonc") => Ok(OperationPayload::Jsonc(
            crate::provider::json::JsonOperation::Set {
                path: path.into(),
                value,
            },
        )),
        Some("yaml") => Ok(OperationPayload::Yaml(
            crate::provider::yaml::YamlOperation::Set {
                path: path.into(),
                value,
            },
        )),
        Some("toml") => {
            let value = serde_json::from_value(value)
                .map_err(|error| format!("TOML value is not representable: {error}"))?;
            Ok(OperationPayload::Toml(
                crate::provider::toml::TomlOperation::Set {
                    path: path.into(),
                    value,
                },
            ))
        }
        Some("ini") => match value {
            Value::String(value) => Ok(OperationPayload::Ini(
                crate::provider::ini::IniOperation::Set {
                    path: path.into(),
                    value,
                },
            )),
            _ => Err("INI values must be JSON strings".into()),
        },
        Some("dotenv") => match value {
            Value::String(value) => Ok(OperationPayload::Dotenv(
                crate::provider::dotenv::DotenvOperation::Set {
                    key: path.into(),
                    value,
                },
            )),
            _ => Err("dotenv values must be JSON strings".into()),
        },
        Some(provider) => Err(format!(
            "set-value is not supported for registry provider {provider} (target {})",
            detection.target_kind
        )),
        None if detection.confidence_class == "ambiguous" => Err(format!(
            "set-value target is ambiguous; choose one of: {}",
            detection.alternatives.join(", ")
        )),
        None => Err(format!(
            "set-value target is not a supported structured value target ({})",
            detection.target_kind
        )),
    }
}
