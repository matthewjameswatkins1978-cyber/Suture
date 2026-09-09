# What Threadmoth replaces

Threadmoth is designed to replace the improvised collection of tools AI coding agents commonly use for the **last mile between intent and the filesystem**.

It does not exist because tools such as `sed`, `jq`, `apply_patch`, formatters, regexes or small scripts are bad. They are useful tools.

The problem is that an autonomous coding agent will often assemble a different ad-hoc mutation path for every job, with different ambiguity rules, preservation behaviour, stale-state handling, failure modes and evidence.

Threadmoth gives those common editing jobs **one deterministic mutation boundary**.

## The old toolbox

| Traditional approach | What can go wrong in an agent loop |
|---|---|
| `sed`, `perl -pi`, PowerShell replacement | Text matches can hit the wrong occurrence and usually carry little structural evidence. |
| One-off Python or Node editing scripts | Every task invents a new mutation program with its own edge cases and failure behaviour. |
| Regex replacement | Powerful, but structural intent is reduced to pattern matching. |
| `apply_patch` / unified diffs | Excellent when an exact patch is the right representation, less useful when structural targeting or broader verification is required. |
| `jq`, `yq`, TOML/INI-specific tools | Good specialist tools, but each has different preservation and output semantics. |
| AST/codemod tooling | Powerful for semantic transformations, but many workflows regenerate or reformat more source than the requested local edit. |
| Direct whole-file writes | Simple, but place stale context, collateral-edit detection and post-write verification on the agent. |
| Formatters such as `rustfmt`, Prettier, Black and `gofmt` | Excellent desired-state producers, but not mutation-policy engines with budgets, stale guards, recovery and certificates. |

Threadmoth does **not** claim all of these tools should disappear. It gives agents a common safe boundary for the mutation jobs that would otherwise be scattered across them.

```text
AI decides what should change
        |
        +-- exact text / bounded pattern
        +-- JSON / JSONC / TOML / YAML / INI / dotenv
        +-- Markdown regions
        +-- syntax nodes, including Java/C#/PHP/HCL and existing languages
        +-- strict patch
        +-- desired file state
        +-- filesystem operation
                |
                v
           THREADMOTH
                |
      identity / cardinality
      workspace containment
      effect budgets
      stale-state protection
      structural validation
      transaction recovery
      post-write verification
                |
                v
          certified result
```

## What it is replacing in practice

A useful mental model is:

```text
sed
+ regex
+ apply_patch
+ jq/yq/INI-style structural editing
+ little one-off editing scripts
+ Tree-sitter targeting
+ guarded file operations
+ deterministic diff planning
+ transaction recovery

            becomes

        one Threadmoth boundary
```

The point is not that Threadmoth has every feature of every tool in that list. The point is that the common **file-mutation role** they play inside an AI agent can now use one set of rules and evidence.

## Specialist tools still have a job

Threadmoth intentionally does not execute formatters, compilers, language servers, Git, arbitrary subprocesses or general network tools.

A formatter can remain responsible for deciding the desired state:

```text
rustfmt / Prettier / Black / gofmt
                 |
            desired bytes
                 v
             Threadmoth
                 |
       bounded diff + budgets
       stale-state protection
       verification + certificate
```

The formatter decides what the file should look like. Threadmoth decides whether that requested divergence is allowed to land and proves what actually landed.

Likewise, HCL syntax support does not make Threadmoth a Terraform engine, and source-code syntax targeting does not make Threadmoth a compiler or type checker.

## The questions Threadmoth takes away from the agent

Threadmoth is most useful when you care about questions such as:

- Did the agent target the exact occurrence it intended?
- Was the file changed after the agent observed it?
- Did an external desired state touch 400 lines when the caller expected 12?
- Was the requested syntax node actually unique?
- Did anything outside the authorised effect change?
- Can an interrupted multi-file operation be recovered safely?
- Can another agent, supervisor or human inspect evidence of what happened?

If none of those questions matter, a normal text editor or shell command may be enough.

When they do matter, Threadmoth is the deterministic pair of hands between **AI intent** and **filesystem reality**.

> The goal is not "never use sed, jq, Prettier or rustfmt again."
>
> The goal is **do not make the AI agent itself responsible for deciding whether its mutation was safe.**
