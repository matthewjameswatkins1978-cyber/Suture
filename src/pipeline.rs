#![forbid(unsafe_code)]

use crate::engine::{apply_byte_edits, compute_sha256, ByteEdit};
use crate::lifecycle::FileOperation;
use crate::path::PathNormalizer;
use crate::pattern::{self, PatternError};
use crate::protocol::{
    ByteRange, Certificate, CommitGuarantee, EffectBudget, EffectUsage, FailureReason,
    MutationPlan, OperationPayload, Outcome, PreservationFacts, RefusalReason, Request,
    StructuralValidation, TransactionCertificate, TransactionRequest, PROTOCOL_VERSION,
};
use crate::provider::code::{self, CodeError, CodeOperation};
use crate::provider::dotenv::{self, DotenvError};
use crate::provider::json::{JsonProvider, JsonProviderError};
use crate::provider::jsonc::JsoncProvider;
use crate::provider::markdown::{self, MarkdownError};
use crate::provider::patch::{self, PatchError};
use crate::provider::text::{TextOperation, TextProvider, TextProviderError};
use crate::provider::toml::{TomlProvider, TomlProviderError};
use crate::provider::yaml::{self, YamlError};
use crate::recovery::{self, Journal, JournalEntry};
use crate::workspace::{Workspace, WorkspaceError};

