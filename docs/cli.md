# Threadmoth CLI

Threadmoth 1.5.1 uses one structured command grammar for parsing, help, validation, completion, and manpage generation.

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

Threadmoth 1.5.1 continues to accept the older request spelling `"provider":"file"` as a compatibility alias. When serialized or described by Threadmoth, the provider is canonicalized to `filesystem`.

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

Threadmoth 1.5.1 keeps the important pre-1.3 spellings as compatibility routes, including `apply`, `dry-run`, positional benchmark profiles, `torture`, `transaction-preview`, and the request provider alias `file`. They are not the preferred documentation surface, but existing agent scripts do not need an immediate flag-day migration.
