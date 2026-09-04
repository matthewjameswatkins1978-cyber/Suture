<p align="center">
  <img src="assets/threadmoth-icon-light.svg" alt="Threadmoth" width="180" />
</p>

<h1 align="center">Threadmoth</h1>

<p align="center"><strong>Fast, deterministic structural search and rewrite for AI agents.</strong></p>

<p align="center">Find the exact thing. Change only that thing. Prove what happened.</p>

---

Threadmoth is a small Rust runtime for **safe, source-preserving file mutation**. It gives agents a much better option than “open file, edit some text, hope for the best.”

It can target text, structured data, configuration files, Markdown, dotenv files, regex patterns, and syntax-aware code operations while preserving unrelated bytes. Every successful mutation is checked, committed through one controlled core, and returned with evidence describing what was observed and what actually changed.

The execution pipeline is deliberately boring in the best possible way:

```text
OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY
```

If reality is ambiguous, stale, unsafe, or outside the requested bounds, Threadmoth refuses instead of guessing.

## Why Threadmoth?

AI agents are very good at deciding *what* should change. They are much less trustworthy when the last step is an unconstrained text edit.

Threadmoth turns that final step into a narrow deterministic operation with explicit identity, cardinality, validation, effect budgets, and post-commit verification.

**In practical terms:**

- **Fast**: designed for tiny local edits with very low overhead.
- **Deterministic**: the same request against the same bytes means the same decision.
- **Structural**: understands more than raw string replacement.
- **Source-preserving**: unrelated bytes stay untouched.
- **Refusal-first**: ambiguity is surfaced, not silently “resolved.”
- **Agent-friendly**: JSON in, JSON out, stable exit codes, schemas, capabilities, preview, and recovery.
- **Auditable**: `APPLIED` includes hashes, changed byte ranges, bounded diff evidence, and commit evidence.
- **Contained**: workspace boundaries, traversal checks, symlink containment, staged writes, stale-read protection, and recovery state.

## It is also very quick

Threadmoth ships with correctness-checked benchmark and torture commands. On one Windows x86_64 Threadmoth 1.2 tough-profile run:

| workload | average |
|---|---:|
| 21-byte file | **456 us** |
| 30 KB config | **836 us** |
| 1 MB text file | **7.57 ms** |
| 5 MB text file | **37.3 ms** |
| 32 MB text file | **252 ms** |

That run reported **zero wrong successful mutations** across the tough profile. These are local measurements rather than universal cross-platform claims, so the checked-in benchmark harness remains the source of truth. See the [benchmark report](docs/benchmark-report.md) and run it on your own machine:

```text
threadmoth benchmark tough
threadmoth torture
```

## Install

Download the platform binary from a GitHub release and put `threadmoth` (or `Threadmoth.exe`) on `PATH`.

Building from source requires Rust 1.85 or newer:

```text
cargo install --path .
```

`threadmoth` is the canonical command for the 1.2 release. The legacy `suture` executable remains as a compatibility alias during the name migration.

## 60-second example

Suppose `config.txt` contains one occurrence of `old` and you want exactly that one occurrence changed to `new`:

```text
echo {"version":"1.1.0","request_id":"example-1","file_path":"config.txt","cardinality":{"type":"exactly_one"},"operation":{"provider":"text","operation":{"type":"replace","target":"old","replacement":"new"}}} | threadmoth mutate
```

If the target appears twice, Threadmoth does **not** choose one. It returns `REFUSED` with bounded candidate offsets and context so the caller can disambiguate deliberately.

Structured edits work the same way. For JSON:

```json
{
  "version": "1.1.0",
  "request_id": "example-2",
  "file_path": "config.json",
  "cardinality": { "type": "exactly_one" },
  "operation": {
    "provider": "json",
    "operation": {
      "type": "set",
      "path": "$.server.port",
      "value": 8080
    }
  }
}
```

Use `preview` before committing when you want to inspect the candidate first:

```text
threadmoth preview --request request.json
threadmoth mutate --request request.json
```

## What a successful mutation gives you

An `APPLIED` certificate includes:

- pre- and post-mutation SHA-256 hashes;
- the byte ranges that changed;
- a bounded diff;
- structural validation results;
- commit evidence from the bytes read back after replacement.

That is the central Threadmoth promise: **success is not merely “the write call returned OK.”** Success means the requested mutation survived the full observe-to-certify pipeline.

## Providers

Threadmoth currently supports:

| Provider | What it does |
|---|---|
| Text | exact replacement, byte-range edits, desired-state operations |
| JSON / JSONC | source-range structural edits |
| TOML | structure-aware edits using `toml_edit` |
| YAML | conservative source-preserving subset |
| Markdown | edits bounded heading sections |
| dotenv | key edits while preserving comments |
| Pattern | bounded Rust regex operations |
| Code | Tree-sitter syntax-aware targeting for JavaScript/TypeScript, Python, Rust, and Go |

Providers propose candidates. **Core alone commits them.**

## Built for agents, not just humans

Threadmoth exposes machine-readable discovery and validation surfaces:

```text
threadmoth --version
threadmoth doctor
threadmoth capabilities
threadmoth schema
threadmoth examples
threadmoth suggest PATH
threadmoth preview --request request.json
threadmoth mutate --request request.json
threadmoth recover
threadmoth benchmark
threadmoth benchmark tough
threadmoth torture
```

Machine mutation output is JSON on stdout. Diagnostics stay on stderr.

Exit codes are:

```text
0  APPLIED or NO_CHANGE
2  REFUSED
3  runtime failure
```

This makes Threadmoth easy to drop into agent loops, scripts, orchestration systems, and toolbelts without scraping prose.

## Safety model

Threadmoth is intentionally narrow. It mutates files. It does **not** execute Git, builds, tests, formatters, arbitrary subprocesses, or network operations.

The workspace layer confines paths, rejects traversal and escaping symlinks, stages writes in the destination directory, flushes before replacement, rechecks the observed hash immediately before commit, and reads the committed bytes back before certification.

The v1.1 contract supports UTF-8, UTF-8 BOM, LF, CRLF, and either final-newline state. Unknown legacy encodings are refused. Requests may declare hard effect budgets. Multi-file transactions stage candidates in memory, journal before commit, roll back on failure where possible, and expose recovery state.

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [Provider contract](docs/provider-contract.md)
- [Threat model](docs/threat-model.md)
- [Benchmark report](docs/benchmark-report.md)
- [v1 acceptance](docs/v1-acceptance.md)
- [v1.1 discovery](docs/v1.1-discovery.md)

## The short version

Threadmoth gives an AI agent a scalpel instead of a paint roller.

It is **small, quick, deterministic, source-preserving, refusal-first, and built to prove its own edits**.

That is the whole point.
