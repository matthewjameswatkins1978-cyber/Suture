# Threadmoth documentation

Threadmoth is a fast, deterministic structural search-and-rewrite runtime for AI agents. The main [README](../README.md) is the best place to start; this page is the map for the technical documentation.

## Start here

| Document | Purpose |
|---|---|
| [CLI guide](cli.md) | Threadmoth 1.5 commands, desired-state mode, recovery inspection, completion, help and manpage generation |
| [Architecture](architecture.md) | How Threadmoth separates observation, identification, mutation, verification, and commit |
| [Protocol](protocol.md) | Request/response contract and machine-facing behaviour |
| [Provider contract](provider-contract.md) | Rules every mutation provider must obey |
| [Threat model](threat-model.md) | What Threadmoth protects against, and what it deliberately does not do |
| [Benchmark report](benchmark-report.md) | Reproducible correctness-first performance evidence |
| [v1 acceptance](v1-acceptance.md) | Acceptance criteria and release guarantees |
| [v1.1 discovery](v1.1-discovery.md) | Capability/schema discovery behaviour |
| [Syntax targeting](syntax-targeting.md) | AST-grounded versus AST-typed source-preserving edits |
| [Desired state](desired-state.md) | Deterministic desired-state planning and verification |

## Core idea

Threadmoth does not ask an agent to be careful while performing an unconstrained edit. It narrows the edit itself.

```text
OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY
```

A provider may identify and propose a candidate mutation, but **Core alone commits**. If identity is ambiguous, reality has changed since observation, the request exceeds its bounds, or validation fails, the operation is refused rather than guessed.

## Performance

Threadmoth includes its own correctness-checked benchmark and torture modes:

```text
threadmoth benchmark
threadmoth benchmark --tough
threadmoth benchmark --torture
```

The benchmark checks expected bytes before presenting timing results. The current observed tough-profile run recorded about 14.3 ms for a 1 MB file, about 55.7 ms for 5 MB, and about 291 ms for 32 MB, with zero wrong successful mutations across the profile. Treat these as local measurements, not universal platform claims; see the [benchmark report](benchmark-report.md) for the exact evidence.

## Useful CLI discovery

```text
threadmoth help
threadmoth doctor
threadmoth capabilities
threadmoth schema
threadmoth examples
threadmoth suggest PATH
threadmoth completions powershell
threadmoth manpage
```

See the [CLI guide](cli.md) for shell completion and the Threadmoth 1.5 command surface.

For machine integration, mutation output is JSON on stdout, diagnostics are on stderr, and stable exit codes distinguish success/no-change, refusal, and runtime failure.

## Design rule

The important contract is not merely that Threadmoth can rewrite a file. It is that an `APPLIED` result carries enough evidence to say what bytes were observed, what edit was authorised, what validation ran, and what bytes were actually committed.
