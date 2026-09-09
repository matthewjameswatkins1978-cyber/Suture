# Threadmoth documentation

Threadmoth is a fast, deterministic structural search-and-rewrite runtime for AI agents. The main [README](../README.md) is the best place to start; this page is the map for the technical documentation.

## Start here

| Document | Purpose |
|---|---|
| [CLI guide](cli.md) | Threadmoth 1.8.1 commands, plans, assertions, updater, safe shorthands, desired-state mode, recovery inspection, completion, help and manpage generation |
| [Agent integration](agent-integration.md) | Minimal instructions and safe usage flow for coding agents and MCP clients |
| [Architecture](architecture.md) | How Threadmoth separates observation, identification, mutation, verification, and commit |
| [Protocol](protocol.md) | Request/response contract and machine-facing behaviour |
| [Provider contract](provider-contract.md) | Rules every mutation provider must obey |
| [Threat model](threat-model.md) | What Threadmoth protects against, and what it deliberately does not do |
| [Benchmark report](benchmark-report.md) | Reproducible correctness-first performance evidence |
| [v1 acceptance](v1-acceptance.md) | Acceptance criteria and release guarantees |
| [v1.1 discovery](v1.1-discovery.md) | Capability/schema discovery behaviour introduced with protocol 1.1 |
| [Syntax targeting](syntax-targeting.md) | AST-grounded versus AST-typed source-preserving edits |
| [Desired state](desired-state.md) | Deterministic desired-state planning and verification |
| [Distribution ledger](distribution-ledger.md) | Shipped agent integrations, outreach status, metrics, and next gates |

## Core idea

Threadmoth does not ask an agent to be careful while performing an unconstrained edit. It narrows the edit itself.

```text
OBSERVE -> IDENTIFY -> GUARD -> PLAN -> VERIFY PROSPECTIVE -> MUTATE -> VERIFY COMMITTED -> CERTIFY
```

A provider may identify and propose a candidate mutation, but **Core alone commits**. If identity is ambiguous, reality has changed since observation, the request exceeds its bounds, or validation fails, the operation is refused rather than guessed.

## Performance

Threadmoth includes its own correctness-checked benchmark and torture modes:

```text
threadmoth benchmark
threadmoth benchmark --tough
threadmoth benchmark --torture
```

The benchmark checks expected bytes before presenting timing results. Treat reported timings as local measurements, not universal platform claims; see the [benchmark report](benchmark-report.md) and run the checked-in harness on the machine that matters to you.

## Useful CLI discovery

```text
threadmoth help
threadmoth doctor --json
threadmoth doctor
threadmoth capabilities
threadmoth capabilities --for PATH
threadmoth schema
threadmoth examples
threadmoth suggest PATH
threadmoth inspect PATH
threadmoth explain REASON_CODE
threadmoth completions powershell
threadmoth manpage
threadmoth mcp
```

See the [CLI guide](cli.md) for the Threadmoth 1.8.1 command surface and [Agent integration](agent-integration.md) for the intended discovery → plan/preview → refusal recovery → commit loop.

For machine integration, mutation output is JSON on stdout, diagnostics are on stderr, and stable exit codes distinguish success/no-change, refusal, and runtime failure.

## Design rule

The important contract is not merely that Threadmoth can rewrite a file. It is that an `APPLIED` result carries enough evidence to say what bytes were observed, what edit was authorised, what validation ran, and what bytes were actually committed.
