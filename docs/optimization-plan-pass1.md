# Threadmoth optimisation / idiomatic Rust pass 1

Threadmoth is the new public identity of the project previously called Suture. The behavioural contract and safety thesis are unchanged.

Canonical executable: `threadmoth`.
Convenience alias where practical: `thm`.

`threadmoth` is the compatibility contract. `thm` is not. The source-file extension remains undecided and must not be inferred from the alias.

Working branch for the first performance, stress-hardening, and idiomatic-Rust pass.

Planned changes:

- replace handwritten literal scans with `memchr::memmem`;
- eliminate repeated whole-prefix newline counting for changed line ranges;
- preallocate exact output capacity in the byte-edit engine;
- avoid computing a redundant post-commit SHA-256 on the success path;
- remove the unconditional general-purpose line diff from effect accounting only after equivalence tests prove an edit-derived implementation matches existing semantics;
- preserve the stress-test fixes as permanent regressions, including clean failed commits and clean successful transaction recovery state;
- add regression coverage for empty replacement handling in text `set`/`rename` paths;
- preserve all safety, cardinality, refusal, transaction, and certificate semantics;
- run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, the full test suite, and the frozen torture suite before merge;
- complete the broad Suture -> Threadmoth crate/binary/docs/release rename as a controlled mechanical pass after the semantics-sensitive refactors.

The performance and stress-hardening work is complete; the final merge gate is the current release-candidate build, tests, benchmark, torture suite, and hosted CI.
