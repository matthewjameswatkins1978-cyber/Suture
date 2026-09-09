# Changelog

## 1.9.0 - Coverage and performance foundations

Threadmoth 1.9.0 adds a canonical coverage registry, Java/C#/PHP/HCL syntax
grammar admission, source-preserving nested YAML targeting, conservative INI
targeting, structured discovery metadata, and explicit portable/modern/native
build tooling. Advanced YAML constructs remain fail-closed and PGO remains an
experiment until measured evidence justifies release adoption.

## 1.8.1 - Completion and hardening

Threadmoth 1.8.1 hardens the 1.8 agent workflow without expanding semantic authorship. It adds actionable refusal recovery, bounded cross-process mutation locking, strict structured schema diagnostics, safe shorthand entry points, structured doctor output, and adversarial plan/filesystem coverage.

- Added deterministic refusal remedies and complete guarded retry templates for ambiguity, stale state, missing targets, and effect-budget refusals.
- Added `replace-exact`, `set-value`, and `create-file` CLI shorthands plus equivalent exact-replace/set-value MCP tools through the canonical Core pipeline.
- Added `WORKSPACE_BUSY` fail-closed locking across mutations, plan application, transactions, and recovery.
- Added protocol 1.3.1 while continuing to accept protocol 1.3.0, 1.2.0, and 1.1.0.
- Added `threadmoth doctor --json` and structured schema diagnostics for strict parser failures.
- Added release, plan, path-identity, and refusal-recovery regression coverage.

## 1.8.0 - Plans and proof

Threadmoth 1.8 makes guarded mutations portable and provable. It adds deterministic serialisable plans, exact stale-state rechecking, prospective and committed postcondition checks, and an explicit CLI-only updater for standalone installations.

- Added `threadmoth plan`, `threadmoth apply-plan`, and `threadmoth explain --plan`.
- Added bounded `file_exists`, `file_absent`, `sha256`, and `literal_count` assertions.
- Added protocol 1.3 capability flags, plan limits, and MCP plan/apply-plan parity.
- Added official GitHub-release self-update with archive SHA-256, GitHub digest, extracted-binary version, and safe replacement verification.
- Preserved local-only mutation and existing preview, mutate, transaction, recovery, and candidate-guard flows.

## 1.7.1 - Repository and distribution hardening

Threadmoth 1.7.1 keeps the 1.7 agent-usability runtime stable while making the public repository and release process consistent with the Threadmoth identity.

- Renamed the canonical GitHub repository to Threadmoth and updated links and package metadata.
- Added the native Antigravity adapter manifest and current integration documentation.
- Preserved refusal-first mutation semantics and published reproducible release artifacts with checksums and a manifest.

## 1.7.0 - Agent usability

Threadmoth 1.7.0 makes the refusal-first boundary easier for agents to discover, inspect, and recover through.

### Added

- MCP parity for inspect, suggest, explain, path-scoped capabilities, and non-writing transaction preview.
- Protocol 1.2 candidate selection guards with deterministic physical selection IDs bound to the exact observed file identity.
- Provider-preserving refusal recovery and adversarial text/code candidate-selection coverage.
- macOS Apple Silicon and Intel release targets.
- A short agent integration loop and copy-paste MCP configuration.

### Changed

- Protocol 1.1 requests and recovery journals remain accepted for compatibility.
- Common `.env.*`, Dockerfile, Makefile, Cargo, package, TypeScript, and config filename detection is deterministic without content guessing.

## 1.6.0 - Hardening release

Threadmoth 1.6.0 strengthens Threadmoth against several newly identified ambiguity, recovery, protocol, and pathological-input edge cases.

### Fixed

- False AST ambiguity caused by duplicate Tree-sitter spans.
- Recovery journal byte-array expansion.
- Pathological long-line diff refinement cost.
- JSON-RPC notification responses from the MCP stdio server.

### Added

- MCP `threadmoth_preview`.
- Regression and adversarial coverage for same-span AST nodes, binary-safe recovery, long-line refinement, and MCP protocol behavior.
- A dedicated long-line refinement benchmark case.

### Changed

- Recovery journal payloads use compact base64 strings and retain v1.5.1 decimal-array read compatibility.
- Diff planning trims common prefix/suffix bytes and applies a bounded byte-refinement cutoff.

This release preserves Threadmoth's source-preserving, refusal-first architecture: providers plan byte ranges, while Core remains responsible for guards, atomic writes, verification, and certification.
