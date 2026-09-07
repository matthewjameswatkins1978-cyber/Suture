<p align="center">
  <img src="assets/threadmoth-icon-light.svg" alt="Threadmoth" width="180" />
</p>

<h1 align="center">Threadmoth</h1>

<p align="center"><strong>Fast, deterministic structural search and rewrite for AI agents.</strong></p>

<p align="center">Find the exact thing. Change only that thing. Prove what happened.</p>

---

Threadmoth is a small Rust runtime for **safe, source-preserving file mutation**. It gives humans and AI agents a better option than “open file, edit some text, hope for the best.”

> The parser gets to point at the cloth. It doesn’t get to re-weave it.

Providers and planners propose bounded byte edits. **Core alone guards, applies, verifies, and certifies them.** Threadmoth can target text, structured data, configuration files, Markdown, dotenv files, regex patterns, syntax-aware code and web structures, strict patches, desired state, and guarded file lifecycle operations while preserving unrelated bytes wherever the provider contract promises it.

The execution pipeline is deliberately boring in the best possible way:

```text
OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY
```

If reality is ambiguous, stale, unsafe, or outside the requested bounds, Threadmoth refuses instead of guessing.

## Current status

The latest published release is **Threadmoth 1.6.0**.

**Threadmoth 1.7.0 is implemented and has passed its local release gates; publication is pending.** It keeps the same narrow mutation boundary while making Threadmoth substantially easier for agents to use correctly.

### What 1.7 adds

- **MCP parity**: read-only inspect, suggest, explain, path-specific capabilities, and transaction preview alongside preview, mutate, capabilities, and transact.
- **Guarded candidate selection**: when several identical targets are genuinely ambiguous, Threadmoth can return deterministic candidate identities so a caller can safely say “that one” against the exact observed file state.
- **Protocol 1.2 compatibility** for candidate-selection guards while retaining compatibility with existing 1.1 requests where applicable.
- **Provider-preserving refusal recovery** so a code, JSON, Markdown, or other provider refusal does not collapse into an inappropriate generic text-edit suggestion.
- **Better filename detection** for common real-world files without turning provider detection into guesswork.
- **macOS release targets** for Apple Silicon and Intel in addition to Windows and Linux.
- **A 30-second agent workflow** showing discovery, preview, refusal recovery, guarded selection, and commit.

The important part is what 1.7 does **not** add: no LLM inside Threadmoth, no fuzzy relocation, no Git execution, no formatter execution, no network access, and no automatic weakening of a refusal.

## Why Threadmoth?

AI agents are very good at deciding *what* should change. They are much less trustworthy when the last step is an unconstrained text edit.

Threadmoth turns that final step into a narrow deterministic operation with explicit identity, cardinality, validation, effect budgets, and post-commit verification.

**In practical terms:**

- **Fast**: designed for tiny local edits with low overhead.
- **Deterministic**: the same request against the same bytes means the same decision.
- **Structural**: understands more than raw string replacement.
- **Source-preserving**: unrelated bytes stay untouched where the provider guarantees it.
- **Refusal-first**: ambiguity is surfaced, not silently “resolved.”
- **Agent-friendly**: JSON in, JSON out, stable exit codes, schemas, capabilities, preview, recovery, and MCP.
- **Auditable**: `APPLIED` includes hashes, changed byte ranges, bounded diff evidence, and commit evidence.
- **Contained**: workspace boundaries, traversal checks, symlink containment, staged writes, stale-read protection, and recovery state.

## What does Threadmoth replace?

Threadmoth is designed to replace the **improvised last-mile editing toolbox** AI coding agents commonly assemble from `sed`, regex replacement, `apply_patch`, one-off Python/Node scripts, direct file writes, format-specific editors, and AST tooling.

Those tools are useful. The problem is that an autonomous agent can choose a different mutation path for every task, each with different ambiguity handling, stale-state behaviour, preservation rules, failure modes, and evidence.

A useful mental model is:

```text
sed
+ regex
+ apply_patch
+ jq/yq-style structural editing
+ one-off editing scripts
+ Tree-sitter targeting
+ guarded file operations
+ deterministic diff planning
+ transaction recovery

            becomes

        one Threadmoth boundary
```

Threadmoth does **not** try to make specialist tools obsolete. Formatters such as `rustfmt`, Prettier, Black, and `gofmt` can still decide the desired state. Threadmoth can then enforce effect budgets, stale-state checks, containment, verification, and certification before that state lands.

