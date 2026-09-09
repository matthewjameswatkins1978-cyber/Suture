# Break the moth

## Threadmoth 1.9.0 coverage and recovery focus

Threadmoth is ready for field testing by humans and coding agents. The goal is not stars. It is evidence: real repository edits, real refusals, real agent behaviour, and real failures we can fix.

Field testing should record whether an agent stayed inside the Threadmoth workflow after `REFUSED`: did it consume the certificate remedy, construct the guarded retry, and obtain a final certificate? Record ambiguity, stale state, bad schema, effect-budget, prepared-plan, transaction, path-escape and unsupported-preservation cases separately. A raw-write fallback is an observed outcome, not a successful recovery.

## What is Threadmoth replacing?

Threadmoth is aimed at the improvised **last-mile editing toolbox** coding agents commonly assemble from `sed`, regex replacement, `apply_patch`, one-off Python/Node scripts, format-specific editors, direct file writes and AST tooling.

Those tools are useful. The problem is that an autonomous agent can choose a different mutation path for every task, each with different ambiguity handling, stale-state behaviour, preservation rules and evidence.

Threadmoth gives those common mutation jobs one deterministic boundary:

```text
agent intent
   |
   +-- exact text / bounded pattern
   +-- structured data
   +-- syntax nodes
   +-- exact patches
   +-- desired file state
   +-- filesystem operations
          |
          v
      THREADMOTH
          |
 identity + cardinality
 effect budgets
 stale-state checks
 containment
 verification
 recovery
 certificate
```

It does **not** try to abolish specialist tools. A formatter such as `rustfmt`, Prettier, Black or `gofmt` can still decide the desired state; Threadmoth can be the guarded mutation boundary that decides whether that state is allowed to land.

Read [What Threadmoth replaces](docs/what-threadmoth-replaces.md) for the full comparison.

During field testing, a particularly useful question is:

> **When your agent did not use Threadmoth, what did it use instead, and why?**

## What we want to learn

Please try Threadmoth on ordinary work and tell us:

- Did you or your agent discover the right capability without heavy instruction?
- Did `threadmoth capabilities --for PATH`, `suggest`, `examples` and `--help` get you to a usable request quickly?
- Did the 1.9 coverage model correctly classify structured, syntax, region, exact and opaque targets?
- Did Threadmoth refuse an edit that should have worked?
- Did it ever touch bytes you did not expect?
- Did you fall back to raw editing? If so, why and what tool did you use instead?
- Was a refusal useful enough to recover from?
- Which operating system, shell, language/provider and coding agent were you using?

Particularly useful 1.9 coverage tests include Java, C#, PHP, HCL, nested YAML, INI-style files, unknown UTF-8 fallback and opaque/binary refusal.

## Try Threadmoth in five minutes

Install the latest published release binary on `PATH`, or build the current source with Rust 1.85+:

```text
cargo install --git https://github.com/matthewjameswatkins1978-cyber/Threadmoth --bin threadmoth
```

Check the installation:

```text
threadmoth --version
threadmoth doctor
threadmoth capabilities
```

Create a file named `threadmoth-demo.txt` containing exactly:

```text
old
```

Create `request.json`:

```json
{
  "version": "1.3.1",
  "request_id": "field-test-1",
  "file_path": "threadmoth-demo.txt",
  "cardinality": { "type": "exactly_one" },
  "budget": { "max_files": 1, "max_matches": 1 },
  "operation": {
    "provider": "text",
    "operation": {
      "type": "replace",
      "target": "old",
      "replacement": "new"
    }
  }
}
```

Preview first:

```text
threadmoth preview --request request.json --summary
```

Then apply it:

```text
threadmoth mutate --request request.json --summary
```

Now change the file to contain two copies of `old` and run the same request again. Threadmoth should refuse instead of picking one.

That refusal is part of the product.

## Give an AI agent only this much help

For a useful discovery test, do not teach the agent the Threadmoth request format. Give it only:

```text
Threadmoth is installed and available for deterministic, source-preserving file mutation.
Use it when its capabilities fit the task.
Discover it yourself with `threadmoth --help`, `threadmoth capabilities`, and `threadmoth suggest <path>`.
Preview when uncertain.
Do not bypass a Threadmoth refusal with a broader raw edit unless the user explicitly authorizes that.
```

Then give the agent a normal repository task.

We especially want results from Codex, Claude Code, Gemini CLI, Cline, OpenCode and other coding agents.

See [the reproducible agent challenge](docs/agent-challenge.md) for a comparable multi-agent test.

## What counts as a useful report

A useful report can be short. Please include:

- Threadmoth version and build flavour if relevant;
- OS and shell;
- coding agent/tool, if any;
- language/provider involved;
- the goal;
- the command or request used;
- expected result;
- actual result;
- what the agent would otherwise have used to edit the file, if known;
- certificate/refusal output if relevant.

Please remove secrets, private paths, tokens and proprietary source before posting.

## Where to report things

Use the repository issue templates:

- **Agent failed to use Threadmoth** for discovery/tool-selection failures;
- **Valid edit refused** for false refusals or awkward capability gaps;
- **Unexpected mutation or preservation problem** for any surprising write or collateral change.

For everything else, open a normal issue and start the title with `Field test:`.

## Current 1.9 known boundaries

Threadmoth 1.9 deliberately stops short of several tempting expansions:

- Kotlin and Swift syntax targeting remain deferred until parser/integration quality clears the admission bar;
- Dockerfile and Makefile remain explicit exact-text targets rather than partial structural claims;
- Java `.properties` remains exact text because its escaping, continuation and historical encoding rules do not fit the current UTF-8 structured contract cleanly;
- advanced YAML constructs such as anchors, aliases, tags, merge keys and directives refuse when local preservation cannot be proved;
- SQL support is a common-dialect envelope, not complete vendor-specific SQL;
- HTML support does not claim deep JavaScript/CSS semantics inside embedded `<script>` or `<style>` regions;
- runtime loading of arbitrary third-party Tree-sitter grammars is intentionally unsupported;
- the full compiler optimization matrix, PGO validation and published 1.9 updater exercise remain release-performance gates until completed.

If one of those limits blocks real work, report the use case rather than assuming it must become a feature.

## The rule

> The parser gets to point at the cloth. It doesn't get to re-weave it.

Threadmoth should either make the exact authorised change or refuse. If you find a third category, please tell us.
