#![forbid(unsafe_code)]

use crate::engine::{apply_byte_edits, compute_sha256, ByteEdit};
use crate::path::PathNormalizer;
use crate::protocol::{
    ByteRange, Certificate, CommitGuarantee, FailureReason, MutationPlan, OperationPayload,
    Outcome, PreservationFacts, RefusalReason, Request, StructuralValidation, PROTOCOL_VERSION,
};
use crate::provider::json::{JsonProvider, JsonProviderError};
use crate::provider::text::{TextOperation, TextProvider, TextProviderError};
use crate::provider::toml::{TomlProvider, TomlProviderError};
use crate::workspace::{Workspace, WorkspaceError};

pub fn execute_pipeline(
    workspace: &Workspace,
    plan: &MutationPlan,
    op: &TextOperation,
    dry_run: bool,
) -> Certificate {
    let request = Request {
        version: plan.version.clone(),
        file_path: plan.file_path.clone(),
        namespace: Default::default(),
        expected_pre_hash: (!plan.expected_pre_hash.is_empty())
            .then(|| plan.expected_pre_hash.clone()),
        cardinality: plan.cardinality.clone(),
        operation: OperationPayload::Text(op.clone()),
    };
    execute_request(workspace, &request, dry_run)
}

pub fn execute_request(workspace: &Workspace, request: &Request, dry_run: bool) -> Certificate {
    let file_path = PathNormalizer::normalize(&request.file_path, &request.namespace);
    let provider = provider_name(&request.operation);
    if request.version != PROTOCOL_VERSION {
        return refusal(
            request,
            &file_path,
            provider,
            RefusalReason::UnsupportedProtocolVersion {
                requested: request.version.clone(),
                supported: PROTOCOL_VERSION.into(),
            },
            String::new(),
        );
    }
    let original = match workspace.read_file(&file_path) {
        Ok(b) => b,
        Err(e) => return workspace_error(request, &file_path, provider, e),
    };
    let pre_hash = compute_sha256(&original);
    if let Some(expected) = request
        .expected_pre_hash
        .as_deref()
        .filter(|s| !s.is_empty())
    {
        let expected = expected.strip_prefix("sha256:").unwrap_or(expected);
        if expected != pre_hash {
            return refusal(
                request,
                &file_path,
                provider,
                RefusalReason::StaleIdentity {
                    expected_hash: expected.into(),
                    actual_hash: pre_hash.clone(),
                },
                pre_hash,
            );
        }
    }
    if let Some(reason) = unsupported_encoding(&original) {
        return refusal(request, &file_path, provider, reason, pre_hash);
    }
    let edits = match plan_edits(&original, request) {
        Ok(x) => x,
        Err((r, _d)) => return refusal(request, &file_path, provider, r, pre_hash),
    };
    let engine_edits: Vec<ByteEdit> = edits
        .iter()
        .map(|e| ByteEdit {
            start: e.start,
            end: e.end,
            replacement: e.replacement.clone(),
        })
        .collect();
    let candidate = match apply_byte_edits(&original, &engine_edits) {
        Ok(x) => x,
        Err(e) => {
            return failure(
                request,
                &file_path,
                provider,
                pre_hash,
                FailureReason::InternalInvariant {
                    details: e.to_string(),
                },
            )
        }
    };
    let ranges = changed_ranges(&engine_edits);
    let provider_structural = match &request.operation {
        OperationPayload::Json(_) => {
            if let Err(e) = JsonProvider::validate(&candidate) {
                return failure(
                    request,
                    &file_path,
                    provider,
                    pre_hash,
                    FailureReason::ProviderError {
                        details: e.to_string(),
                    },
                );
            }
            StructuralValidation::Valid {
                format: "strict_json".into(),
            }
        }
        OperationPayload::Toml(_) => {
            let valid = std::str::from_utf8(&candidate)
                .ok()
                .and_then(|s| s.parse::<toml_edit::DocumentMut>().ok())
                .is_some();
            if !valid {
                return failure(
                    request,
                    &file_path,
                    provider,
                    pre_hash,
                    FailureReason::ProviderError {
                        details: "candidate TOML failed validation".into(),
                    },
                );
            }
            StructuralValidation::Valid {
                format: "toml".into(),
            }
        }
        OperationPayload::Text(_) => StructuralValidation::NotApplicable,
    };
    if candidate == original {
        return completed(
            request,
            &file_path,
            provider,
            pre_hash.clone(),
            Some(pre_hash.clone()),
            Outcome::NoChange,
            ranges,
            provider_structural,
            PreservationFacts::from_bytes(&original, &candidate),
            CommitGuarantee::default(),
            String::new(),
            false,
        );
    }
    let post_hash = compute_sha256(&candidate);
    let (diff, diff_truncated) = bounded_diff(&original, &engine_edits);
    if !dry_run {
        match workspace.write_file_atomic_checked(&file_path, &pre_hash, &candidate) {
            Ok(()) => {}
            Err(WorkspaceError::StaleIdentity { expected, actual }) => {
                return refusal(
                    request,
                    &file_path,
                    provider,
                    RefusalReason::StaleIdentity {
                        expected_hash: expected,
                        actual_hash: actual,
                    },
                    pre_hash,
                )
            }
            Err(e) => {
                return failure(
                    request,
                    &file_path,
                    provider,
                    pre_hash,
                    FailureReason::CommitFailure {
                        message: e.to_string(),
                    },
                )
            }
        }
    }
    let commit = if dry_run {
        CommitGuarantee {
            mode: "dry_run".into(),
            ..CommitGuarantee::default()
        }
    } else {
        CommitGuarantee {
            mode: "committed_atomic_replace".into(),
            content_replacement: "atomic replacement after staged flush".into(),
            permissions: "platform-dependent; not asserted".into(),
            timestamps: "not preserved".into(),
            acl_xattr: "unknown".into(),
        }
    };
    if !dry_run {
        let landed = match workspace.read_file(&file_path) {
            Ok(b) => b,
            Err(e) => {
                return failure(
                    request,
                    &file_path,
                    provider,
                    pre_hash,
                    FailureReason::PostCommitVerificationFailure {
                        expected_hash: post_hash.clone(),
                        actual_hash: format!("read failed: {e}"),
                    },
                )
            }
        };
        let actual = compute_sha256(&landed);
        if landed != candidate {
            return failure(
                request,
                &file_path,
                provider,
                pre_hash,
                FailureReason::PostCommitVerificationFailure {
                    expected_hash: post_hash,
                    actual_hash: actual,
                },
            );
        }
    }
    completed(
        request,
        &file_path,
        provider,
        pre_hash,
        Some(post_hash),
        Outcome::Applied,
        ranges,
        provider_structural,
        PreservationFacts::from_bytes(&original, &candidate),
        commit,
        diff,
        diff_truncated,
    )
}

