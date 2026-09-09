# Agent integration

Threadmoth is designed to be discovered by an agent rather than memorised by one.

For most coding-agent setups, start with this minimal instruction:

```text
Threadmoth is installed and available for deterministic, source-preserving file mutation.
Prefer it for bounded structural edits when its capabilities apply.
Discover usage with:
  threadmoth --help
  threadmoth capabilities
  threadmoth suggest <path>
Preview before committing when uncertain.
Do not bypass a Threadmoth refusal with a broader raw edit unless the user explicitly authorizes it.
```

That is intentionally small. The point is to test Threadmoth's discovery surfaces rather than preload the model with its protocol.

## Antigravity

The repository root contains a native Antigravity [`plugin.json`](../plugin.json) that packages the existing `skills/threadmoth/SKILL.md` without duplicating its instruction text. Install it with `agy plugin install <repository>`. Antigravity 1.1.27 authentication and skill discovery were verified locally. In a disposable fixture, a neutral mutation request led the agent to invoke `threadmoth preview`, which returned the real `TARGET_AMBIGUOUS` refusal with no file change; the follow-up recovery wandered into unsupported provider exploration and did not complete `mutate`, so Antigravity is not yet live-verified end-to-end.

## Useful discovery commands

```text
threadmoth --help
threadmoth capabilities
threadmoth capabilities --for PATH
threadmoth examples
threadmoth schema
threadmoth suggest PATH
threadmoth explain REASON_CODE
```

## Recommended agent policy

An agent should:

1. inspect capabilities before guessing a request shape;
2. use `suggest` for unfamiliar files or formats;
3. preview when the intended effect is not obvious, or use a prepared plan when work must cross an agent-step or human review boundary;
4. treat `REFUSED` as information, consume its machine-readable `recovery` remedies, and let the caller choose the next request;
5. only fall back to a broader edit when Threadmoth genuinely does not cover the task or the user explicitly authorizes the wider effect;
6. preserve and report the resulting certificate when diagnosing surprising behaviour.

## What Threadmoth is not

Threadmoth is not an AI task planner, formatter, compiler, test runner, Git client, or shell. Its `plan` command prepares a deterministic guarded mutation artifact; it does not decide what work should be done.

The model decides what should happen. Threadmoth provides a narrow deterministic mutation boundary and proves what actually changed.

## Recovery loop

Certificates may contain `recovery.requires_choice` and bounded remedies. For
ambiguity, each remedy includes a complete request patch with the exact
`candidate_guard` and observed `expected_pre_hash`. For stale state, the
remedy refreshes the observed hash; for effect budgets, it reports the exact
observed dimension. `suggest --from-refusal` exposes the same templates for
agents that prefer a separate discovery step. Threadmoth never picks a
candidate on the caller's behalf.

## MCP

Threadmoth also exposes an MCP stdio adapter:

```text
threadmoth mcp
```

MCP is an adapter over the same deterministic core. The CLI/JSON contract remains the lowest-common-denominator integration surface. A minimal stdio configuration is:

```json
{
  "mcpServers": {
    "threadmoth": {
      "command": "threadmoth",
      "args": ["mcp"]
    }
  }
}
```

The MCP tools are `threadmoth_capabilities`, `threadmoth_inspect`,
`threadmoth_suggest`, `threadmoth_explain`, `threadmoth_preview`,
`threadmoth_plan`, `threadmoth_apply_plan`, `threadmoth_transact_preview`,
`threadmoth_mutate`, and `threadmoth_transact`. There is deliberately no
`threadmoth_update`; self-update is an explicit CLI-only maintenance command.

## The 30-second agent loop

```text
capabilities / suggest
        ↓
preview
        ↓
APPLIED or REFUSED
        ↓
if REFUSED: explain / choose a reported candidate selection_id
        ↓
preview again with expected_pre_hash + candidate_guard
        ↓
mutate the same guarded request
        ↓
keep the certificate as proof
```

Tiny example conversation:

```text
Agent: threadmoth_inspect({"path":"config.json"})
Threadmoth: {"sha256":"...","encoding":"utf8","newline_profile":"lf",...}
Agent: threadmoth_suggest({"path":"config.json","goal":"set-value","at":"$.port"})
Threadmoth: {"provider":"json","request_template":{...}}
Agent: threadmoth_preview(request_template)
Threadmoth: {"outcome":"APPLIED","commit":{"mode":"dry_run"},...}
Agent: threadmoth_mutate(the_same_request)
Threadmoth: {"outcome":"APPLIED","post_hash":"...",...}
```

When work must cross an agent-step or human review boundary, use
`threadmoth_plan`/`threadmoth_apply_plan` (or the equivalent CLI commands).
The plan is portable but not trusted: apply refuses stale pre-images and
rechecks all guards, budgets, exact edits, and postconditions.
