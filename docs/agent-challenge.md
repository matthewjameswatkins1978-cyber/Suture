# Threadmoth multi-agent field challenge

This challenge measures whether coding agents can discover and use Threadmoth with minimal coaching.

It is not a benchmark of which model is smartest. It is a field test of Threadmoth's discoverability, refusal ergonomics, and usefulness inside real agent workflows.

## Agents to try

Use any coding agents you have access to. Suggested set:

- Codex
- Claude Code
- Gemini CLI
- Cline
- OpenCode

Fresh sessions are preferred.

## Keep the setup comparable

For each agent:

1. use the same repository snapshot;
2. use the same task;
3. ensure the same Threadmoth version is on `PATH`;
4. start with a fresh agent context where practical;
5. provide only the minimal Threadmoth hint below;
6. do not explain the request schema unless the agent asks or gets stuck;
7. record what the agent actually does.

## Minimal hint

Give each agent only:

```text
Threadmoth is installed and available for deterministic, source-preserving file mutation.
Use it when its capabilities fit the task.
Discover it yourself with `threadmoth --help`, `threadmoth capabilities`, and `threadmoth suggest <path>`.
Preview when uncertain.
Do not bypass a Threadmoth refusal with a broader raw edit unless the user explicitly authorizes that.
```

## Recommended task shape

Use a small disposable repository containing several file types, ideally including:

- JSON or TOML configuration
- Markdown
- one programming language supported by Threadmoth
- HTML/CSS or PowerShell if available

Give the agent an ordinary maintenance request that needs two or three bounded changes, for example:

```text
Update the local development port in the config, rename one documented command in the README, and make the matching code change. Keep unrelated formatting and comments exactly as they are. Use the safest installed tooling available.
```

Do not say "use Threadmoth for every edit." Tool selection is part of the test.

## Optional refusal round

After the first successful task, introduce an ambiguous target on purpose, such as two identical code snippets where only one should change.

Observe whether the agent:

- notices/refines the target;
- uses AST typing or another stronger guard;
- asks for clarification;
- bypasses the refusal with a broad editor;
- gives up.

## Record these observations

For each agent, record:

| Field | Result |
|---|---|
| Agent/version | |
| Threadmoth version | |
| OS/shell | |
| Discovered Threadmoth without schema coaching? | yes/no |
| First discovery command used | |
| Used `suggest`? | yes/no |
| Used `capabilities`? | yes/no |
| Previewed before first mutation? | yes/no |
| Number of Threadmoth mutations attempted | |
| Number applied | |
| Number refused | |
| Recovered safely from refusal? | yes/no/n-a |
| Fell back to raw editing? | yes/no |
| If fallback, why? | |
| Unexpected collateral changes? | yes/no |
| Task ultimately correct? | yes/no |
| Human intervention required? | |
| Notes | |

## What counts as a Threadmoth failure

Please report cases such as:

- the agent could not discover an available capability;
- the CLI/schema made the correct operation unnecessarily hard to construct;
- a valid bounded edit was refused because of an avoidable limitation;
- a refusal did not provide enough information for the agent to recover safely;
- Threadmoth changed bytes outside the authorised effect;
- the agent repeatedly preferred a raw editor even though Threadmoth clearly fit the task.

The last item is especially interesting. A safe tool that agents consistently refuse to use has a usability problem even if its code is correct.

## What is not automatically a failure

A refusal is not itself a failure.

Threadmoth is expected to refuse stale, ambiguous, unsupported, or over-budget requests. The question is whether the refusal was correct and whether the caller could understand the safe next step.

## Sharing results

Open a repository issue using the relevant field-test template. For comparative runs, one issue covering all agents is fine if you include the table above for each agent.

Please remove secrets, private repository content, personal paths, API keys, and proprietary code before posting.