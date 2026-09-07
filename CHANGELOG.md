# Changelog

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
