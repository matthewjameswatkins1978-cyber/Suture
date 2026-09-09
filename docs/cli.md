# Threadmoth CLI

Threadmoth 1.9.0 uses one structured command grammar for parsing, help, validation, completion, and manpage generation.

The MCP stdio server exposes read-only `threadmoth_inspect`, `threadmoth_suggest`, and `threadmoth_explain` alongside `threadmoth_capabilities`, `threadmoth_preview`, `threadmoth_plan`, `threadmoth_apply_plan`, `threadmoth_transact_preview`, mutation, and transaction tools. Preview runs the same guarded planning and certification pipeline as mutation with commit disabled. Self-update remains CLI-only. JSON-RPC notifications, including `notifications/initialized`, are consumed without a response; ordinary requests receive a JSON-RPC result or standard error response.

## Plans, assertions, and updates

Prepare without writing, inspect the deterministic artifact, then apply it only if the workspace still matches:

```text
threadmoth plan --request request.json --output plan.json --summary
threadmoth explain --plan plan.json
threadmoth apply-plan --plan plan.json --summary
```

Plan input may include a top-level `assertions` array containing `file_exists`, `file_absent`, `sha256`, or bounded `literal_count` assertions. Assertions run against the prospective in-memory state before writes and against bytes read back after commit. A stale or tampered plan returns `PLAN_STALE`/`PLAN_INVALID` and writes nothing.

The updater is an explicit maintenance operation:

```text
threadmoth update --check
threadmoth update
threadmoth update --yes --json
```

It has no arbitrary URL/source flags and is not exposed through MCP. `threadmoth doctor` reports local installation provenance without performing a network check.

## Benchmark commands

The canonical benchmark surface is:

```text
threadmoth benchmark
threadmoth benchmark --quick
threadmoth benchmark --tough
threadmoth benchmark --torture
```

Short forms:

```text
threadmoth benchmark -q
threadmoth benchmark -t
threadmoth benchmark -x
```

Add `--json` (or `-j`) for machine-readable output. Without `--json`, benchmark and torture use the same compact human table and final PASS/FAIL summary.

For compatibility, Threadmoth still accepts:

```text
threadmoth benchmark tough
threadmoth torture
```

New documentation and automation should prefer the canonical flag forms.

## Mutation output

Mutation commands continue to return the full JSON certificate by default so existing agent and script integrations do not change behaviour in a patch release:

```text
threadmoth preview --request request.json
threadmoth mutate --request request.json
threadmoth transact --request transaction.json --preview
```

For a compact human view, add `--summary`:

```text
threadmoth preview --request request.json --summary
threadmoth mutate --request request.json --summary
threadmoth transact --request transaction.json --preview --summary
```

Desired-state requests use the explicit `desired_state` provider and carry desired bytes as JSON data. Preview reports the derived regions and effect budget before any write; mutate repeats the same guarded plan and verifies the landed desired hash.

Recovery discovery is read-only:

```text
threadmoth recover --list
threadmoth recover --inspect TRANSACTION_ID
threadmoth recover --transaction TRANSACTION_ID
```

The summary shows the outcome, provider, effect size, budget result, newline/preservation facts, hashes, and commit state without dumping the bounded diff. If a declared effect budget is too small, the summary lists the exact minimum values implied by the prepared plan for every undersized numeric dimension. Threadmoth never changes the caller's budget automatically.

## Provider naming

`filesystem` is the canonical lifecycle provider name in capabilities, schema output, certificates, and new requests:

```json
{
  "provider": "filesystem",
  "operation": {
    "type": "create_file",
    "expected_absent": true,
    "content": [104, 105, 10]
  }
}
```

Threadmoth 1.9.0 continues to accept the older request spelling `"provider":"file"` as a compatibility alias. When serialized or described by Threadmoth, the provider is canonicalized to `filesystem`.

## Shell completion

Threadmoth generates completion from the same CLI grammar used to parse commands:

```text
threadmoth completions powershell
threadmoth completions bash
threadmoth completions zsh
threadmoth completions fish
```

The generated script should be installed using the normal mechanism for the target shell. Threadmoth deliberately prints completion rather than silently rewriting shell startup files.

### PowerShell

For the current session:

```powershell
threadmoth completions powershell | Out-String | Invoke-Expression
```

For a persistent setup, save the generated completion script somewhere stable and source it from your PowerShell profile.

### Bash

For the current session:

```bash
source <(threadmoth completions bash)
```

For a persistent setup, save the output in your normal Bash completion directory or source it from your shell configuration.

### zsh

Generate the zsh completion file and place it in a directory on `fpath`, then refresh completion with `compinit`.

### fish

Save the output as `threadmoth.fish` in your normal fish completions directory.

## Help

All subcommands support generated help, and the main high-frequency commands include concrete examples in their long help:

```text
threadmoth --help
threadmoth mutate --help
threadmoth preview --help
threadmoth benchmark --help
threadmoth capabilities --help
```

The existing Threadmoth help-search surface remains available:

```text
threadmoth help mutate
threadmoth help --find refusal
```

Because command names, flags and enumerated values are parsed by `clap`, invalid input gets structured usage errors and close-match suggestions instead of a generic unknown-command fallback.

## Path-aware arguments

Arguments that represent files are marked as path values in the command grammar. Completion systems can therefore offer filesystem candidates for commands such as:

```text
threadmoth mutate --request <TAB>
threadmoth preview --request <TAB>
threadmoth suggest <TAB>
threadmoth inspect <TAB>
threadmoth capabilities --for <TAB>
```

## Man page

Generate the main roff man page:

```text
threadmoth manpage > threadmoth.1
```

Or write it directly:

```text
threadmoth manpage --output threadmoth.1
```

Packaging systems can install that file into the platform's normal manpage location.

## Doctor

`threadmoth doctor` reports core runtime information plus CLI usability hints including detected shell, whether the running executable directory appears on `PATH`, and the available completion/manpage commands.

```text
threadmoth doctor
```

## Compatibility policy

Threadmoth 1.9.0 keeps the important pre-1.3 command and provider spellings as compatibility routes, including `apply`, `dry-run`, positional benchmark profiles, `torture`, `transaction-preview`, and the request provider alias `file`. It accepts protocol 1.1, 1.2, and 1.3.0 requests with their promised semantics while advertising protocol 1.3.1 as current.

## Safe shorthands and plan review

`replace-exact FILE OLD NEW`, `set-value FILE PATH JSON_VALUE`, and
`create-file FILE CONTENT` compile into ordinary canonical requests. Each uses
one-file, one-target, one-region budgets and still refuses ambiguity or stale
state. MCP exposes the equivalent `threadmoth_exact_replace` and
`threadmoth_set_value` tools.

`threadmoth explain --plan plan.json --format diff` and
`threadmoth explain --plan plan.json --format markdown` are read-only review
renderers. They include the operation, hashes, fresh/stale state, and bounded
before/after diff; they never apply the plan.
