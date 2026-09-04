# Architecture

Threadmoth runs `OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY`.

> The parser gets to point at the cloth. It doesn’t get to re-weave it.

Core observes bytes through `Workspace`, checks the request version and optional pre-hash, asks exactly one provider for a byte-edit plan, applies edits in memory, validates the candidate, and owns persistence. Providers understand formats and propose edits; only Core commits.

A plan is not trusted merely because it is syntactically valid. Core checks edit ordering and bounds, compares the candidate with the original, bounds evidence, stages through the workspace, rechecks the pre-hash immediately before commit, and reads back the committed file.

Providers return byte ranges against the observed source. Text edits are exact byte matches. JSON uses strict parsing plus a source-range tree for localized edits. TOML derives a candidate with `toml_edit`, then narrows it to the changed source span and refuses representation drift outside the v0.1 contract.

The workspace rejects absolute paths, `..` escapes, and symlink paths resolving outside the declared root. Commit uses destination-directory staged atomic replacement and reports metadata limits explicitly.

## One core, two plan constructors

Structural providers use the shared statically compiled Tree-sitter syntax engine to locate exact source-node bytes. A separate desired-state Diff Planner accepts observed bytes and explicitly supplied desired bytes, then derives deterministic, bounded, disjoint byte edits. Neither provider nor planner writes files. The external formatter, migration tool, or AI that produced desired bytes is outside Threadmoth; Threadmoth does not execute it.

Before commit, Core proves that the derived edits produce the exact desired bytes. After commit, it reads the landed bytes and proves the post-hash equals the desired hash. Desired-state mode accounts for all supplied divergence; it does not claim unrelated-byte preservation when the desired state intentionally reformats a file.

## Syntax targeting

AST-grounded targeting means exact text plus a Tree-sitter node boundary plus cardinality. AST-typed targeting adds an explicitly requested `node_kind`. Threadmoth never infers a grammatical role during committed mutation and never unparses or pretty-prints an AST.
