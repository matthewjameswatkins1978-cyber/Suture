# Threat model

Threadmoth 1.8 keeps the parser deliberately subordinate: it points at source bytes and validates candidates; it does not regenerate or reformat source.

Threadmoth assumes the caller may have stale context and the target file may be concurrently modified. The optional expected hash rejects stale observations; Core also hashes the file again immediately before staging. A post-commit read verifies landed bytes.

Path traversal, absolute paths, and symlink escapes are refused. Ancestor checks are repeated while resolving and commit uses a canonical destination path. This narrows pathname races; no portable userspace API makes replacement immune to an attacker with equivalent filesystem authority.

Structured input is parsed before mutation and the candidate is parsed again. Malformed JSON/TOML is never modified. JSON is strict: comments and extensions are not accepted. Request, file, plan, assertion, and diagnostic evidence sizes are bounded by Threadmoth's advertised resource limits rather than being accepted without limit.

Certificates do not include full file contents. Text duplicate diagnostics contain at most eight small contexts and hashes. Diff output is bounded; callers should treat operation values as potentially sensitive.

Atomic replacement protects readers from observing a partially written file after the staged file is flushed. Replacement may change timestamps and does not assert ACL/xattr preservation; permissions are platform-dependent.

Recovery journals are validated for structure, supported version, safe transaction ID, workspace-contained member paths, duplicate paths, size limits, and SHA-256 consistency before recovery writes. The writer applies the same 8 MiB compact-journal limit before any transaction commit; byte payloads use base64 strings, while the reader accepts the v1.5.1 decimal-array representation for upgrade recovery. Their location in `.threadmoth-recovery` or legacy `.suture-recovery` is not authenticated provenance: another same-user process with equivalent filesystem authority may plant or tamper with a journal. Recovery refuses when member state is not provably original or candidate.

## 1.8 surfaces

Prepared plans are untrusted files. They may be stale, edited, oversized, copied from another workspace, or path-manipulated. Applying one never bypasses Core guards: schema and deterministic identity are checked, paths remain contained, symlink escapes refuse, every pre-image hash is re-read, exact edits and budgets are revalidated, and prospective plus committed assertions are evaluated. No fuzzy relocation or trusted-plan flag exists.

The explicit self-updater is the only network-capable command. It uses the canonical GitHub Releases repository, accepts stable semantic versions, requires exactly one platform archive and checksum manifest, verifies SHA-256 and any published GitHub asset digest, rejects archive/executable/version mismatches, and uses the maintained replacement machinery only after all checks pass. Package-managed installations are not overwritten. Checksums provide integrity, not independent release authenticity; signature verification remains future hardening unless enabled in a later release.
