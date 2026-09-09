#![forbid(unsafe_code)]

//! The canonical file-coverage registry used by discovery surfaces.
//!
//! Syntax grammars remain an implementation detail of the syntax provider;
//! this registry owns the user-visible classification and deterministic file
//! detection rules.

use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetCategory {
    ProgrammingLanguage,
    DeclarativeLanguage,
    SyntaxVariant,
    StructuredFormat,
    Markup,
    Document,
    ExactText,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnderstandingLevel {
    Structured,
    Syntax,
    Region,
    Exact,
    Opaque,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreservationLevel {
    UnrelatedBytes,
    BoundedRegion,
    ExplicitDesiredState,
    Unavailable,
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct TargetSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: TargetCategory,
    pub provider: Option<&'static str>,
    pub aliases: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub exact_filenames: &'static [&'static str],
    pub understanding_level: UnderstandingLevel,
    pub preservation_level: PreservationLevel,
    pub fallback_available: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Detection {
    pub target_kind: String,
    pub provider: Option<String>,
    pub basis: String,
    pub confidence_class: String,
    pub alternatives: Vec<String>,
    pub understanding_level: UnderstandingLevel,
    pub preservation_level: PreservationLevel,
    pub fallback_routes: Vec<String>,
}

static REGISTRY: &[TargetSpec] = &[
    TargetSpec {
        id: "javascript",
        display_name: "JavaScript",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["js"],
        extensions: &[".js", ".mjs", ".cjs"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "jsx",
        display_name: "JSX",
        category: TargetCategory::SyntaxVariant,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".jsx"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "typescript",
        display_name: "TypeScript",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["ts"],
        extensions: &[".ts"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "tsx",
        display_name: "TSX",
        category: TargetCategory::SyntaxVariant,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".tsx"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "python",
        display_name: "Python",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["py"],
        extensions: &[".py"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "rust",
        display_name: "Rust",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["rs"],
        extensions: &[".rs"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "go",
        display_name: "Go",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["golang"],
        extensions: &[".go"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "c",
        display_name: "C",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".c"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "cpp",
        display_name: "C++",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["c++"],
        extensions: &[".cpp", ".cc", ".cxx", ".hpp"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "java",
        display_name: "Java",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".java"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "csharp",
        display_name: "C#",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["cs", "c#"],
        extensions: &[".cs"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "php",
        display_name: "PHP",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".php"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "hcl",
        display_name: "HCL / Terraform syntax",
        category: TargetCategory::DeclarativeLanguage,
        provider: Some("code"),
        aliases: &["terraform", "tf"],
        extensions: &[".tf", ".tfvars", ".hcl"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "bash",
        display_name: "Bash / Shell",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["sh", "shell"],
        extensions: &[".sh", ".bash"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "powershell",
        display_name: "PowerShell",
        category: TargetCategory::ProgrammingLanguage,
        provider: Some("code"),
        aliases: &["pwsh"],
        extensions: &[".ps1", ".psm1", ".psd1"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "sql",
        display_name: "SQL",
        category: TargetCategory::DeclarativeLanguage,
        provider: Some("code"),
        aliases: &[],
        extensions: &[".sql"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "html",
        display_name: "HTML",
        category: TargetCategory::Markup,
        provider: Some("web"),
        aliases: &[],
        extensions: &[".html", ".htm"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "css",
        display_name: "CSS",
        category: TargetCategory::Markup,
        provider: Some("web"),
        aliases: &[],
        extensions: &[".css"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "xml",
        display_name: "XML",
        category: TargetCategory::Markup,
        provider: Some("web"),
        aliases: &[],
        extensions: &[".xml"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Syntax,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "json",
        display_name: "JSON",
        category: TargetCategory::StructuredFormat,
        provider: Some("json"),
        aliases: &[],
        extensions: &[".json"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: false,
    },
    TargetSpec {
        id: "jsonc",
        display_name: "JSON with comments",
        category: TargetCategory::StructuredFormat,
        provider: Some("jsonc"),
        aliases: &[],
        extensions: &[".jsonc"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "toml",
        display_name: "TOML",
        category: TargetCategory::StructuredFormat,
        provider: Some("toml"),
        aliases: &[],
        extensions: &[".toml"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "yaml",
        display_name: "YAML",
        category: TargetCategory::StructuredFormat,
        provider: Some("yaml"),
        aliases: &["yml"],
        extensions: &[".yaml", ".yml"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "ini",
        display_name: "INI-style configuration",
        category: TargetCategory::StructuredFormat,
        provider: Some("ini"),
        aliases: &[],
        extensions: &[".ini"],
        exact_filenames: &["setup.cfg", "tox.ini", "pytest.ini", ".editorconfig"],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "dotenv",
        display_name: "dotenv",
        category: TargetCategory::StructuredFormat,
        provider: Some("dotenv"),
        aliases: &[],
        extensions: &[".env"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Structured,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "markdown",
        display_name: "Markdown regions",
        category: TargetCategory::Document,
        provider: Some("markdown"),
        aliases: &["md"],
        extensions: &[".md", ".markdown"],
        exact_filenames: &[],
        understanding_level: UnderstandingLevel::Region,
        preservation_level: PreservationLevel::BoundedRegion,
        fallback_available: true,
    },
    TargetSpec {
        id: "dockerfile",
        display_name: "Dockerfile text",
        category: TargetCategory::ExactText,
        provider: Some("text"),
        aliases: &[],
        extensions: &[],
        exact_filenames: &["dockerfile"],
        understanding_level: UnderstandingLevel::Exact,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
    TargetSpec {
        id: "makefile",
        display_name: "Makefile text",
        category: TargetCategory::ExactText,
        provider: Some("text"),
        aliases: &[],
        extensions: &[],
        exact_filenames: &["makefile", "gnumakefile"],
        understanding_level: UnderstandingLevel::Exact,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_available: true,
    },
];

pub fn registry() -> &'static [TargetSpec] {
    REGISTRY
}

pub fn lookup(id_or_alias: &str) -> Option<&'static TargetSpec> {
    let wanted = id_or_alias.to_ascii_lowercase();
    REGISTRY
        .iter()
        .find(|spec| spec.id == wanted || spec.aliases.iter().any(|alias| *alias == wanted))
}

pub fn detect(path: &str, bytes: Option<&[u8]>) -> Detection {
    let lower = path.to_ascii_lowercase();
    let file_name = std::path::Path::new(&lower)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if let Some(bytes) = bytes {
        if std::str::from_utf8(bytes).is_err() {
            return opaque("invalid UTF-8");
        }
        if bytes.contains(&0) {
            return opaque("NUL-heavy or binary content");
        }
    }
    if let Some(spec) = REGISTRY
        .iter()
        .find(|spec| spec.exact_filenames.contains(&file_name))
    {
        return from_spec(
            spec,
            format!("recognized exact filename {file_name}"),
            "exact_filename",
        );
    }
    if file_name == ".env" || file_name.starts_with(".env.") {
        return from_spec(
            lookup("dotenv").expect("dotenv is registered"),
            "recognized .env filename family".into(),
            "filename_family",
        );
    }
    let extension = std::path::Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"));
    if let Some(extension) = extension {
        if let Some(spec) = REGISTRY
            .iter()
            .find(|spec| spec.extensions.contains(&extension.as_str()))
        {
            return from_spec(
                spec,
                format!("registered extension {extension}"),
                "extension",
            );
        }
    }
    let Some(bytes) = bytes else {
        return exact_unknown();
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return opaque("invalid UTF-8");
    };
    let trimmed = text.trim_start();
    if let Some(shebang) = trimmed.lines().next().filter(|line| line.starts_with("#!")) {
        let interpreter = shebang.to_ascii_lowercase();
        let target = if interpreter.contains("python") {
            Some("python")
        } else if interpreter.contains("pwsh") || interpreter.contains("powershell") {
            Some("powershell")
        } else if interpreter.contains("bash")
            || interpreter.ends_with("/sh")
            || interpreter.contains("/env sh")
        {
            Some("bash")
        } else if interpreter.contains("node") {
            Some("javascript")
        } else {
            None
        };
        if let Some(target) = target {
            return from_spec(
                lookup(target).expect("shebang target is registered"),
                "recognized shebang".into(),
                "shebang",
            );
        }
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Detection {
            target_kind: "ambiguous_structured_text".into(),
            provider: None,
            basis: "content resembles several structured formats".into(),
            confidence_class: "ambiguous".into(),
            alternatives: vec!["json".into(), "jsonc".into(), "yaml".into()],
            understanding_level: UnderstandingLevel::Exact,
            preservation_level: PreservationLevel::UnrelatedBytes,
            fallback_routes: vec!["exact".into(), "patch".into(), "desired_state".into()],
        };
    }
    exact_unknown()
}

fn from_spec(spec: &TargetSpec, basis: String, confidence: &str) -> Detection {
    Detection {
        target_kind: spec.id.into(),
        provider: spec.provider.map(str::to_owned),
        basis,
        confidence_class: confidence.into(),
        alternatives: Vec::new(),
        understanding_level: spec.understanding_level,
        preservation_level: spec.preservation_level,
        fallback_routes: if spec.fallback_available {
            vec!["exact".into(), "patch".into(), "desired_state".into()]
        } else {
            Vec::new()
        },
    }
}

fn exact_unknown() -> Detection {
    Detection {
        target_kind: "unknown_text".into(),
        provider: Some("text".into()),
        basis: "valid UTF-8 without a more-specific registered target".into(),
        confidence_class: "fallback".into(),
        alternatives: Vec::new(),
        understanding_level: UnderstandingLevel::Exact,
        preservation_level: PreservationLevel::UnrelatedBytes,
        fallback_routes: vec!["exact".into(), "patch".into(), "desired_state".into()],
    }
}

fn opaque(basis: &str) -> Detection {
    Detection {
        target_kind: "opaque".into(),
        provider: None,
        basis: basis.into(),
        confidence_class: "certain".into(),
        alternatives: Vec::new(),
        understanding_level: UnderstandingLevel::Opaque,
        preservation_level: PreservationLevel::Unavailable,
        fallback_routes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_unique_ids_and_unambiguous_extensions() {
        for (index, left) in REGISTRY.iter().enumerate() {
            assert!(REGISTRY[index + 1..]
                .iter()
                .all(|right| right.id != left.id));
            for right in &REGISTRY[index + 1..] {
                assert!(left
                    .extensions
                    .iter()
                    .all(|extension| !right.extensions.contains(extension)));
            }
        }
    }

    #[test]
    fn detection_distinguishes_exact_text_and_opaque() {
        assert_eq!(
            detect("notes.conf", Some(b"key = value")).understanding_level,
            UnderstandingLevel::Exact
        );
        assert_eq!(
            detect("data.bin", Some(&[0, 1, 2])).understanding_level,
            UnderstandingLevel::Opaque
        );
        assert_eq!(
            detect("src/Main.java", Some(b"class Main {}")).target_kind,
            "java"
        );
    }
}
