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
3. preview when the intended effect is not obvious;
4. treat `REFUSED` as information, not an obstacle to route around;
5. only fall back to a broader edit when Threadmoth genuinely does not cover the task or the user explicitly authorizes the wider effect;
6. preserve and report the resulting certificate when diagnosing surprising behaviour.

## What Threadmoth is not

Threadmoth is not a planner, formatter, compiler, test runner, Git client, or shell.

The model decides what should happen. Threadmoth provides a narrow deterministic mutation boundary and proves what actually changed.

## MCP

Threadmoth also exposes an MCP stdio adapter:

```text
threadmoth mcp
```

MCP is an adapter over the same deterministic core. The CLI/JSON contract remains the lowest-common-denominator integration surface.