pub fn execute_pipeline(
    workspace: &Workspace,
    plan: &MutationPlan,
    op: &TextOperation,
    dry_run: bool,
) -> Certificate {
    let request = Request {
        version: plan.version.clone(),
        request_id: String::new(),
        allow_generated: false,
        file_path: plan.file_path.clone(),
        namespace: Default::default(),
        expected_pre_hash: (!plan.expected_pre_hash.is_empty())
            .then(|| plan.expected_pre_hash.clone()),
        region_guard: None,
        cardinality: plan.cardinality.clone(),
        budget: EffectBudget::default(),
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
    if !request.budget.allowed_path_prefixes.is_empty()
        && !request
            .budget
            .allowed_path_prefixes
            .iter()
            .any(|prefix| file_path == *prefix || file_path.starts_with(&format!("{prefix}/")))
    {
        return refusal(
            request,
            &file_path,
            provider,
            RefusalReason::WorkspaceTraversal {
                path: "path is outside requested budget scope".into(),
            },
            String::new(),
        );
    }
    if let OperationPayload::File(operation) = &request.operation {
        return execute_file_operation(workspace, request, &file_path, operation, dry_run);
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
    if let Some(guard) = &request.region_guard {
        let matches: Vec<_> = original
            .windows(guard.anchor.len())
            .enumerate()
            .filter(|(_, bytes)| *bytes == guard.anchor.as_bytes())
            .map(|(offset, _)| offset)
            .collect();
        if matches.len() != 1 {
            return refusal(
                request,
                &file_path,
                provider,
                RefusalReason::CardinalityMismatch {
                    expected: "one durable region anchor".into(),
                    actual: matches.len(),
                },
                pre_hash,
            );
        }
        let actual = compute_sha256(guard.anchor.as_bytes());
        if actual != normalize_hash(&guard.target_sha256) {
            return refusal(
                request,
                &file_path,
                provider,
                RefusalReason::StaleIdentity {
                    expected_hash: normalize_hash(&guard.target_sha256),
                    actual_hash: actual,
                },
                pre_hash,
            );
        }
    }
    if let Some(reason) = unsupported_encoding(&original) {
        return refusal(request, &file_path, provider, reason, pre_hash);
    }
    if original.contains(&0) {
        return refusal(
            request,
            &file_path,
            provider,
            RefusalReason::BinaryInput,
            pre_hash,
        );
    }
    if is_generated_file(&original) && !request.allow_generated {
        return refusal(
            request,
            &file_path,
            provider,
            RefusalReason::GeneratedFileRequiresOptIn {
                marker: generated_marker(&original).into(),
            },
            pre_hash,
        );
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
    let line_ranges = changed_line_ranges(&original, &engine_edits);
    let effect = effect_usage(&original, &candidate, &engine_edits, &request.budget);
    if let Some((dimension, limit, actual)) = budget_violation(&effect, &request.budget) {
        return refusal_with_effect(
            request,
            &file_path,
            provider,
            RefusalReason::EffectBudgetExceeded {
                dimension,
                limit,
                actual,
            },
            pre_hash,
            EffectUsage {
                passed: false,
                ..effect
            },
        );
    }
    let provider_structural = match &request.operation {
        OperationPayload::Json(_) | OperationPayload::Jsonc(_) => {
            let result = if matches!(&request.operation, OperationPayload::Jsonc(_)) {
                JsoncProvider::validate(&candidate)
            } else {
                JsonProvider::validate(&candidate)
            };
            if let Err(e) = result {
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
        OperationPayload::Text(_) | OperationPayload::Pattern(_) => {
            StructuralValidation::NotApplicable
        }
        OperationPayload::Markdown(_) => StructuralValidation::Valid {
            format: "markdown_regions".into(),
        },
        OperationPayload::Yaml(_) => {
            if let Err(e) = yaml::validate(&candidate) {
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
                format: "yaml_conservative_source".into(),
            }
        }
        OperationPayload::File(_) => StructuralValidation::NotApplicable,
        OperationPayload::Code(operation) => {
            let language = match operation {
                CodeOperation::ReplaceNode { language, .. }
                | CodeOperation::InsertBeforeNode { language, .. }
                | CodeOperation::InsertAfterNode { language, .. }
                | CodeOperation::RemoveNode { language, .. } => language,
            };
            if let Err(e) = code::validate(&candidate, language) {
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
                format: format!("tree_sitter:{language}"),
            }
        }
        OperationPayload::Dotenv(_) => StructuralValidation::Valid {
            format: "dotenv_lines".into(),
        },
        OperationPayload::Patch(_) => StructuralValidation::NotApplicable,
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
            effect,
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
    let mut certificate = completed(
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
        effect,
    );
    certificate.changed_line_ranges = line_ranges;
    certificate
}

/// Prepare every member against its accepted source, then commit the complete
/// set behind one durable recovery journal. No member is written during the
/// preparation pass.
pub fn execute_transaction(
    workspace: &Workspace,
    transaction: &TransactionRequest,
    dry_run: bool,
) -> TransactionCertificate {
    if transaction.version != PROTOCOL_VERSION {
        return transaction_refusal(
            transaction,
            RefusalReason::UnsupportedProtocolVersion {
                requested: transaction.version.clone(),
                supported: PROTOCOL_VERSION.into(),
            },
        );
    }
    if transaction.requests.is_empty() {
        return transaction_refusal(
            transaction,
            RefusalReason::Custom {
                message: "transaction must contain at least one request".into(),
            },
        );
    }
    let mut paths = std::collections::HashSet::new();
    for request in &transaction.requests {
        let path = PathNormalizer::normalize(&request.file_path, &request.namespace);
        if !paths.insert(path.clone()) {
            return transaction_refusal(
                transaction,
                RefusalReason::Custom { message: format!("multiple operations for {path} require a single-file operation batch; refusing ambiguous transaction ordering") },
            );
        }
    }
    let mut prepared = Vec::new();
    let mut aggregate = EffectUsage {
        files: 0,
        matches: 0,
        changed_regions: 0,
        changed_lines: 0,
        changed_bytes: 0,
        passed: true,
    };
    for request in &transaction.requests {
        if matches!(request.operation, OperationPayload::File(_)) {
            return transaction_refusal(transaction, RefusalReason::UnsupportedOperation { operation: "filesystem lifecycle operations are not yet composable in multi-file transactions".into() });
        }
        let certificate = execute_request(workspace, request, true);
        if certificate.outcome == Outcome::Refused {
            return TransactionCertificate {
                protocol_version: transaction.version.clone(),
                transaction_id: transaction.transaction_id.clone(),
                outcome: Outcome::Refused,
                certificates: vec![certificate.clone()],
                rollback_state: "not_started".into(),
                transaction_guarantee: "not_committed".into(),
                refusal_reason: certificate.refusal_reason,
                failure_reason: certificate.failure_reason,
            };
        }
        if certificate.outcome == Outcome::Failed {
            return TransactionCertificate {
                protocol_version: transaction.version.clone(),
                transaction_id: transaction.transaction_id.clone(),
                outcome: Outcome::Failed,
                certificates: vec![certificate.clone()],
                rollback_state: "not_started".into(),
                transaction_guarantee: "not_committed".into(),
                refusal_reason: None,
                failure_reason: certificate.failure_reason,
            };
        }
        let path = PathNormalizer::normalize(&request.file_path, &request.namespace);
        let original = match workspace.read_file(&path) {
            Ok(x) => x,
            Err(e) => {
                return transaction_failure(
                    transaction,
                    FailureReason::IoError {
                        message: e.to_string(),
                    },
                )
            }
        };
        let edits = match plan_edits(&original, request) {
            Ok(x) => x,
            Err((reason, _)) => return transaction_refusal(transaction, reason),
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
                return transaction_failure(
                    transaction,
                    FailureReason::InternalInvariant {
                        details: e.to_string(),
                    },
                )
            }
        };
        aggregate.files += usize::from(original != candidate);
        aggregate.matches += certificate.effect.matches;
        aggregate.changed_regions += certificate.effect.changed_regions;
        aggregate.changed_lines += certificate.effect.changed_lines;
        aggregate.changed_bytes += certificate.effect.changed_bytes;
        prepared.push((path, original, candidate));
    }
    if let Some((dimension, limit, actual)) = budget_violation(&aggregate, &transaction.budget) {
        return transaction_refusal(
            transaction,
            RefusalReason::EffectBudgetExceeded {
                dimension,
                limit,
                actual,
            },
        );
    }
    let certificates: Vec<Certificate> = transaction
        .requests
        .iter()
        .map(|request| execute_request(workspace, request, true))
        .collect();
    if dry_run {
        return TransactionCertificate {
            protocol_version: transaction.version.clone(),
            transaction_id: transaction.transaction_id.clone(),
            outcome: if aggregate.files == 0 {
                Outcome::NoChange
            } else {
                Outcome::Applied
            },
            certificates,
            rollback_state: "not_required".into(),
            transaction_guarantee: "dry_run".into(),
            refusal_reason: None,
            failure_reason: None,
        };
    }
    let journal = Journal {
        protocol_version: transaction.version.clone(),
        transaction_id: transaction.transaction_id.clone(),
        entries: prepared
            .iter()
            .map(|(path, original, candidate)| JournalEntry {
                path: path.clone(),
                pre_hash: compute_sha256(original),
                candidate_hash: compute_sha256(candidate),
                original: original.clone(),
                candidate: candidate.clone(),
            })
            .collect(),
    };
    if let Err(e) = recovery::write_journal(workspace, &journal) {
        return transaction_failure(
            transaction,
            FailureReason::CommitFailure {
                message: e.to_string(),
            },
        );
    }
    let mut committed = Vec::new();
    for entry in &journal.entries {
        match workspace.write_file_atomic_checked(&entry.path, &entry.pre_hash, &entry.candidate) {
            Ok(()) => committed.push(entry),
            Err(error) => {
                let mut rollback_ok = true;
                for prior in committed.iter().rev() {
                    if workspace
                        .write_file_atomic_checked(
                            &prior.path,
                            &prior.candidate_hash,
                            &prior.original,
                        )
                        .is_err()
                    {
                        rollback_ok = false;
                    }
                }
                return TransactionCertificate {
                    protocol_version: transaction.version.clone(),
                    transaction_id: transaction.transaction_id.clone(),
                    outcome: Outcome::Failed,
                    certificates,
                    rollback_state: if rollback_ok {
                        "rolled_back".into()
                    } else {
                        "manual_recovery_required".into()
                    },
                    transaction_guarantee: "transactional_with_rollback".into(),
                    refusal_reason: None,
                    failure_reason: Some(FailureReason::CommitFailure {
                        message: error.to_string(),
                    }),
                };
            }
        }
    }
    let _ = recovery::remove_journal(workspace, &transaction.transaction_id);
    TransactionCertificate {
        protocol_version: transaction.version.clone(),
        transaction_id: transaction.transaction_id.clone(),
        outcome: if aggregate.files == 0 {
            Outcome::NoChange
        } else {
            Outcome::Applied
        },
        certificates,
        rollback_state: "not_required".into(),
        transaction_guarantee: "transactional_with_rollback".into(),
        refusal_reason: None,
        failure_reason: None,
    }
}

fn transaction_refusal(
    transaction: &TransactionRequest,
    reason: RefusalReason,
) -> TransactionCertificate {
    TransactionCertificate {
        protocol_version: transaction.version.clone(),
        transaction_id: transaction.transaction_id.clone(),
        outcome: Outcome::Refused,
        certificates: Vec::new(),
        rollback_state: "not_started".into(),
        transaction_guarantee: "not_committed".into(),
        refusal_reason: Some(reason),
        failure_reason: None,
    }
}
fn transaction_failure(
    transaction: &TransactionRequest,
    reason: FailureReason,
) -> TransactionCertificate {
    TransactionCertificate {
        protocol_version: transaction.version.clone(),
        transaction_id: transaction.transaction_id.clone(),
        outcome: Outcome::Failed,
        certificates: Vec::new(),
        rollback_state: "not_started".into(),
        transaction_guarantee: "not_committed".into(),
        refusal_reason: None,
        failure_reason: Some(reason),
    }
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
        OperationPayload::Jsonc(o) => JsoncProvider::plan(original, o, &request.cardinality)
            .map_err(|e| {
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
            }),
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
        OperationPayload::Pattern(o) => {
            pattern::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    PatternError::Refused(r) => (r, detail),
                    PatternError::Error { message } => (
                        RefusalReason::Custom {
                            message: message.clone(),
                        },
                        message,
                    ),
                }
            })
        }
        OperationPayload::Markdown(o) => {
            markdown::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    MarkdownError::Refused(r) => (r, detail),
                }
            })
        }
        OperationPayload::Yaml(o) => yaml::plan(original, o, &request.cardinality).map_err(|e| {
            let detail = e.to_string();
            match e {
                YamlError::Refused(r) => (r, detail),
            }
        }),
        OperationPayload::File(_) => Err((
            RefusalReason::UnsupportedOperation {
                operation: "filesystem lifecycle is handled before content planning".into(),
            },
            "unreachable lifecycle operation".into(),
        )),
        OperationPayload::Code(o) => code::plan(original, o, &request.cardinality).map_err(|e| {
            let detail = e.to_string();
            match e {
                CodeError::Refused(r) => (r, detail),
            }
        }),
        OperationPayload::Dotenv(o) => {
            dotenv::plan(original, o, &request.cardinality).map_err(|e| {
                let detail = e.to_string();
                match e {
                    DotenvError::Refused(r) => (r, detail),
                }
            })
        }
        OperationPayload::Patch(o) => patch::plan(original, o, &request.cardinality).map_err(|e| {
            let detail = e.to_string();
            match e {
                PatchError::Refused(r) => (r, detail),
            }
        }),
    }
}

