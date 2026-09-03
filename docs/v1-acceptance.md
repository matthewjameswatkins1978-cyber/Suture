# Suture 1.0 acceptance boundary

Suture 1.0 keeps one mutation authority: providers propose byte edits and Core alone validates, budgets, commits, verifies, and certifies them.

The public protocol is `1.0.0`. The normal one-file surface is `suture mutate` (or `suture preview`); `suture capabilities` describes the exact runtime surface. `suture transact` prepares all content candidates before writing, journals them before commit, and reports `transactional_with_rollback` when rollback is available. `suture recover` never silently ignores an interrupted journal.

Built-in providers are explicit and never fall back to one another:

- text: exact byte targets, cardinality, move, and idempotent desired-state operations;
- strict JSON and JSONC: source-range JSON Pointer edits, with JSONC comments/trailing commas masked without changing offsets;
- TOML: `toml_edit` candidate narrowed to the changed span, refusing fidelity drift;
- YAML: parsed and source-preserving for conservative local key/value edits, including inline scalar/sequence/mapping values; anchors, aliases, block scalars, and unsupported structure refuse rather than reserialize the document;
- Markdown: bounded heading sections;
- dotenv: guarded key/value edits preserving comments and unrelated lines;
- pattern: bounded Rust regex matching, explicit cardinality, and no callbacks;
- patch: exact unified-diff preimages and hunk counts, never fuzzy relocation;
- code: Tree-sitter syntax validation and exact node text/kind targeting for JavaScript/TypeScript/JSX/TSX, Python, Rust, and Go;
- filesystem: guarded create, delete, rename, and move operations.

The certificate includes request identity, provider identity, pre/post hashes, changed byte and line ranges, bounded diff, structural validation, newline/BOM facts, effect-budget usage, commit guarantee, and recovery state. Generated-file markers, binary input, unknown encodings, stale identities, duplicate targets, path escape, and unsupported preservation are fail-closed refusals.

Git, builds, tests, formatters, linters, package managers, arbitrary subprocesses, network access, LSP semantics, and general workflow control remain outside Suture.
