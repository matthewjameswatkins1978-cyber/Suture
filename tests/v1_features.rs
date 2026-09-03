use suture::engine::compute_sha256;
use suture::lifecycle::FileOperation;
use suture::pipeline::execute_request;
use suture::protocol::{
    Cardinality, EffectBudget, OperationPayload, Outcome, RegionGuard, Request, TransactionRequest,
    PROTOCOL_VERSION,
};
use suture::provider::code::CodeOperation;
use suture::provider::json::JsonOperation;
use suture::provider::jsonc::JsoncProvider;
use suture::provider::markdown::MarkdownOperation;
use suture::provider::patch::PatchOperation;
use suture::provider::text::{TextOperation, TextProvider};
use suture::provider::yaml::YamlOperation;
use suture::workspace::Workspace;
use tempfile::TempDir;

#[test]
fn canonical_examples_are_current_protocol_values() {
    for example in suture::metadata::examples(None) {
        if example.request.get("requests").is_some() {
            let transaction: TransactionRequest = serde_json::from_value(example.request).unwrap();
            assert_eq!(transaction.version, PROTOCOL_VERSION, "{}", example.topic);
        } else {
            let request: Request = serde_json::from_value(example.request).unwrap();
            assert_eq!(request.version, PROTOCOL_VERSION, "{}", example.topic);
        }
    }
}

#[test]
fn every_public_reason_code_is_explainable() {
    let manifest = suture::metadata::capabilities();
    for reason in manifest.reason_codes {
        assert_eq!(
            suture::metadata::reason(reason.code).unwrap().code,
            reason.code
        );
    }
}

#[test]
fn every_provider_operation_is_in_canonical_operation_metadata() {
    let operations: std::collections::HashSet<_> = suture::metadata::operation_metadata()
        .into_iter()
        .map(|operation| operation.name)
        .collect();
    for provider in suture::metadata::provider_metadata() {
        for operation in provider.operations {
            assert!(
                operations.contains(operation),
                "{}.{operation}",
                provider.name
            );
        }
    }
}

#[test]
fn suggestion_is_schema_valid_and_refusal_recovery_is_machine_readable() {
    let suggestion = suture::metadata::suggest(
        "config.json",
        Some("set-value"),
        Some("$.name"),
        "safe",
        None,
    );
    let suggested_request: Request =
        serde_json::from_value(suggestion.request_template.unwrap()).unwrap();
    assert_eq!(suggested_request.version, PROTOCOL_VERSION);
    let temp = TempDir::new().unwrap();
    let refusal = execute_request(
        &Workspace::new(temp.path()).unwrap(),
        &request(
            "missing.txt",
            OperationPayload::Text(TextOperation::Replace {
                target: "x".into(),
                replacement: "y".into(),
            }),
        ),
        true,
    );
    let recovery = suture::metadata::refusal_recovery(&refusal);
    assert_eq!(recovery["reason_code"], "TARGET_NOT_FOUND");
    assert!(recovery["suggestions"].as_array().unwrap().len() == 1);
}

fn request(path: &str, operation: OperationPayload) -> Request {
    Request {
        version: PROTOCOL_VERSION.into(),
        request_id: "test-v1".into(),
        allow_generated: false,
        file_path: path.into(),
        namespace: Default::default(),
        expected_pre_hash: None,
        region_guard: None,
        cardinality: Cardinality::ExactlyOne,
        budget: EffectBudget::default(),
        operation,
    }
}

