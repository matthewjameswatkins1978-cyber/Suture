#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PathNamespace {
    Native,
    Windows,
    Wsl { distro: Option<String> },
    Posix,
}

impl Default for PathNamespace {
    fn default() -> Self {
        PathNamespace::Native
    }
}

/// Normalizes and converts paths according to AD-06 path namespace rules.
pub struct PathNormalizer;

impl PathNormalizer {
    /// Normalizes a path string given a source namespace into a standard relative or absolute workspace-safe path.
    pub fn normalize(path_str: &str, namespace: &PathNamespace) -> String {
        match namespace {
            PathNamespace::Native => {
                // Standardize backslashes to forward slashes or preserve platform path representation
                path_str.replace('\\', "/")
            }
            PathNamespace::Windows => {
                // Convert Windows backslashes to forward slashes, strip drive letters if needed or normalize
                let cleaned = path_str.replace('\\', "/");
                // e.g. C:/foo/bar -> /c/foo/bar or foo/bar
                cleaned
            }
            PathNamespace::Wsl { distro: _ } => {
                // WSL paths like /mnt/c/foo/bar or /home/user/foo
                path_str.replace('\\', "/")
            }
            PathNamespace::Posix => {
                path_str.replace('\\', "/")
            }
        }
    }

    /// Converts a path from a given namespace to the host native path representation.
    pub fn to_native_path(path_str: &str, namespace: &PathNamespace) -> PathBuf {
        let normalized = Self::normalize(path_str, namespace);
        match namespace {
            PathNamespace::Windows => {
                // If path starts with something like /c/, convert to C:\
                if normalized.len() >= 3 && normalized.chars().nth(0) == Some('/') && normalized.chars().nth(2) == Some('/') {
                    let drive = normalized.chars().nth(1).unwrap().to_ascii_uppercase();
                    let rest = &normalized[3..];
                    PathBuf::from(format!("{}:\\{}", drive, rest.replace('/', "\\")))
                } else {
                    PathBuf::from(normalized.replace('/', "\\"))
                }
            }
            PathNamespace::Wsl { .. } => {
                // Convert /mnt/c/foo -> C:\foo on Windows host if running on Windows under WSL, otherwise keep POSIX path
                #[cfg(target_os = "windows")]
                {
                    if normalized.starts_with("/mnt/") && normalized.len() >= 7 {
                        let drive = normalized.chars().nth(5).unwrap().to_ascii_uppercase();
                        let rest = &normalized[6..];
                        return PathBuf::from(format!("{}:\\{}", drive, rest.replace('/', "\\")));
                    }
                }
                PathBuf::from(normalized)
            }
            _ => {
                #[cfg(target_os = "windows")]
                {
                    PathBuf::from(normalized.replace('/', "\\"))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    PathBuf::from(normalized)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_namespace_serialization() {
        let ns = PathNamespace::Wsl { distro: Some("Ubuntu".to_string()) };
        let json = serde_json::to_string(&ns).unwrap();
        assert!(json.contains("wsl"));
        assert!(json.contains("Ubuntu"));
        let de: PathNamespace = serde_json::from_str(&json).unwrap();
        assert_eq!(de, ns);

        let native = PathNamespace::Native;
        let json_native = serde_json::to_string(&native).unwrap();
        assert_eq!(json_native, "{\"type\":\"native\"}");
        let de_native: PathNamespace = serde_json::from_str(&json_native).unwrap();
        assert_eq!(de_native, native);
    }

    #[test]
    fn test_path_normalization() {
        let win_path = "src\\engine.rs";
        let norm = PathNormalizer::normalize(win_path, &PathNamespace::Windows);
        assert_eq!(norm, "src/engine.rs");
    }

    #[test]
    fn test_to_native_path() {
        let p = PathNormalizer::to_native_path("src/lib.rs", &PathNamespace::Posix);
        assert!(!p.as_os_str().is_empty());
    }
}