fn execute_file_operation(
    workspace: &Workspace,
    request: &Request,
    file_path: &str,
    operation: &FileOperation,
    dry_run: bool,
) -> Certificate {
    let provider = "filesystem";
    let source = workspace.resolve_path(file_path);
    let (pre_hash, post_hash, effect) = match operation {
        FileOperation::CreateFile {
            expected_absent,
            content,
        } => {
            if !expected_absent {
                return refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::Custom {
                        message: "create_file requires expected_absent=true".into(),
                    },
                    String::new(),
                );
            }
            match source {
                Ok(path) if path.exists() => {
                    return refusal(
                        request,
                        file_path,
                        provider,
                        RefusalReason::DestinationExists {
                            path: file_path.into(),
                        },
                        String::new(),
                    )
                }
                Err(e) => return workspace_error(request, file_path, provider, e),
                _ => {}
            }
            let effect = EffectUsage {
                files: 1,
                matches: 1,
                changed_regions: 1,
                changed_lines: content.split(|b| *b == b'\n').count(),
                changed_bytes: content.len(),
                passed: true,
            };
            (String::new(), Some(compute_sha256(content)), effect)
        }
        FileOperation::DeleteFile { expected_hash } => {
            let original = match workspace.read_file(file_path) {
                Ok(x) => x,
                Err(e) => return workspace_error(request, file_path, provider, e),
            };
            let pre = compute_sha256(&original);
            if normalize_hash(expected_hash) != pre {
                return refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::StaleIdentity {
                        expected_hash: normalize_hash(expected_hash),
                        actual_hash: pre.clone(),
                    },
                    pre,
                );
            }
            (
                pre,
                None,
                EffectUsage {
                    files: 1,
                    matches: 1,
                    changed_regions: 1,
                    changed_lines: original.split(|b| *b == b'\n').count(),
                    changed_bytes: original.len(),
                    passed: true,
                },
            )
        }
        FileOperation::RenameFile {
            destination,
            expected_source_hash,
            destination_absent,
        }
        | FileOperation::MoveFile {
            destination,
            expected_source_hash,
            destination_absent,
        } => {
            let original = match workspace.read_file(file_path) {
                Ok(x) => x,
                Err(e) => return workspace_error(request, file_path, provider, e),
            };
            let pre = compute_sha256(&original);
            if normalize_hash(expected_source_hash) != pre {
                return refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::StaleIdentity {
                        expected_hash: normalize_hash(expected_source_hash),
                        actual_hash: pre.clone(),
                    },
                    pre,
                );
            }
            let dest = match workspace
                .resolve_path(PathNormalizer::normalize(destination, &request.namespace))
            {
                Ok(x) => x,
                Err(e) => return workspace_error(request, file_path, provider, e),
            };
            if *destination_absent && dest.exists() {
                return refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::DestinationExists {
                        path: destination.clone(),
                    },
                    pre,
                );
            }
            (
                pre,
                Some(compute_sha256(&original)),
                EffectUsage {
                    files: 1,
                    matches: 1,
                    changed_regions: 1,
                    changed_lines: 0,
                    changed_bytes: 0,
                    passed: true,
                },
            )
        }
    };
    if let Some((dimension, limit, actual)) = budget_violation(&effect, &request.budget) {
        return refusal_with_effect(
            request,
            file_path,
            provider,
            RefusalReason::EffectBudgetExceeded {
                dimension,
                limit,
                actual,
            },
            pre_hash,
            EffectUsage {
                passed: false,
                ..effect
            },
        );
    }
    if !dry_run {
        let result = match operation {
            FileOperation::CreateFile { content, .. } => {
                workspace.create_file_new(file_path, content)
            }
            FileOperation::DeleteFile { expected_hash } => {
                workspace.delete_file_checked(file_path, &normalize_hash(expected_hash))
            }
            FileOperation::RenameFile {
                destination,
                expected_source_hash,
                destination_absent,
            }
            | FileOperation::MoveFile {
                destination,
                expected_source_hash,
                destination_absent,
            } => workspace.rename_file_checked(
                file_path,
                PathNormalizer::normalize(destination, &request.namespace),
                &normalize_hash(expected_source_hash),
                *destination_absent,
            ),
        };
        if let Err(e) = result {
            return match e {
                WorkspaceError::AlreadyExists(path) => refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::DestinationExists { path },
                    pre_hash,
                ),
                WorkspaceError::StaleIdentity { expected, actual } => refusal(
                    request,
                    file_path,
                    provider,
                    RefusalReason::StaleIdentity {
                        expected_hash: expected,
                        actual_hash: actual,
                    },
                    pre_hash,
                ),
                other => failure(
                    request,
                    file_path,
                    provider,
                    pre_hash,
                    FailureReason::CommitFailure {
                        message: other.to_string(),
                    },
                ),
            };
        }
    }
    let commit = if dry_run {
        CommitGuarantee {
            mode: "dry_run".into(),
            ..CommitGuarantee::default()
        }
    } else {
        CommitGuarantee {
            mode: "committed_checked_lifecycle".into(),
            content_replacement: "checked filesystem operation".into(),
            ..CommitGuarantee::default()
        }
    };
    completed(
        request,
        file_path,
        provider,
        pre_hash,
        post_hash,
        Outcome::Applied,
        Vec::new(),
        StructuralValidation::NotApplicable,
        PreservationFacts::default(),
        commit,
        String::new(),
        false,
        effect,
    )
}