fn plan_edits(
    original: &[u8],
    request: &Request,
) -> Result<Vec<ByteEdit>, (RefusalReason, String)> {
    match &request.operation {
        OperationPayload::Text(o) => {
            TextProvider::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    TextProviderError::Refused(r) => (r, detail),
                    TextProviderError::Error { message } => (
                        RefusalReason::Custom {
                            message: message.clone(),
                        },
                        message,
                    ),
                }
            })
        }
        OperationPayload::Json(o) => {
            JsonProvider::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    JsonProviderError::Refused(r) => (r, detail),
                    JsonProviderError::Error { message } => (
                        RefusalReason::Custom {
                            message: message.clone(),
                        },
                        message,
                    ),
                }
            })
        }
        OperationPayload::Toml(o) => {
            TomlProvider::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    TomlProviderError::Refused(r) => (r, detail),
                    TomlProviderError::Error { message } => (
                        RefusalReason::Custom {
                            message: message.clone(),
                        },
                        message,
                    ),
                }
            })
        }
    }
}
fn provider_name(op: &OperationPayload) -> &'static str {
    match op {
        OperationPayload::Text(_) => "text",
        OperationPayload::Json(_) => "json",
        OperationPayload::Toml(_) => "toml",
    }
}
fn provider_version(op: &OperationPayload) -> &'static str {
    match op {
        OperationPayload::Text(_) => "text-byte-v1",
        OperationPayload::Json(_) => "json-source-v1",
        OperationPayload::Toml(_) => "toml-edit-narrow-v1",
    }
}
fn unsupported_encoding(bytes: &[u8]) -> Option<RefusalReason> {
    if bytes.starts_with(&[0xff, 0xfe])
        || bytes.starts_with(&[0xfe, 0xff])
        || bytes.starts_with(&[0xff, 0xfe, 0, 0])
        || bytes.starts_with(&[0, 0, 0xfe, 0xff])
    {
        Some(RefusalReason::UnsupportedEncoding {
            details: "UTF-16/UTF-32 is not part of v0.1".into(),
        })
    } else if std::str::from_utf8(bytes).is_err() {
        Some(RefusalReason::UnsupportedEncoding {
            details: "input is not valid UTF-8".into(),
        })
    } else {
        None
    }
}
fn changed_ranges(edits: &[ByteEdit]) -> Vec<ByteRange> {
    edits
        .iter()
        .filter(|e| e.start != e.end || !e.replacement.is_empty())
        .map(|e| ByteRange {
            start: e.start,
            end: e.end,
        })
        .collect()
}
fn bounded_diff(original: &[u8], edits: &[ByteEdit]) -> (String, bool) {
    const LIMIT: usize = 4096;
    let mut d = String::from("byte-range diff (unchanged bytes omitted):\n");
    for edit in edits {
        let old = String::from_utf8_lossy(&original[edit.start..edit.end]);
        let new = String::from_utf8_lossy(&edit.replacement);
        d.push_str(&format!(
            "@@ {}..{} @@\n-{}\n+{}\n",
            edit.start, edit.end, old, new
        ));
    }
    if d.len() > LIMIT {
        let end = d
            .char_indices()
            .take_while(|(i, _)| *i < LIMIT)
            .map(|(i, _)| i)
            .last()
            .unwrap_or(0);
        (
            format!(
                "{}\n[diff truncated; total characters: {}]",
                &d[..end],
                d.len()
            ),
            true,
        )
    } else {
        (d, false)
    }
}
fn refusal(
    r: &Request,
    path: &str,
    provider: &str,
    reason: RefusalReason,
    pre: String,
) -> Certificate {
    completed(
        r,
        path,
        provider,
        pre,
        None,
        Outcome::Refused,
        Vec::new(),
        StructuralValidation::NotApplicable,
        PreservationFacts::default(),
        CommitGuarantee::default(),
        format!("{reason:?}"),
        false,
    )
    .with_reason(reason)
}
fn failure(
    r: &Request,
    path: &str,
    provider: &str,
    pre: String,
    reason: FailureReason,
) -> Certificate {
    let mut c = completed(
        r,
        path,
        provider,
        pre,
        None,
        Outcome::Failed,
        Vec::new(),
        StructuralValidation::NotApplicable,
        PreservationFacts::default(),
        CommitGuarantee::default(),
        String::new(),
        false,
    );
    c.failure_reason = Some(reason);
    c
}
fn workspace_error(r: &Request, path: &str, provider: &str, e: WorkspaceError) -> Certificate {
    match e {
        WorkspaceError::Traversal(p) => refusal(
            r,
            path,
            provider,
            RefusalReason::WorkspaceTraversal { path: p.clone() },
            String::new(),
        ),
        WorkspaceError::SymlinkEscape(p) => refusal(
            r,
            path,
            provider,
            RefusalReason::SymlinkEscape { path: p.clone() },
            String::new(),
        ),
        WorkspaceError::NotFound(p) => refusal(
            r,
            path,
            provider,
            RefusalReason::MissingTarget { target: p.clone() },
            String::new(),
        ),
        WorkspaceError::StaleIdentity { expected, actual } => refusal(
            r,
            path,
            provider,
            RefusalReason::StaleIdentity {
                expected_hash: expected,
                actual_hash: actual,
            },
            String::new(),
        ),
        WorkspaceError::Io(e) => failure(
            r,
            path,
            provider,
            String::new(),
            FailureReason::IoError {
                message: e.to_string(),
            },
        ),
    }
}
#[allow(clippy::too_many_arguments)]
fn completed(
    r: &Request,
    path: &str,
    provider: &str,
    pre: String,
    post: Option<String>,
    outcome: Outcome,
    ranges: Vec<ByteRange>,
    structural: StructuralValidation,
    preservation: PreservationFacts,
    commit: CommitGuarantee,
    diff: String,
    truncated: bool,
) -> Certificate {
    Certificate {
        protocol_version: r.version.clone(),
        outcome,
        file_path: path.into(),
        provider: provider.into(),
        provider_version: provider_version(&r.operation).into(),
        expected_cardinality: r.cardinality.clone(),
        observed_cardinality: Some(1),
        pre_hash: pre,
        post_hash: post,
        changed_ranges: ranges,
        diff_summary: Some(diff),
        diff_truncated: truncated,
        structural_validation: structural,
        preservation,
        commit,
        refusal_reason: None,
        failure_reason: None,
        diagnostics: Vec::new(),
    }
}
trait WithReason {
    fn with_reason(self, r: RefusalReason) -> Self;
}
impl WithReason for Certificate {
    fn with_reason(mut self, r: RefusalReason) -> Self {
        self.refusal_reason = Some(r);
        self
    }
}
impl PreservationFacts {
    fn from_bytes(a: &[u8], b: &[u8]) -> Self {
        Self {
            unrelated_bytes_changed: false,
            line_endings_changed: a.contains(&b'\r') != b.contains(&b'\r'),
            bom_changed: a.starts_with(&[0xef, 0xbb, 0xbf]) != b.starts_with(&[0xef, 0xbb, 0xbf]),
            final_newline_changed: a.ends_with(b"\n") != b.ends_with(b"\n"),
            comments_preserved: Some(comment_count(a) == comment_count(b)),
            metadata: "content-only verification; replacement metadata not asserted".into(),
        }
    }
}
fn comment_count(b: &[u8]) -> usize {
    b.split(|x| *x == b'\n')
        .filter(|l| {
            l.iter()
                .position(|x| !*x == b' ' && !*x == b'\t')
                .map(|i| l[i..].starts_with(b"#"))
                .unwrap_or(false)
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Cardinality, OperationPayload};
    use crate::provider::text::TextOperation;
    use tempfile::TempDir;
    #[test]
    fn apply_and_certify_landed_bytes() {
        let t = TempDir::new().unwrap();
        let w = Workspace::new(t.path()).unwrap();
        std::fs::write(t.path().join("x.txt"), b"a b\n").unwrap();
        let r = Request {
            version: PROTOCOL_VERSION.into(),
            file_path: "x.txt".into(),
            namespace: Default::default(),
            expected_pre_hash: None,
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "b".into(),
                replacement: "c".into(),
            }),
        };
        let c = execute_request(&w, &r, false);
        let expected = compute_sha256(b"a c\n");
        assert_eq!(c.outcome, Outcome::Applied);
        assert_eq!(c.post_hash.as_deref(), Some(expected.as_str()));
        assert_eq!(w.read_file("x.txt").unwrap(), b"a c\n");
    }
    #[test]
    fn dry_run_is_non_mutating() {
        let t = TempDir::new().unwrap();
        let w = Workspace::new(t.path()).unwrap();
        std::fs::write(t.path().join("x.txt"), b"a b").unwrap();
        let r = Request {
            version: PROTOCOL_VERSION.into(),
            file_path: "x.txt".into(),
            namespace: Default::default(),
            expected_pre_hash: None,
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "b".into(),
                replacement: "c".into(),
            }),
        };
        let c = execute_request(&w, &r, true);
        assert_eq!(c.outcome, Outcome::Applied);
        assert_eq!(w.read_file("x.txt").unwrap(), b"a b");
    }
}
