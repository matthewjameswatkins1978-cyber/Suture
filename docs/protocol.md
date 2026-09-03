# Protocol v0.1

A request is JSON with `version: "0.1.0"`, a workspace-relative `file_path`, optional `namespace`, optional `expected_pre_hash`, a `cardinality`, and an operation. Run `suture schema` for the generated schema. Unknown fields are rejected.

The operation is encoded as an outer provider and nested tagged operation:

```json
{"version":"0.1.0","file_path":"config.json","expected_pre_hash":null,"cardinality":{"type":"exactly_one"},"operation":{"provider":"json","operation":{"type":"set","path":"$.server.port","value":8080}}}
```

Text supports `replace`, `insert_before`, `insert_after`, and `delete`, with exact UTF-8 byte matching. JSON supports `set`, `insert`, `delete`, and `rename_key` on `$`/dot/bracket paths. TOML supports the corresponding four operations on dotted paths. Structured paths require `exactly_one`; `All` is meaningful only for repeated text matches.

Outcomes are `APPLIED`, `NO_CHANGE`, `REFUSED`, and `FAILED`. Refusals include stable reasons such as `stale_identity`, `cardinality_mismatch`, `cardinality_ambiguous`, `unsupported_encoding`, `malformed_input`, `workspace_traversal`, `symlink_escape`, `preservation_unavailable`, and `unsupported_protocol_version`.

An applied certificate includes protocol/provider identity, expected and observed cardinality, pre/post SHA-256, changed ranges, a bounded diff, structural validation, preservation facts, and commit guarantees. Diff output is capped at 4096 characters.

Exit codes: `0` applied/no-change, `2` refused, `3` runtime failure.
