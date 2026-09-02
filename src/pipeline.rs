#![forbid(unsafe_code)]

use crate::engine::{apply_byte_edits, compute_sha256, generate_diff};
use crate::protocol::{Certificate, FailureReason, MutationPlan, Outcome, RefusalReason};
use crate::provider::text::{TextOperation, TextProvider, TextProviderError};
use crate::workspace::{Workspace, WorkspaceError};

/// Executes the core mutation pipeline:
/// Observe -> Guard -> Mutate -> Verify -> Certify
pub fn execute_pipeline(
    workspace: &Workspace,
    plan: &MutationPlan,
    op: &TextOperation,
    dry_run: bool,
) -> Certificate {
    let file_path = plan.file_path.clone();

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
                    // Check if it's a NotFound or permission/traversal error
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

    // 2. Guard: Verify expected_pre_hash if provided; if mismatched, return Certificate with Outcome::Refused and RefusalReason::StaleIdentity.
    if !plan.expected_pre_hash.is_empty() && plan.expected_pre_hash != actual_pre_hash {
        return Certificate {
            outcome: Outcome::Refused,
            file_path,
            pre_hash: actual_pre_hash.clone(),
            post_hash: None,
            refusal_reason: Some(RefusalReason::StaleIdentity {
                expected_hash: plan.expected_pre_hash.clone(),
                actual_hash: actual_pre_hash,
            }),
            failure_reason: None,
            diff_summary: None,
        };
    }

    // 3. Mutate / Plan edits: Plan edits with TextProvider::plan (or provider operation).
    // On provider refusal, return Outcome::Refused with appropriate RefusalReason.
    let edits = match TextProvider::plan(&original_bytes, op, &plan.cardinality) {
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
    };

    // 4. Check NoChange: If candidate bytes equal original bytes, return Outcome::NoChange.
    // Apply byte edits to produce in-memory candidate bytes via apply_byte_edits.
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

    // 5. Verify: Compute candidate post_hash and bounded unified diff via generate_diff.
    let post_hash = compute_sha256(&candidate_bytes);
    let diff_summary = generate_diff(&original_bytes, &candidate_bytes);

    // 6. Commit or Dry-run: If not dry_run, commit candidate to disk atomically via Workspace::write_file_atomic.
    if !dry_run {
        if let Err(e) = workspace.write_file_atomic(&file_path, &candidate_bytes) {
            return match e {
                WorkspaceError::Traversal(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::WorkspaceTraversal { path: p }),
                    failure_reason: None,
                    diff_summary: Some(diff_summary),
                },
                WorkspaceError::SymlinkEscape(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::SymlinkEscape { path: p }),
                    failure_reason: None,
                    diff_summary: Some(diff_summary),
                },
                WorkspaceError::NotFound(p) => Certificate {
                    outcome: Outcome::Refused,
                    file_path,
                    pre_hash: actual_pre_hash,
                    post_hash: Some(post_hash),
                    refusal_reason: Some(RefusalReason::MissingTarget { target: p }),
                    failure_reason: None,
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

    // 7. Certify: Return Certificate with outcome APPLIED, pre_hash, post_hash, diff_summary.
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
    use crate::protocol::Cardinality;
    use tempfile::TempDir;

    #[test]
    fn test_pipeline_successful_apply() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let pre_hash = compute_sha256(b"Hello World\n");
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            expected_pre_hash: pre_hash.clone(),
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        let op = TextOperation::Replace {
            target: "World".to_string(),
            replacement: "Suture".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, false);
        assert_eq!(cert.outcome, Outcome::Applied);
        assert_eq!(cert.pre_hash, pre_hash);
        assert!(cert.post_hash.is_some());
        assert_ne!(cert.post_hash.as_ref().unwrap(), &pre_hash);
        let diff = cert.diff_summary.as_ref().unwrap();
        assert!(diff.contains("-Hello World"));
        assert!(diff.contains("+Hello Suture"));

        // Verify file updated on disk
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
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            expected_pre_hash: pre_hash.clone(),
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        let op = TextOperation::Replace {
            target: "World".to_string(),
            replacement: "Suture".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, true);
        assert_eq!(cert.outcome, Outcome::Applied);
        assert!(cert.post_hash.is_some());
        assert!(cert.diff_summary.is_some());

        // Verify file NOT touched on disk
        let disk_content = std::fs::read_to_string(tmp.path().join(file_path)).unwrap();
        assert_eq!(disk_content, "Hello World\n");
    }

    #[test]
    fn test_pipeline_stale_identity() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            expected_pre_hash: "stale_hash_value".to_string(),
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        let op = TextOperation::Replace {
            target: "World".to_string(),
            replacement: "Suture".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, false);
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
    fn test_pipeline_near_miss_and_duplicate_refusal() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"foo bar foo bar\n").unwrap();

        let pre_hash = compute_sha256(b"foo bar foo bar\n");
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            expected_pre_hash: pre_hash,
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        // "foo" appears twice, so ExactlyOne should refuse with DuplicateTarget
        let op = TextOperation::Replace {
            target: "foo".to_string(),
            replacement: "baz".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, false);
        assert_eq!(cert.outcome, Outcome::Refused);
        match cert.refusal_reason.unwrap() {
            RefusalReason::DuplicateTarget { target, count } => {
                assert_eq!(target, "foo");
                assert_eq!(count, 2);
            }
            other => panic!("Expected DuplicateTarget, got {:?}", other),
        }
    }

    #[test]
    fn test_pipeline_no_change() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let file_path = "test.txt";
        std::fs::write(tmp.path().join(file_path), b"Hello World\n").unwrap();

        let pre_hash = compute_sha256(b"Hello World\n");
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: file_path.to_string(),
            expected_pre_hash: pre_hash.clone(),
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        // Replacement is identical to target
        let op = TextOperation::Replace {
            target: "World".to_string(),
            replacement: "World".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, false);
        assert_eq!(cert.outcome, Outcome::NoChange);
        assert_eq!(cert.pre_hash, pre_hash);
        assert_eq!(cert.post_hash.unwrap(), pre_hash);
    }

    #[test]
    fn test_pipeline_workspace_traversal_refusal() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let plan = MutationPlan {
            version: "0.1.0".to_string(),
            file_path: "../outside.txt".to_string(),
            expected_pre_hash: "".to_string(),
            edits: vec![],
            cardinality: Cardinality::ExactlyOne,
        };
        let op = TextOperation::Replace {
            target: "a".to_string(),
            replacement: "b".to_string(),
        };

        let cert = execute_pipeline(&ws, &plan, &op, false);
        assert_eq!(cert.outcome, Outcome::Refused);
        match cert.refusal_reason.unwrap() {
            RefusalReason::WorkspaceTraversal { .. } => {}
            other => panic!("Expected WorkspaceTraversal, got {:?}", other),
        }
    }
}
