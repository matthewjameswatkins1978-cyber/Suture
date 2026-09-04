# Break the moth

Threadmoth is ready for field testing by humans and coding agents.

The goal of this phase is not stars. It is evidence: real repository edits, real refusals, real agent behaviour, and real failures we can fix.

## What is Threadmoth replacing?

Threadmoth is aimed at the improvised **last-mile editing toolbox** coding agents commonly assemble from `sed`, regex replacement, `apply_patch`, one-off Python/Node scripts, format-specific editors, direct file writes, and AST tooling.

Those tools are useful. The problem is that an autonomous agent can choose a different mutation path for every task, each with different ambiguity handling, stale-state behaviour, preservation rules, and evidence.

Threadmoth gives those common mutation jobs one deterministic boundary:

```text
agent intent
   |
   +-- text / regex
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

It does **not** try to abolish specialist tools. A formatter such as `rustfmt`, Prettier, Black, or `gofmt` can still decide the desired state; Threadmoth can be the guarded mutation boundary that decides whether that state is allowed to land.

Read [What Threadmoth replaces](docs/what-threadmoth-replaces.md) for the full comparison.

During field testing, a particularly useful question is:

> **When your agent did not use Threadmoth, what did it use instead, and why?**

That tells us whether Threadmoth is actually replacing the risky editing paths it was built to replace.

## What we want to learn

Please try Threadmoth on ordinary work and tell us:

- Did you or your agent discover the right Threadmoth capability without being heavily instructed?
- Did `threadmoth suggest`, `capabilities`, `examples`, and `--help` get you to a usable request quickly?
- Did Threadmoth refuse an edit that should have worked?
- Did it ever touch bytes you did not expect?
- Did you fall back to raw editing? If so, why and what tool did you use instead?
- Was a refusal useful enough to recover from?
- Which operating system, shell, language, and coding agent were you using?

## Try Threadmoth in five minutes

Install the latest release binary on `PATH`, or build/install from source with Rust 1.85+:

```text
cargo install --git https://github.com/matthewjameswatkins1978-cyber/Suture --bin threadmoth
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
  "version": "1.1.0",
  "request_id": "field-test-1",
  "file_path": "threadmoth-demo.txt",
  "cardinality": { "type": "exactly_one" },
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

We especially want results from Codex, Claude Code, Gemini CLI, Cline, OpenCode, and other coding agents.

See [the reproducible agent challenge](docs/agent-challenge.md) for a comparable multi-agent test.

## What counts as a useful report

A useful report can be short. Please include:

- Threadmoth version
- OS and shell
- coding agent/tool, if any
- language/provider involved
- the goal
- the command or request used
- expected result
- actual result
- what the agent would otherwise have used to edit the file, if known
- certificate/refusal output if relevant

Please remove secrets, private paths, tokens, and proprietary source before posting.

## Where to report things

Use the repository issue templates:

- **Agent failed to use Threadmoth** for discovery/tool-selection failures
- **Valid edit refused** for false refusals or awkward capability gaps
- **Unexpected mutation or preservation problem** for any surprising write or collateral change

For everything else, open a normal issue and start the title with `Field test:`.

## Success criteria for this phase

The first target is deliberately small:

- 20 serious testers
- 5 different coding agents
- 50 real mutations
- 10 useful failures

Ten useful failures are more valuable than a thousand drive-by stars.

## Current known limits

Threadmoth 1.5.x currently has a few deliberate boundaries:

- desired-state composition accepts explicit JSON `desired_bytes`; there is not yet a dedicated `--desired-file` convenience flag;
- SQL support is a common-dialect envelope, not complete vendor-specific SQL;
- HTML support does not claim deep JavaScript/CSS semantic targeting inside embedded `<script>` or `<style>` blocks;
- runtime loading of arbitrary third-party Tree-sitter grammars is intentionally not supported.

If one of those limits blocks real work, report the use case rather than assuming it must become a feature.

## The rule

> The parser gets to point at the cloth. It doesn't get to re-weave it.

Threadmoth should either make the exact authorised change or refuse. If you find a third category, please tell us.