fn normalize_hash(value: &str) -> String {
    value.strip_prefix("sha256:").unwrap_or(value).into()
}
fn provider_name(op: &OperationPayload) -> &'static str {
    match op {
        OperationPayload::Text(_) => "text",
        OperationPayload::Json(_) => "json",
        OperationPayload::Jsonc(_) => "jsonc",
        OperationPayload::Toml(_) => "toml",
        OperationPayload::Pattern(_) => "pattern",
        OperationPayload::Markdown(_) => "markdown",
        OperationPayload::Yaml(_) => "yaml",
        OperationPayload::File(_) => "filesystem",
        OperationPayload::Code(_) => "code",
        OperationPayload::Dotenv(_) => "dotenv",
        OperationPayload::Patch(_) => "patch",
    }
}
fn provider_version(op: &OperationPayload) -> &'static str {
    match op {
        OperationPayload::Text(_) => "text-byte-v1",
        OperationPayload::Json(_) => "json-source-v1",
        OperationPayload::Jsonc(_) => "jsonc-source-v1",
        OperationPayload::Toml(_) => "toml-edit-narrow-v1",
        OperationPayload::Pattern(_) => "regex-automata-bounded-v1",
        OperationPayload::Markdown(_) => "markdown-regions-v1",
        OperationPayload::Yaml(_) => "yaml-conservative-source-v1",
        OperationPayload::File(_) => "lifecycle-checked-v1",
        OperationPayload::Code(_) => "tree-sitter-node-v1",
        OperationPayload::Dotenv(_) => "dotenv-lines-v1",
        OperationPayload::Patch(_) => "unified-diff-strict-v1",
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

fn generated_marker(bytes: &[u8]) -> &'static str {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_ascii_lowercase();
    if sample.contains("code generated") {
        "code generated"
    } else if sample.contains("machine-generated") {
        "machine-generated"
    } else if sample.contains("do not edit") {
        "do not edit"
    } else {
        "generated file"
    }
}

