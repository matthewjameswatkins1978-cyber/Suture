# Threadmoth optimisation / idiomatic Rust pass 1

> **Historical design record.** This file documents an early optimization/rename pass and is retained for provenance. It is not current Threadmoth 1.9 performance guidance. See [Performance builds](performance-builds.md) and [Performance results](performance-results.md) for the current build-flavour and benchmark policy.

Threadmoth is the public identity of the project previously called Suture. The behavioural contract and safety thesis are unchanged.

Canonical executable: `threadmoth`.
Convenience alias where practical: `thm`.

`threadmoth` is the compatibility contract. `thm` is not. No `.thm` source-file extension is part of the Threadmoth contract.

This record describes the first performance, stress-hardening and idiomatic-Rust pass.

Planned work at that milestone included:

- replace handwritten literal scans with `memchr::memmem`;
- eliminate repeated whole-prefix newline counting for changed line ranges;
- preallocate exact output capacity in the byte-edit engine;
- avoid computing a redundant post-commit SHA-256 on the success path;
- remove unconditional general-purpose line diff work from effect accounting only after equivalence tests proved edit-derived semantics;
- preserve stress-test fixes as permanent regressions, including clean failed commits and clean successful transaction recovery state;
- add regression coverage for empty replacement handling in text `set`/`rename` paths;
- preserve all safety, cardinality, refusal, transaction and certificate semantics;
- run formatting, clippy, the full test suite and the frozen torture suite before merge;
- complete the broad Suture -> Threadmoth crate/binary/docs/release rename as a controlled mechanical pass.

That pass is complete and historical. Threadmoth 1.9 adds a separate named `maxperf` profile plus explicit portable, x86-64-v3 modern and local-native build flavours; those later decisions are documented in the current performance pages rather than retrofitted into this record.
