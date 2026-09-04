# Threat model

Threadmoth 1.5 keeps the parser deliberately subordinate: it points at source bytes and validates candidates; it does not regenerate or reformat source.

Threadmoth assumes the caller may have stale context and the target file may be concurrently modified. The optional expected hash rejects stale observations; Core also hashes the file again immediately before staging. A post-commit read verifies landed bytes.

Path traversal, absolute paths, and symlink escapes are refused. Ancestor checks are repeated while resolving and commit uses a canonical destination path. This narrows pathname races; no portable userspace API makes replacement immune to an attacker with equivalent filesystem authority.

Structured input is parsed before mutation and the candidate is parsed again. Malformed JSON/TOML is never modified. JSON is strict: comments and extensions are not accepted. Certificate output is bounded, but v0.1 has no configurable parser quota.

Certificates do not include full file contents. Text duplicate diagnostics contain at most eight small contexts and hashes. Diff output is bounded; callers should treat operation values as potentially sensitive.

Atomic replacement protects readers from observing a partially written file after the staged file is flushed. Replacement may change timestamps and does not assert ACL/xattr preservation; permissions are platform-dependent.

Recovery journals are validated for structure, supported version, safe transaction ID, workspace-contained member paths, duplicate paths, size limits, and SHA-256 consistency before recovery writes. The writer applies the same 8 MiB serialized-journal limit before any transaction commit, so Threadmoth cannot create recovery evidence that its own reader will reject solely for size. Their location in `.threadmoth-recovery` or legacy `.suture-recovery` is not authenticated provenance: another same-user process with equivalent filesystem authority may plant or tamper with a journal. Recovery refuses when member state is not provably original or candidate.
