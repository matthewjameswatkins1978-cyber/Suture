#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FileOperation {
    CreateFile {
        expected_absent: bool,
        content: Vec<u8>,
    },
    DeleteFile {
        expected_hash: String,
    },
    RenameFile {
        destination: String,
        expected_source_hash: String,
        destination_absent: bool,
    },
    MoveFile {
        destination: String,
        expected_source_hash: String,
        destination_absent: bool,
    },
}