The goal is not “never use `sed`, `jq`, Prettier, or `rustfmt` again.”

The goal is **do not make the AI agent itself responsible for deciding whether its file mutation was safe.**

Read the full comparison: [What Threadmoth replaces](docs/what-threadmoth-replaces.md).

## It is also very quick

Threadmoth ships with correctness-checked benchmark and torture modes. The benchmark harness checks correctness as well as timing, and exits non-zero if an expected successful mutation is wrong.

Run it on your own machine:

```text
threadmoth benchmark --quick
threadmoth benchmark --tough
threadmoth benchmark --torture
```

A successful tough run ends with a compact signal like:

```text
PASS  8/8 cases · 0 wrong mutations · correctness checked
```

The tough profile includes tiny files, large files, many-line inputs, repeated small-file work, and pathological long-line refinement. Torture adds deterministic refusal, preservation, transaction, path/symlink, recovery, and FOOTGUN regression coverage.

These are local measurements rather than universal performance claims, so the checked-in benchmark harness remains the source of truth. See the [benchmark report](docs/benchmark-report.md).

## Install

Download the platform binary from a [GitHub release](https://github.com/matthewjameswatkins1978-cyber/Suture/releases) and put `threadmoth` (or `threadmoth.exe`) on `PATH`.

Building from source requires Rust 1.85 or newer:

```text
cargo install --path .
```

`threadmoth` is the canonical command. The historical `suture` executable is no longer built.

## CLI that behaves like a proper CLI

Threadmoth uses a structured `clap` grammar. That gives the binary generated help, typo suggestions, typed arguments, shell completion, and manpage generation from the same command definition.

```text
threadmoth --help
threadmoth doctor
threadmoth capabilities
threadmoth capabilities --for src/main.rs
threadmoth schema
threadmoth examples
threadmoth suggest Cargo.toml --goal set-value --at package.version
threadmoth inspect src/main.rs
threadmoth explain TARGET_AMBIGUOUS
```

### Preview, mutate, transact

The full JSON certificate is the default output for `preview`, `mutate`, and `transact`.

```text
threadmoth preview --request request.json
threadmoth mutate --request request.json
threadmoth transact --request transaction.json --preview
threadmoth transact --request transaction.json
```

For a compact human view:

```text
threadmoth preview --request request.json --summary
threadmoth mutate --request request.json --summary
threadmoth transact --request transaction.json --preview --summary
```

The summary reports outcome, provider, effect size, budget status, newline/preservation facts, hashes, and commit state without dumping the bounded diff. If a numeric effect budget is too small, Threadmoth reports the minimum implied by the prepared plan. It never raises the budget for you.

### Shell completion

```text
threadmoth completions powershell
threadmoth completions bash
threadmoth completions zsh
threadmoth completions fish
```

### Man page and doctor

```text
threadmoth manpage > threadmoth.1
threadmoth doctor
```

`doctor` reports runtime basics plus shell/PATH usability hints.

## 60-second example

Suppose `config.txt` contains one occurrence of `old` and you want exactly that one occurrence changed to `new`:

```text
echo {"version":"1.1.0","request_id":"example-1","file_path":"config.txt","cardinality":{"type":"exactly_one"},"operation":{"provider":"text","operation":{"type":"replace","target":"old","replacement":"new"}}} | threadmoth mutate
```

If the target appears twice, Threadmoth does **not** choose one. It returns `REFUSED` with bounded candidate information so the caller can disambiguate deliberately.

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

Protocol 1.1 requests remain the common baseline. Threadmoth 1.7 adds protocol 1.2 compatibility for deterministic guarded candidate selection.

## The 30-second agent loop

Threadmoth is intended to be discovered rather than memorised:

```text
capabilities / suggest
        ↓
preview
        ↓
APPLIED or REFUSED
        ↓
if REFUSED: explain / recover / select candidate deliberately
        ↓
preview again
        ↓
mutate
        ↓
certificate
```

A minimal instruction for an agent is:

```text
Threadmoth is available for deterministic, source-preserving file mutation.
Prefer it for bounded structural edits when its capabilities apply.
Discover usage with `threadmoth capabilities` and `threadmoth suggest <path>`.
Preview when uncertain. Treat REFUSED as information, not an obstacle to route around.
```

See [Agent integration](docs/agent-integration.md).

## What a successful mutation gives you

An `APPLIED` certificate includes:

- pre- and post-mutation SHA-256 hashes;
- the byte ranges that changed;
- a bounded diff;
- structural validation results;
- preservation facts;
- effect-budget evidence;
- commit evidence from bytes read back after replacement.

That is the central Threadmoth promise: **success is not merely “the write call returned OK.”** Success means the requested mutation survived the full observe-to-certify pipeline.

## Providers

Threadmoth currently supports:

| Provider | What it does |
|---|---|
| Text | exact replacement, byte-range edits, desired-state operations |
| JSON / JSONC | source-range structural edits |
| TOML | structure-aware edits using `toml_edit` |
| YAML | conservative source-preserving subset |
| Markdown | edits bounded to structural Markdown regions |
| dotenv | key edits while preserving unrelated comments and lines |
| Pattern | bounded Rust regex operations |
| Patch | exact unified-diff application with no fuzzy relocation |
| Code | shared Tree-sitter syntax-aware targeting for JavaScript, JSX, TypeScript, TSX, Python, Rust, Go, C, C++, Bash/Shell, PowerShell, and common SQL |
| Web | shared structural Tree-sitter targeting for HTML, CSS, and XML |
| Desired state | deterministic bounded diff from observed bytes to explicitly supplied desired bytes |
| Filesystem | guarded create/delete/rename/move operations |

Providers propose candidates. **Core alone commits them.**

`filesystem` is the canonical lifecycle-provider name. The older request spelling `file` remains a compatibility alias in supported protocol versions; discovery and newly serialized requests use `filesystem`.

## MCP

Run the stdio server with:

```text
threadmoth mcp
```

The published 1.6 surface includes mutation, preview, capabilities, and transactions. Threadmoth 1.7 extends MCP with the read-only discovery/recovery tools needed for the full safe agent loop, including transaction preview.

MCP remains an adapter over the same deterministic core. It does not get a looser safety contract than the CLI.

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

## Built for agents, not just humans

Threadmoth exposes machine-readable discovery and validation surfaces:

```text
threadmoth --version
threadmoth doctor
threadmoth capabilities
threadmoth schema
threadmoth examples
threadmoth suggest PATH
threadmoth inspect PATH
threadmoth explain REASON_CODE
threadmoth preview --request request.json
threadmoth mutate --request request.json
threadmoth recover
threadmoth recover --list
threadmoth recover --inspect TRANSACTION_ID
threadmoth recover --transaction TRANSACTION_ID
threadmoth benchmark --tough
threadmoth benchmark --torture
threadmoth completions powershell
threadmoth manpage
threadmoth mcp
```

Machine mutation output is JSON on stdout by default. Diagnostics stay on stderr. Human mutation summaries are explicit with `--summary`.

Exit codes are:

```text
0  APPLIED or NO_CHANGE
2  REFUSED
3  runtime failure
```

### Portable agent integrations

Threadmoth also ships a portable [Agent Skill](skills/threadmoth/SKILL.md) for agents that support the open `SKILL.md` format. The same skill is bundled as a minimal Claude Code plugin and Gemini CLI extension in this repository.

These adapters teach discovery, bounded requests, preview-first operation, refusal handling, and certificate preservation. They do not install or replace the `threadmoth` executable.

See [Agent integration](docs/agent-integration.md) and the [distribution ledger](docs/distribution-ledger.md).

## Safety model

Threadmoth is intentionally narrow. It mutates files. It does **not** execute Git, builds, tests, formatters, arbitrary subprocesses, or network operations.

The workspace layer confines paths, rejects traversal and escaping symlinks, stages writes in the destination directory, flushes before replacement, rechecks the observed hash immediately before commit, and reads the committed bytes back before certification.

Supported requests are UTF-8/UTF-8 BOM with LF or CRLF and either final-newline state. Unknown legacy encodings are refused. Requests may declare hard effect budgets. Multi-file content transactions stage candidates in memory, journal before commit, roll back on failure where possible, and expose recovery state.

Threadmoth never turns an ambiguous target into a successful mutation merely because choosing one would be convenient.

## Documentation

- [Documentation index](docs/README.md)
- [Agent integration](docs/agent-integration.md)
- [What Threadmoth replaces](docs/what-threadmoth-replaces.md)
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
