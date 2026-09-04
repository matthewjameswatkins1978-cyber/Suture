# Protocol v1.1

A request is JSON with `version: "1.1.0"`, a stable `request_id`, a workspace-relative `file_path`, optional `namespace`, optional `expected_pre_hash`, a `cardinality`, an optional hard effect `budget`, and an operation. Run `threadmoth help`, `threadmoth examples`, `threadmoth schema`, `threadmoth explain`, `threadmoth suggest`, or `threadmoth capabilities` for local discovery. Unknown fields are rejected.

The operation is encoded as an outer provider and nested tagged operation:

```json
{"version":"1.1.0","request_id":"example-2","file_path":"config.json","cardinality":{"type":"exactly_one"},"budget":{"max_files":1,"max_matches":1,"max_changed_lines":4},"operation":{"provider":"json","operation":{"type":"set","path":"$.server.port","value":8080}}}
```

Text supports exact and idempotent desired-state operations. JSON/JSONC support source-preserving structured paths; TOML supports dotted paths; YAML, Markdown, dotenv, bounded pattern, strict unified diff, filesystem lifecycle, and Tree-sitter code providers are separate explicit providers. Structured paths require `exactly_one`; broad operations must state their cardinality.

Outcomes are `APPLIED`, `NO_CHANGE`, `REFUSED`, and `FAILED`. Refusals include stable reasons such as `stale_identity`, `cardinality_mismatch`, `cardinality_ambiguous`, `unsupported_encoding`, `malformed_input`, `workspace_traversal`, `symlink_escape`, `preservation_unavailable`, and `unsupported_protocol_version`.

An applied certificate includes protocol/provider identity, request ID, expected and observed cardinality, pre/post SHA-256, changed byte/line ranges, bounded diff, structural validation, preservation facts, effect-budget usage, commit guarantee, and recovery state. Diff output is capped at 4096 characters. Transaction certificates contain one certificate per member and one rollback/recovery state.

Every refusal and relevant failure certificate includes a stable `reason_code`; use `threadmoth explain REASON_CODE` for local recovery guidance or `threadmoth suggest --from-refusal CERTIFICATE` for deterministic next request skeletons. Exit codes: `0` applied/no-change, `2` refused, `3` runtime failure.
