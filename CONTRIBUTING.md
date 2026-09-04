# Contributing to Threadmoth

Threadmoth is currently looking for field-test evidence from both humans and coding agents.

Start with [FIELD_TESTING.md](FIELD_TESTING.md).

If you use an AI coding agent, also see:

- [Agent integration](docs/agent-integration.md)
- [Multi-agent field challenge](docs/agent-challenge.md)

## The most useful contributions right now

We especially want reports about:

- agents failing to discover or choose Threadmoth when it fits the task;
- valid bounded edits being refused;
- refusals that do not provide enough information to recover safely;
- unexpected mutation or preservation problems;
- confusing CLI/schema/capability behaviour;
- real language/provider gaps discovered during ordinary work.

Please use the matching GitHub issue template where possible.

## Before posting

Remove API keys, credentials, personal paths, proprietary source, and other secrets.

Reduce examples to the smallest case that still reproduces the behaviour. Include the Threadmoth version, OS, shell, provider/language, and coding agent if relevant.

## Pull requests

Small focused fixes are welcome. Please keep Threadmoth's core rules intact:

- deterministic;
- refusal-first;
- source-preserving;
- effect-budgeted;
- Core owns commits;
- providers/planners propose edits;
- no fuzzy guessing when evidence is ambiguous.

> The parser gets to point at the cloth. It doesn't get to re-weave it.
