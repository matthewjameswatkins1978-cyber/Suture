# Architecture

Threadmoth runs `OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY`.

Threadmoth 1.9.0 keeps the mutation boundary deliberately small. All CLI,
MCP, shorthand, plan, and transaction paths converge on Core preparation,
guarding, verification, and certification. Cooperating writers take the
workspace mutation lock for the full read/prepare/commit/verify boundary;
read-only discovery remains unlocked. The lock protects against another
Threadmoth process, not an unrelated editor, so stale hashes and post-commit
readback remain part of the guarantee.

For the prepared-plan workflow the same core becomes `OBSERVE -> IDENTIFY -> GUARD -> PLAN -> VERIFY PROSPECTIVE -> MUTATE -> VERIFY COMMITTED -> CERTIFY`.

> The parser gets to point at the cloth. It doesn’t get to re-weave it.

Core observes bytes through `Workspace`, checks the request version and optional pre-hash, asks exactly one provider for a byte-edit plan, applies edits in memory, validates the candidate, and owns persistence. Providers understand formats and propose edits; only Core commits.

A plan is not trusted merely because it is syntactically valid. Core checks edit ordering and bounds, compares the candidate with the original, bounds evidence, stages through the workspace, rechecks the pre-hash immediately before commit, and reads back the committed file.

Providers return byte ranges against the observed source. Text edits are exact byte matches. JSON uses strict parsing plus a source-range tree for localized edits. TOML derives a candidate with `toml_edit`, then narrows it to the changed source span and refuses representation drift outside the v0.1 contract.

The workspace rejects absolute paths, `..` escapes, and symlink paths resolving outside the declared root. Commit uses destination-directory staged atomic replacement and reports metadata limits explicitly.

## Prepared plans and maintenance boundary

`threadmoth plan` serialises the existing guarded preparation result without writing. `threadmoth apply-plan` treats that file as staleable and tamperable input: it checks the deterministic plan ID, workspace containment, stored pre-images, provider resolution, exact byte edits, effect budgets, and assertions before entering the existing journaled commit path. It then reads committed bytes back and evaluates assertions again. Preview and ordinary mutation continue to use their existing surfaces; plans are an additional prepare/commit handoff, not a second transaction engine.

The updater is a separate CLI maintenance boundary. Mutation Core, providers, and MCP mutation tools have no updater call path and no network capability. Only an explicit `threadmoth update` invocation contacts the official release repository.

## One core, two plan constructors

Structural providers use the shared statically compiled Tree-sitter syntax engine to locate exact source-node bytes. A separate desired-state Diff Planner accepts observed bytes and explicitly supplied desired bytes, then derives deterministic, bounded, disjoint byte edits. Neither provider nor planner writes files. The external formatter, migration tool, or AI that produced desired bytes is outside Threadmoth; Threadmoth does not execute it.

Before commit, Core proves that the derived edits produce the exact desired bytes. After commit, it reads the landed bytes and proves the post-hash equals the desired hash. Desired-state mode accounts for all supplied divergence; it does not claim unrelated-byte preservation when the desired state intentionally reformats a file.

## Syntax targeting

AST-grounded targeting means exact text plus a Tree-sitter node boundary plus cardinality. AST-typed targeting adds an explicitly requested `node_kind`. Threadmoth never infers a grammatical role during committed mutation and never unparses or pretty-prints an AST.
