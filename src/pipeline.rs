#![forbid(unsafe_code)]

use crate::engine::{apply_byte_edits, compute_sha256, generate_diff};
use crate::path::PathNormalizer;
use crate::protocol::{
    Certificate, FailureReason, MutationPlan, OperationPayload, Outcome, RefusalReason, Request,
};
use crate::provider::json::{JsonProvider, JsonProviderError};
use crate::provider::text::{TextOperation, TextProvider, TextProviderError};
use crate::provider::toml::{TomlProvider, TomlProviderError};
use crate::workspace::{Workspace, WorkspaceError};

/// Executes the core mutation pipeline:
/// Observe -> Guard -> Mutate -> Verify -> Certify
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
        expected_pre_hash: if plan.expected_pre_hash.is_empty() {
            None
        } else {
            Some(plan.expected_pre_hash.clone())
        },
        cardinality: plan.cardinality.clone(),
        operation: OperationPayload::Text(op.clone()),
    };
    execute_request(workspace, &request, dry_run)
}

/// Executes a top-level Request routing through Text, JSON, or TOML providers.
pub fn execute_request(workspace: &Workspace, request: &Request, dry_run: bool) -> Certificate {
    let normalized_file_path = PathNormalizer::normalize(&request.file_path, &request.namespace);
    let file_path = normalized_file_path.clone();

    // 1. Observe: Read target file via Workspace::read_file and compute actual pre-hash.
    let original_bytes = match workspace.read_file(&file_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return match e {
                WorkspaceError::Traversal(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: String::new(),
                    post_hash: None,
                    refusal_reason: Some(RefusalReason::WorkspaceTraversal { path: p }),
                    failure_reason: None,
                    diff_summary: None,
                },
                WorkspaceError::SymlinkEscape(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: String::new(),
                    post_hash: None,
                    refusal_reason: Some(RefusalReason::SymlinkEscape { path: p }),
                    failure_reason: None,
                    diff_summary: None,
                },
                WorkspaceError::NotFound(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: String::new(),
                    post_hash: None,
                    refusal_reason: Some(RefusalReason::MissingTarget { target: p }),
                    failure_reason: None,
                    diff_summary: None,
                },
                WorkspaceError::Io(io_err) => {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        Certificate {
                            outcome: Outcome::Refused,
                            file_path: file_path.clone(),
                            pre_hash: String::new(),
                            post_hash: None,
                            refusal_reason: Some(RefusalReason::MissingTarget {
                                target: file_path.clone(),
                            }),
                            failure_reason: None,
                            diff_summary: None,
                        }
                    } else {
                        Certificate {
                            outcome: Outcome::Failed,
                            file_path: file_path.clone(),
                            pre_hash: String::new(),
                            post_hash: None,
                            refusal_reason: None,
                            failure_reason: Some(FailureReason::IoError {
                                message: io_err.to_string(),
                            }),
                            diff_summary: None,
                        }
                    }
                }
            };
        }
    };

    let actual_pre_hash = compute_sha256(&original_bytes);

    // 2. Guard: Verify expected_pre_hash if provided
    if let Some(ref expected) = request.expected_pre_hash {
        if !expected.is_empty() && expected != &actual_pre_hash {
            return Certificate {
                outcome: Outcome::Refused,
                file_path,
                pre_hash: actual_pre_hash.clone(),
                post_hash: None,
                refusal_reason: Some(RefusalReason::StaleIdentity {
                    expected_hash: expected.clone(),
                    actual_hash: actual_pre_hash,
                }),
                failure_reason: None,
                diff_summary: None,
            };
        }
    }

    // 3. Mutate / Plan edits across providers
    let edits = match &request.operation {
        OperationPayload::Text(op) => {
            match TextProvider::plan(&original_bytes, op, &request.cardinality) {
                Ok(edits) => edits,
                Err(err) => match err {
                    TextProviderError::Refused(reason) => {
                        return Certificate {
                            outcome: Outcome::Refused,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: Some(reason),
                            failure_reason: None,
                            diff_summary: None,
                        };
                    }
                    TextProviderError::Error { message } => {
                        return Certificate {
                            outcome: Outcome::Failed,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: None,
                            failure_reason: Some(FailureReason::ParseError { details: message }),
                            diff_summary: None,
                        };
                    }
                },
            }
        }
        OperationPayload::Json(op) => {
            match JsonProvider::plan(&original_bytes, op, &request.cardinality) {
                Ok(edits) => edits,
                Err(err) => match err {
                    JsonProviderError::Refused(reason) => {
                        return Certificate {
                            outcome: Outcome::Refused,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: Some(reason),
                            failure_reason: None,
                            diff_summary: None,
                        };
                    }
                    JsonProviderError::Error { message } => {
                        return Certificate {
                            outcome: Outcome::Failed,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: None,
                            failure_reason: Some(FailureReason::ParseError { details: message }),
                            diff_summary: None,
                        };
                    }
                },
            }
        }
        OperationPayload::Toml(op) => {
            match TomlProvider::plan(&original_bytes, op, &request.cardinality) {
                Ok(edits) => edits,
                Err(err) => match err {
                    TomlProviderError::Refused(reason) => {
                        return Certificate {
                            outcome: Outcome::Refused,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: Some(reason),
                            failure_reason: None,
                            diff_summary: None,
                        };
                    }
                    TomlProviderError::Error { message } => {
                        return Certificate {
                            outcome: Outcome::Failed,
                            file_path,
                            pre_hash: actual_pre_hash,
                            post_hash: None,
                            refusal_reason: None,
                            failure_reason: Some(FailureReason::ParseError { details: message }),
                            diff_summary: None,
                        };
                    }
                },
            }
        }
    };

    // 4. Apply byte edits and check NoChange
    let candidate_bytes = match apply_byte_edits(
        &original_bytes,
        &edits
            .iter()
            .map(|e| crate::engine::ByteEdit {
                start: e.start,
                end: e.end,
                replacement: e.replacement.clone(),
            })
            .collect::<Vec<_>>(),
    ) {
        Ok(bytes) => bytes,
        Err(engine_err) => {
            return Certificate {
                outcome: Outcome::Failed,
                file_path,
                pre_hash: actual_pre_hash,
                post_hash: None,
                refusal_reason: None,
                failure_reason: Some(FailureReason::ParseError {
                    details: engine_err.to_string(),
                }),
                diff_summary: None,
            };
        }
    };

    if candidate_bytes == original_bytes {
        return Certificate {
            outcome: Outcome::NoChange,
            file_path,
            pre_hash: actual_pre_hash.clone(),
            post_hash: Some(actual_pre_hash),
            refusal_reason: None,
            failure_reason: None,
            diff_summary: Some(String::new()),
        };
    }

    // 5. Verify: Compute candidate post_hash and bounded unified diff
    let post_hash = compute_sha256(&candidate_bytes);
    let diff_summary = generate_diff(&original_bytes, &candidate_bytes);

    // 6. Commit or Dry-run
    if !dry_run {
        if let Err(e) = workspace.write_file_atomic(&file_path, &candidate_bytes) {
            let err_msg = e.to_string();
            return match e {
                WorkspaceError::Traversal(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::WorkspaceTraversal { path: p }),
                    failure_reason: Some(FailureReason::WriteError { message: err_msg }),
                    diff_summary: Some(diff_summary),
                },
                WorkspaceError::SymlinkEscape(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::SymlinkEscape { path: p }),
                    failure_reason: Some(FailureReason::WriteError { message: err_msg }),
                    diff_summary: Some(diff_summary),
                },
                WorkspaceError::NotFound(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::MissingTarget { target: p }),
                    failure_reason: Some(FailureReason::WriteError { message: err_msg }),
                    diff_summary: Some(diff_summary),
                },
                WorkspaceError::Io(io_err) => Certificate {
                    outcome: Outcome::Failed,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: None,
                    failure_reason: Some(FailureReason::WriteError {
                        message: io_err.to_string(),
                    }),
                    diff_summary: Some(diff_summary),
                },
            };
        }
    }

    // 7. Certify
    Certificate {
        outcome: Outcome::Applied,
        file_path,
        pre_hash: actual_pre_hash,
        post_hash: Some(post_hash),
        refusal_reason: None,
        failure_reason: None,
        diff_summary: Some(diff_summary),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Cardinality, OperationPayload};
    use tempfile::TempDir;

    #[test]
    fn test_pipeline_successful_apply() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let pre_hash = compute_sha256(b"Hello World\n");
        let request = Request {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            namespace: Default::default(),
            expected_pre_hash: Some(pre_hash.clone()),
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "World".to_string(),
                replacement: "Suture".to_string(),
            }),
        };

        let cert = execute_request(&ws, &request, false);
        assert_eq!(cert.outcome, Outcome::Applied);
        assert_eq!(cert.pre_hash, pre_hash);
        assert!(cert.post_hash.is_some());
        assert_ne!(cert.post_hash.as_ref().unwrap(), &pre_hash);
        let diff = cert.diff_summary.as_ref().unwrap();
        assert!(diff.contains("-Hello World"));
        assert!(diff.contains("+Hello Suture"));

        let updated = std::fs::read_to_string(tmp.path().join(file_path)).unwrap();
        assert_eq!(updated, "Hello Suture\n");
    }

    #[test]
    fn test_pipeline_dry_run() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let pre_hash = compute_sha256(b"Hello World\n");
        let request = Request {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            namespace: Default::default(),
            expected_pre_hash: Some(pre_hash.clone()),
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "World".to_string(),
                replacement: "Suture".to_string(),
            }),
        };

        let cert = execute_request(&ws, &request, true);
        assert_eq!(cert.outcome, Outcome::Applied);
        assert!(cert.post_hash.is_some());
        assert!(cert.diff_summary.is_some());

        let disk_content = std::fs::read_to_string(tmp.path().join(file_path)).unwrap();
        assert_eq!(disk_content, "Hello World\n");
    }

    #[test]
    fn test_pipeline_stale_identity() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let request = Request {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            namespace: Default::default(),
            expected_pre_hash: Some("stale_hash_value".to_string()),
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "World".to_string(),
                replacement: "Suture".to_string(),
            }),
        };

        let cert = execute_request(&ws, &request, false);
        assert_eq!(cert.outcome, Outcome::Refused);
        match cert.refusal_reason.unwrap() {
            RefusalReason::StaleIdentity {
                expected_hash,
                actual_hash,
            } => {
                assert_eq!(expected_hash, "stale_hash_value");
                assert_ne!(actual_hash, "stale_hash_value");
            }
            other => panic!("Expected StaleIdentity, got {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_workspace_traversal_refusal() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let request = Request {
            version: "0.1.0".to_string(),
            file_path: "../outside.txt".to_string(),
            namespace: Default::default(),
            expected_pre_hash: None,
            cardinality: Cardinality::ExactlyOne,
            operation: OperationPayload::Text(TextOperation::Replace {
                target: "a".to_string(),
                replacement: "b".to_string(),
            }),
        };

        let cert = execute_request(&ws, &request, false);
        assert_eq!(cert.outcome, Outcome::Refused);
        match cert.refusal_reason.unwrap() {
            RefusalReason::WorkspaceTraversal { .. } => {}
            other => panic!("Expected WorkspaceTraversal, got {:?}", other),
        }
    }
}
