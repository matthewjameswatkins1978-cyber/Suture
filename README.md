<p align="center">
  <img src="assets/threadmoth-icon-light.svg" alt="Threadmoth" width="180" />
</p>

<h1 align="center">Threadmoth</h1>

<p align="center"><strong>Fast, deterministic structural search and rewrite for AI agents.</strong></p>

<p align="center">Find the exact thing. Change only that thing. Prove what happened.</p>

---

Threadmoth is a small Rust runtime for **safe, source-preserving file mutation**. It gives agents a much better option than “open file, edit some text, hope for the best.”

It can target text, structured data, configuration files, Markdown, dotenv files, regex patterns, syntax-aware code operations, strict patches, and guarded file lifecycle operations while preserving unrelated bytes. Every successful mutation is checked, committed through one controlled core, and returned with evidence describing what was observed and what actually changed.

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

Threadmoth ships with correctness-checked benchmark and torture modes. In an observed Threadmoth 1.2 `tough`-profile run:

| workload | average |
|---|---:|
| 21-byte file | **0.87 ms** |
| 30 KB config | **1.33 ms** |
| 1 MB text file | **14.3 ms** |
| 5 MB text file | **55.7 ms** |
| 32 MB text file | **291 ms** |
| 2 MB / many lines | **29.0 ms** |
| 2 MB / one long line | **17.1 ms** |
| 250 tiny files | **5.25 ms total** |

The run finished with **zero wrong successful mutations** across the entire tough profile.

Put another way: Threadmoth can inspect, constrain, mutate, verify, and certify a 32 MB file in under a third of a second on this observed run, without reporting an incorrect successful mutation.

These are local measurements rather than universal cross-platform claims, so the checked-in benchmark harness remains the source of truth. See the [benchmark report](docs/benchmark-report.md) and run it on your own machine:

```text
threadmoth benchmark --tough
threadmoth benchmark --torture
```

Threadmoth 1.3.1 gives human benchmark output a compact table while keeping `--json` stable for agents and scripts. A successful run ends with a line like:

```text
PASS  8/8 cases · 0 wrong mutations · correctness checked
```

Torture uses the same presentation rather than a separate wall of state messages.

## Install

Download the platform binary from a GitHub release and put `threadmoth` (or `Threadmoth.exe`) on `PATH`.

Building from source requires Rust 1.85 or newer:

```text
cargo install --path .
```

`threadmoth` is the canonical command. The legacy `suture` executable remains available during the name migration.

## CLI that behaves like a proper CLI

Threadmoth uses a structured `clap` grammar. That gives the binary generated help, typo suggestions, typed arguments, shell completion and manpage generation from the same command definition.

The benchmark family is deliberately simple:

```text
threadmoth benchmark
threadmoth benchmark --quick
threadmoth benchmark --tough
threadmoth benchmark --torture
```

Short forms are available too:

```text
threadmoth benchmark -q
threadmoth benchmark -t
threadmoth benchmark -x
```

Existing automation using `threadmoth benchmark tough` or `threadmoth torture` remains compatible, but the flag forms above are canonical.

### Compact human mutation summaries

The full JSON certificate remains the default output for `preview`, `mutate`, and `transact`, so patch releases do not quietly break agent integrations.

For a compact human view:

```text
threadmoth preview --request request.json --summary
threadmoth mutate --request request.json --summary
threadmoth transact --request transaction.json --preview --summary
```

The summary reports outcome, provider, effect size, budget status, newline/preservation facts, hashes, and commit state without dumping the bounded diff. If a numeric effect budget is too small, it reports the exact minimum values implied by the prepared plan. Threadmoth never raises the budget for you.

### Shell completion

Generate completion directly from the binary:

```text
threadmoth completions powershell
threadmoth completions bash
threadmoth completions zsh
threadmoth completions fish
```

Once that output is installed into your shell’s normal completion location, Tab completion understands Threadmoth commands, flags, benchmark modes, and path-shaped arguments such as `--request`.

Examples:

```text
threadmoth ben<TAB>
threadmoth benchmark --<TAB>
threadmoth mutate --request <TAB>
```

### Better help and mistakes

```text
threadmoth --help
threadmoth mutate --help
threadmoth preview --help
threadmoth benchmark --help
threadmoth help mutate
threadmoth help --find refusal
```

High-frequency commands include concrete examples in long help. Misspelled commands and invalid enum values are handled by the CLI parser with suggestions rather than falling through to a generic “unknown command.”

### Man page and doctor

```text
threadmoth manpage > threadmoth.1
threadmoth doctor
```

`doctor` reports runtime basics plus shell/PATH usability hints and the commands for generating completion and a man page.

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
| Patch | exact unified-diff application with no fuzzy relocation |
| Code | Tree-sitter syntax-aware targeting for JavaScript/TypeScript, Python, Rust, and Go |
| Filesystem | guarded create/delete/rename/move operations |

Providers propose candidates. **Core alone commits them.**

`filesystem` is the canonical lifecycle-provider name. Threadmoth 1.3.1 still accepts the older request spelling `file` as a compatibility alias, but discovery, schemas, certificates, and newly serialized requests use `filesystem`.

## Real-world dogfood: Lantern Keeper

Threadmoth's first substantial repository repair was a formatting cleanup in Lantern Keeper. The starting scan found **45 source files** failing `rustfmt`:

- 16 CRLF-only;
- 12 format-drift;
- 17 mixed newline + format-drift cases.

The repair used guarded `text/replace`, strict `patch/unified_diff`, and `filesystem/create_file` operations with preview, pre-image hashes, path confinement, and effect budgets. Forty-five existing source files were processed and 30 files appeared in the final commit; those are deliberately reported as different counts rather than treating “inspected/processed” as “changed.”

Final verification was clean:

```text
cargo fmt --all -- --check                       PASS
cargo check --workspace --all-targets --locked  PASS
cargo clippy ... -D warnings                     PASS
tests                                             215 passed, 0 failed, 1 ignored
git diff --check                                  PASS
unexpected whole-file churn                      NO
final worktree                                    CLEAN
```

The important result was not that Threadmoth replaced `rustfmt`. It did not. `rustfmt` diagnosed the desired state; Threadmoth applied the bounded repairs and certified what changed.

That dogfood run directly produced the 1.3.1 usability fixes: canonical provider naming, compact summaries, multi-hunk budget guidance, and permanent CRLF/mixed/patch regression tests.

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
threadmoth benchmark --tough
threadmoth benchmark --torture
threadmoth completions powershell
threadmoth manpage
```

Machine mutation output is JSON on stdout by default. Diagnostics stay on stderr. Human mutation summaries are explicit with `--summary`.

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

The v1.1 protocol supports UTF-8, UTF-8 BOM, LF, CRLF, and either final-newline state. Unknown legacy encodings are refused. Requests may declare hard effect budgets. Multi-file transactions stage candidates in memory, journal before commit, roll back on failure where possible, and expose recovery state.

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [CLI](docs/cli.md)
- [Provider contract](docs/provider-contract.md)
- [Threat model](docs/threat-model.md)
- [Benchmark report](docs/benchmark-report.md)
- [v1 acceptance](docs/v1-acceptance.md)
- [v1.1 discovery](docs/v1.1-discovery.md)

## The short version

Threadmoth gives an AI agent a scalpel instead of a paint roller.

It is **small, quick, deterministic, source-preserving, refusal-first, and built to prove its own edits**.

That is the whole point.
