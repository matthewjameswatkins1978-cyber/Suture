# Provider contract

Providers may understand formats and propose edits; only Core commits.

A provider receives observed bytes and a typed operation. It must return bounded, sorted, non-overlapping byte edits or a typed refusal/error. It must not write files, invoke formatters, call a network, execute subprocesses, silently choose an ambiguous target, silently downgrade to another provider, or claim an unverified preservation guarantee.

Core applies edits in memory, checks the resulting bytes, performs structural validation where applicable, guards source identity again, commits, reads the landed bytes back and certifies the result. Providers cannot bypass those gates.

## Provider rules in 1.9

- **Text** uses exact byte targets and preserves unrelated bytes outside the selected range.
- **JSON / JSONC** use source-range structural edits and do not require whole-document serialization for local changes.
- **TOML** uses `toml_edit` only to construct/validate a candidate and narrows the authorised source span; representation drift outside that span refuses.
- **YAML** uses parser-grounded nested-path location and only performs local operations whose source preservation can be proved. Advanced YAML constructs remain fail-closed where locality cannot be guaranteed.
- **INI** targets section/key source ranges and preserves unrelated comments, ordering and layout where supported.
- **Markdown** owns bounded heading/list/fenced regions rather than claiming full Markdown semantics.
- **dotenv** owns narrow key/value lines.
- **Code / Web** use Tree-sitter only to locate and validate source nodes. They never unparse an AST.
- **Pattern** is bounded regex targeting, not structural parsing.
- **Patch** requires exact unified-diff preimages and never performs fuzzy relocation.
- **Desired state** accepts explicit desired bytes and proves the derived bounded edits reproduce those bytes exactly.
- **Filesystem** handles guarded create/delete/rename/move while Core retains containment, locking, verification and recovery responsibilities.

If the requested provider cannot prove its own contract, the correct result is a typed refusal. Threadmoth may advertise a weaker explicit fallback route to the caller, but the caller must choose it.
