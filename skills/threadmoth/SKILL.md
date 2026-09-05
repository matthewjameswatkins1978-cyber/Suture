---
name: threadmoth
description: Use Threadmoth when an AI coding task needs a precise, bounded mutation of an existing JSON, JSONC, TOML, YAML, Markdown, dotenv, pattern, source-code, patch, or text file; preserve unrelated bytes; target a named section or syntax node; handle repeated or ambiguous matches; guard stale files; enforce effect budgets; or produce a machine-readable certificate. Prefer it when an unconstrained replacement could cause collateral edits. Do not use it for read-only inspection, Git, builds, tests, formatting-only rewrites, creating new files, or unsupported file shapes.
license: MIT
compatibility: >-
  Requires the Threadmoth executable on PATH (canonical command: threadmoth).
  Works with any agent that can read files and run local commands.
metadata:
  author: matthewjameswatkins1978-cyber
  version: "1.5.1"
---

# Threadmoth

Threadmoth is a narrow mutation boundary for existing files. It observes the
file, identifies the intended target, guards the request, stages the candidate,
verifies the committed bytes, and returns a certificate. It is not a general
shell, formatter, compiler, test runner, Git client, or network tool.

## Decide whether to use it

Use Threadmoth when the task changes an existing supported file and the exact
scope matters. It is especially useful when:

- a named section, JSON path, syntax node, or exact text occurrence is the target;
- repeated matches must be refused instead of guessed;
- an expected pre-image, path boundary, or effect budget should be enforced;
- the caller needs proof of the observed and committed bytes; or
- preserving encoding, line endings, comments, and unrelated bytes matters.

Do not route a task through Threadmoth merely because it can write a file. Use
the specialist tool when the task is formatting, compilation, testing, Git,
bulk generation, or a new file. If the file shape or requested operation is not
covered by its discovered capabilities, say so and use an appropriate fallback
only when the user authorizes it or the task plainly requires it.

## Discover locally

Before relying on a capability, query the installed runtime:

```text
threadmoth --version
threadmoth doctor
threadmoth capabilities
threadmoth suggest PATH
threadmoth schema
threadmoth examples
```

Use the canonical `threadmoth` executable. `thm` may exist as a convenience
alias, but it is not the compatibility contract. Do not assume a `.thm` source
extension.

## Make a bounded request

Construct a request with a workspace-relative `file_path`, stable `request_id`,
the required protocol version, an explicit operation/provider, and a
cardinality. Add `expected_pre_hash`, a narrow region guard, and a hard effect
`budget` when they are known. Prefer an exact path or named target over a broad
replacement. Read the local schema and examples instead of inventing fields.

For an ordinary text replacement, the shape is:

```json
{
  "version": "1.1.0",
  "request_id": "change-unique-id",
  "file_path": "config.txt",
  "cardinality": { "type": "exactly_one" },
  "operation": {
    "provider": "text",
    "operation": { "type": "replace", "target": "old", "replacement": "new" }
  }
}
```

Use the provider and operation forms returned by `threadmoth schema` for other
file types. Keep requests workspace-relative and never smuggle shell commands,
network work, or an unrelated file operation into the mutation request.

## Preview, then mutate

When the target, cardinality, effect, or preservation result is not already
obvious, preview first:

```text
threadmoth preview --request request.json
threadmoth mutate --request request.json
```

The default output is JSON. `--summary` is for a compact human view; keep the
full JSON certificate for agent state, audit, and follow-up decisions. A
transaction can be previewed with `threadmoth transact --request transaction.json --preview`.

Inspect the preview before mutating. Confirm the outcome, provider, path,
cardinality, changed ranges, effect-budget usage, preservation facts, and any
pre-image identity. Do not treat a successful process exit alone as proof that
the intended bytes landed.

## Handle outcomes deliberately

- `APPLIED` means the guarded mutation was committed and certified.
- `NO_CHANGE` means the requested result already held; keep the certificate.
- `REFUSED` means Threadmoth found ambiguity, stale identity, unsupported input,
  a path or safety violation, invalid data, or an effect outside the budget.
- `FAILED` means a runtime or commit failure; preserve the failure evidence and
  inspect recovery state when the response says recovery is relevant.

Exit codes are stable: `0` for applied/no-change, `2` for refusal, and `3` for
runtime failure. On refusal, read the reason code, candidate context, and
suggested narrowing. Use `threadmoth explain REASON_CODE` and, where provided,
`threadmoth suggest --from-refusal CERTIFICATE`. Narrow the request, ask the
user to disambiguate, or stop. Never silently widen the edit or bypass a
refusal with a raw write.

For transaction failures, use the recovery discovery commands rather than
deleting evidence:

```text
threadmoth recover --list
threadmoth recover --inspect TRANSACTION_ID
threadmoth recover --transaction TRANSACTION_ID
```

Report the certificate or refusal reason in the task result. If a fallback was
necessary, state why Threadmoth did not cover the operation and what broader
tool was used.
