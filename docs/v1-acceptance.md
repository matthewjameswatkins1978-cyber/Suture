# Threadmoth 1.0 acceptance boundary

> **Historical design record.** This document describes the 1.0/early-1.1 acceptance boundary. It is intentionally retained for compatibility history, not as the current capability list. For Threadmoth 1.9 use [Coverage](coverage.md), [Protocol](protocol.md), and `threadmoth capabilities --json --all`.

At the 1.0 milestone, Threadmoth kept one mutation authority: providers proposed byte edits and Core alone validated, budgeted, committed, verified and certified them.

The v1.0 release protocol was `1.0.0`; the self-teaching discovery work introduced protocol `1.1.0`. Later releases retained compatible request handling while extending the protocol. The current 1.9 protocol is documented separately.

The early built-in provider boundary included:

- text: exact byte targets, cardinality, move and idempotent desired-state operations;
- strict JSON and JSONC: source-range structured edits;
- TOML: `toml_edit` candidate narrowed to the changed span, refusing fidelity drift;
- conservative YAML local edits with fail-closed unsupported structure;
- Markdown bounded heading regions;
- dotenv guarded key/value edits;
- pattern bounded Rust regex matching;
- patch exact unified-diff preimages, never fuzzy relocation;
- initial Tree-sitter code targeting;
- guarded filesystem create/delete/rename/move.

The certificate model already carried request/provider identity, pre/post hashes, changed byte and line ranges, bounded diff, structural validation, newline/BOM facts, effect-budget usage, commit guarantee and recovery state.

The core scope rule remains current even though the provider list has grown: Git, builds, tests, formatters, linters, package managers, arbitrary subprocesses, general network work, LSP/type semantics and workflow control are outside Threadmoth's mutation remit.
