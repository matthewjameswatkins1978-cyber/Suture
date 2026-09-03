# Architecture

Suture runs `OBSERVE -> IDENTIFY -> GUARD -> MUTATE -> VERIFY -> CERTIFY`.

Core observes bytes through `Workspace`, checks the request version and optional pre-hash, asks exactly one provider for a byte-edit plan, applies edits in memory, validates the candidate, and owns persistence. Providers understand formats and propose edits; only Core commits.

A plan is not trusted merely because it is syntactically valid. Core checks edit ordering and bounds, compares the candidate with the original, bounds evidence, stages through the workspace, rechecks the pre-hash immediately before commit, and reads back the committed file.

Providers return byte ranges against the observed source. Text edits are exact byte matches. JSON uses strict parsing plus a source-range tree for localized edits. TOML derives a candidate with `toml_edit`, then narrows it to the changed source span and refuses representation drift outside the v0.1 contract.

The workspace rejects absolute paths, `..` escapes, and symlink paths resolving outside the declared root. Commit uses destination-directory staged atomic replacement and reports metadata limits explicitly.