#[test]
fn capabilities_are_versioned_and_advertise_budgets() {
    let caps = serde_json::to_value(suture::capabilities::current()).unwrap();
    assert_eq!(caps["protocol_versions"][0], PROTOCOL_VERSION);
    assert!(caps["providers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == "jsonc"));
    assert!(
        caps["resource_limits"]["max_request_bytes"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn effect_budget_refuses_before_write() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("x.txt"), b"one two\n").unwrap();
    let mut r = request(
        "x.txt",
        OperationPayload::Text(TextOperation::Replace {
            target: "one".into(),
            replacement: "a much longer replacement".into(),
        }),
    );
    r.budget.max_changed_bytes = Some(2);
    let c = execute_request(&w, &r, false);
    assert_eq!(c.outcome, Outcome::Refused);
    assert_eq!(c.reason_code.as_deref(), Some("EFFECT_BUDGET_EXCEEDED"));
    assert!(!c.effect.passed);
    assert_eq!(w.read_file("x.txt").unwrap(), b"one two\n");
}

#[test]
fn capability_and_schema_fingerprints_are_deterministic_and_scoped() {
    let first = suture::metadata::capabilities();
    let second = suture::metadata::capabilities();
    assert_eq!(first.capability_set_id, second.capability_set_id);
    let view = suture::metadata::capability_view(Some("json.set"));
    assert_eq!(view["selected_operation"]["name"], "set");
    let schema = suture::metadata::schema(Some("json"));
    assert_eq!(schema["protocol_version"], PROTOCOL_VERSION);
    assert!(schema["schema_id"].as_str().is_some());
}

#[test]
fn desired_state_replay_is_no_change() {
    let first = TextProvider::plan(
        b"a\n",
        &TextOperation::EnsurePresent {
            content: "b".into(),
        },
        &Cardinality::ExactlyOne,
    )
    .unwrap();
    let after = suture::engine::apply_byte_edits(b"a\n", &first).unwrap();
    let second = TextProvider::plan(
        &after,
        &TextOperation::EnsurePresent {
            content: "b".into(),
        },
        &Cardinality::ExactlyOne,
    )
    .unwrap();
    assert!(second.is_empty());
    let absent = TextProvider::plan(
        b"a",
        &TextOperation::EnsureAbsent {
            target: "missing".into(),
        },
        &Cardinality::ExactlyOne,
    )
    .unwrap();
    assert!(absent.is_empty());
}

#[test]
fn jsonc_comments_are_not_in_edit_ranges() {
    let original = br#"{
  // keep this comment
  "name": "old"
}
"#;
    let edits = JsoncProvider::plan(
        original,
        &JsonOperation::Set {
            path: "$.name".into(),
            value: serde_json::json!("new"),
        },
        &Cardinality::ExactlyOne,
    )
    .unwrap();
    let changed = suture::engine::apply_byte_edits(original, &edits).unwrap();
    assert!(String::from_utf8_lossy(&changed).contains("keep this comment"));
}

#[test]
fn markdown_yaml_and_lifecycle_are_guarded() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("doc.md"), b"# A\nold\n\n# B\nkeep\n").unwrap();
    let c = execute_request(
        &w,
        &request(
            "doc.md",
            OperationPayload::Markdown(MarkdownOperation::ReplaceSection {
                heading: "A".into(),
                content: "new".into(),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    std::fs::write(
        t.path().join("config.yaml"),
        b"name: old # comment\ncount: 1\n",
    )
    .unwrap();
    let c = execute_request(
        &w,
        &request(
            "config.yaml",
            OperationPayload::Yaml(YamlOperation::Set {
                path: "name".into(),
                value: serde_json::json!("new"),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    let content = b"created\n".to_vec();
    let c = execute_request(
        &w,
        &request(
            "new.txt",
            OperationPayload::File(FileOperation::CreateFile {
                expected_absent: true,
                content: content.clone(),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    let c = execute_request(
        &w,
        &request(
            "new.txt",
            OperationPayload::File(FileOperation::DeleteFile {
                expected_hash: compute_sha256(&content),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    assert!(!t.path().join("new.txt").exists());
}

#[test]
fn multi_file_transaction_prepares_then_commits_and_cleans_journal() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("a.txt"), b"old-a").unwrap();
    std::fs::write(t.path().join("b.txt"), b"old-b").unwrap();
    let tx = TransactionRequest {
        version: PROTOCOL_VERSION.into(),
        transaction_id: "tx-v1-test".into(),
        requests: vec![
            request(
                "a.txt",
                OperationPayload::Text(TextOperation::Replace {
                    target: "old-a".into(),
                    replacement: "new-a".into(),
                }),
            ),
            request(
                "b.txt",
                OperationPayload::Text(TextOperation::Replace {
                    target: "old-b".into(),
                    replacement: "new-b".into(),
                }),
            ),
        ],
        budget: EffectBudget {
            max_files: Some(2),
            ..Default::default()
        },
    };
    let c = suture::pipeline::execute_transaction(&w, &tx, false);
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(w.read_file("a.txt").unwrap(), b"new-a");
    assert_eq!(w.read_file("b.txt").unwrap(), b"new-b");
    assert!(!t.path().join(".suture-recovery/tx-v1-test.json").exists());
}

#[test]
fn code_provider_targets_validated_syntax_nodes() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("main.py"), b"value = 1\n").unwrap();
    let c = execute_request(
        &w,
        &request(
            "main.py",
            OperationPayload::Code(CodeOperation::ReplaceNode {
                language: "python".into(),
                target: "1".into(),
                replacement: "2".into(),
                node_kind: Some("integer".into()),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(w.read_file("main.py").unwrap(), b"value = 2\n");
}

#[test]
fn typescript_provider_uses_typescript_grammar() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("main.ts"), b"const value: number = 1;\n").unwrap();
    let c = execute_request(
        &w,
        &request(
            "main.ts",
            OperationPayload::Code(CodeOperation::ReplaceNode {
                language: "typescript".into(),
                target: "1".into(),
                replacement: "2".into(),
                node_kind: Some("number".into()),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(
        w.read_file("main.ts").unwrap(),
        b"const value: number = 2;\n"
    );
}

#[test]
fn generated_files_fail_closed_without_explicit_opt_in() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(
        t.path().join("generated.rs"),
        b"// Code generated by tool; DO NOT EDIT.\nfn old() {}\n",
    )
    .unwrap();
    let c = execute_request(
        &w,
        &request(
            "generated.rs",
            OperationPayload::Text(TextOperation::Replace {
                target: "old".into(),
                replacement: "new".into(),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Refused);
}

#[test]
fn strict_patch_requires_exact_context() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("x.txt"), b"one\ntwo\n").unwrap();
    let patch = "--- a/x.txt\n+++ b/x.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+TWO\n";
    let c = execute_request(
        &w,
        &request(
            "x.txt",
            OperationPayload::Patch(PatchOperation::UnifiedDiff {
                patch: patch.into(),
            }),
        ),
        false,
    );
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(w.read_file("x.txt").unwrap(), b"one\nTWO\n");
}

#[test]
fn durable_region_guard_survives_unrelated_edit() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("x.txt"), b"target\n").unwrap();
    let mut r = request(
        "x.txt",
        OperationPayload::Text(TextOperation::Replace {
            target: "target".into(),
            replacement: "changed".into(),
        }),
    );
    r.region_guard = Some(RegionGuard {
        anchor: "target".into(),
        target_sha256: compute_sha256(b"target"),
    });
    std::fs::write(t.path().join("x.txt"), b"unrelated\ntarget\n").unwrap();
    let c = execute_request(&w, &r, false);
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(w.read_file("x.txt").unwrap(), b"unrelated\nchanged\n");
}

#[test]
fn single_file_transaction_resolves_operations_on_coherent_candidate() {
    let t = TempDir::new().unwrap();
    let w = Workspace::new(t.path()).unwrap();
    std::fs::write(t.path().join("x.txt"), b"one two\n").unwrap();
    let tx = TransactionRequest {
        version: PROTOCOL_VERSION.into(),
        transaction_id: "tx-single-file".into(),
        requests: vec![
            request(
                "x.txt",
                OperationPayload::Text(TextOperation::Replace {
                    target: "one".into(),
                    replacement: "first".into(),
                }),
            ),
            request(
                "x.txt",
                OperationPayload::Text(TextOperation::Replace {
                    target: "two".into(),
                    replacement: "second".into(),
                }),
            ),
        ],
        budget: EffectBudget::default(),
    };
    let c = suture::pipeline::execute_transaction(&w, &tx, false);
    assert_eq!(c.outcome, Outcome::Applied);
    assert_eq!(w.read_file("x.txt").unwrap(), b"first second\n");
}