fn is_generated_file(bytes: &[u8]) -> bool {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_ascii_lowercase();
    [
        "code generated",
        "machine-generated",
        "do not edit",
        "generated file",
    ]
    .iter()
    .any(|marker| sample.contains(marker))
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

fn changed_line_ranges(original: &[u8], edits: &[ByteEdit]) -> Vec<ByteRange> {
    edits
        .iter()
        .map(|edit| {
            let start = original[..edit.start.min(original.len())]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                + 1;
            let end = original[..edit.end.min(original.len())]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                + 1;
            ByteRange { start, end }
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
        zero_effect(),
    )
    .with_reason(reason)
}

fn refusal_with_effect(
    r: &Request,
    path: &str,
    provider: &str,
    reason: RefusalReason,
    pre: String,
    effect: EffectUsage,
) -> Certificate {
    let mut certificate = completed(
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
        effect,
    );
    certificate.refusal_reason = Some(reason);
    certificate
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
        zero_effect(),
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
        WorkspaceError::AlreadyExists(path) => refusal(
            r,
            &path.clone(),
            provider,
            RefusalReason::DestinationExists { path },
            String::new(),
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
    effect: EffectUsage,
) -> Certificate {
    let transaction_guarantee = commit.mode.clone();
    Certificate {
        protocol_version: r.version.clone(),
        request_id: request_id(r),
        outcome,
        file_path: path.into(),
        provider: provider.into(),
        provider_version: provider_version(&r.operation).into(),
        expected_cardinality: r.cardinality.clone(),
        observed_cardinality: Some(1),
        pre_hash: pre,
        post_hash: post,
        changed_ranges: ranges,
        changed_line_ranges: Vec::new(),
        diff_summary: Some(diff),
        diff_truncated: truncated,
        structural_validation: structural,
        preservation,
        commit,
        refusal_reason: None,
        failure_reason: None,
        diagnostics: Vec::new(),
        budget: r.budget.clone(),
        effect,
        transaction_guarantee,
        recovery_state: "not_required".into(),
    }
}

fn zero_effect() -> EffectUsage {
    EffectUsage {
        files: 0,
        matches: 0,
        changed_regions: 0,
        changed_lines: 0,
        changed_bytes: 0,
        passed: true,
    }
}

fn request_id(r: &Request) -> String {
    if r.request_id.is_empty() {
        format!("suture-{}", &compute_sha256(r.file_path.as_bytes())[..16])
    } else {
        r.request_id.clone()
    }
}

fn effect_usage(
    original: &[u8],
    candidate: &[u8],
    edits: &[ByteEdit],
    _budget: &EffectBudget,
) -> EffectUsage {
    let changed_bytes = edits
        .iter()
        .map(|edit| {
            edit.end
                .saturating_sub(edit.start)
                .max(edit.replacement.len())
        })
        .sum();
    let changed_lines = changed_line_count(original, candidate);
    EffectUsage {
        files: usize::from(original != candidate),
        matches: edits.len(),
        changed_regions: edits.len(),
        changed_lines,
        changed_bytes,
        passed: true,
    }
}

fn budget_violation(effect: &EffectUsage, budget: &EffectBudget) -> Option<(String, usize, usize)> {
    [
        ("max_files", budget.max_files, effect.files),
        ("max_matches", budget.max_matches, effect.matches),
        (
            "max_changed_regions",
            budget.max_changed_regions,
            effect.changed_regions,
        ),
        (
            "max_changed_lines",
            budget.max_changed_lines,
            effect.changed_lines,
        ),
        (
            "max_changed_bytes",
            budget.max_changed_bytes,
            effect.changed_bytes,
        ),
    ]
    .into_iter()
    .find_map(|(name, limit, actual)| {
        limit
            .filter(|limit| actual > *limit)
            .map(|limit| (name.into(), limit, actual))
    })
}

fn changed_line_count(original: &[u8], candidate: &[u8]) -> usize {
    let old = String::from_utf8_lossy(original);
    let new = String::from_utf8_lossy(candidate);
    similar::TextDiff::from_lines(&old, &new)
        .ops()
        .iter()
        .map(|op| match op {
            similar::DiffOp::Delete { old_len, .. } => *old_len,
            similar::DiffOp::Insert { new_len, .. } => *new_len,
            similar::DiffOp::Replace {
                old_len, new_len, ..
            } => (*old_len).max(*new_len),
            similar::DiffOp::Equal { .. } => 0,
        })
        .sum()
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
            original_newline_profile: newline_profile(a),
            result_newline_profile: newline_profile(b),
        }
    }
}

fn newline_profile(bytes: &[u8]) -> String {
    let crlf = bytes.windows(2).filter(|w| w == b"\r\n").count();
    let lf = bytes
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        .saturating_sub(crlf);
    let bare_cr = bytes
        .iter()
        .filter(|b| **b == b'\r')
        .count()
        .saturating_sub(crlf);
    match (crlf > 0, lf > 0, bare_cr > 0) {
        (false, false, false) => "none",
        (true, false, false) => "crlf",
        (false, true, false) => "lf",
        (true, true, false) => "mixed",
        _ => "mixed_with_bare_cr",
    }
    .into()
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
            request_id: String::new(),
            allow_generated: false,
            file_path: "x.txt".into(),
            namespace: Default::default(),
            expected_pre_hash: None,
            region_guard: None,
            cardinality: Cardinality::ExactlyOne,
            budget: Default::default(),
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
            request_id: String::new(),
            allow_generated: false,
            file_path: "x.txt".into(),
            namespace: Default::default(),
            expected_pre_hash: None,
            region_guard: None,
            cardinality: Cardinality::ExactlyOne,
            budget: Default::default(),
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
