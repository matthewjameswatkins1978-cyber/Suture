# Suture optimisation / idiomatic Rust pass 1

Working branch for the first performance and idiomatic-Rust pass.

Planned changes:

- replace handwritten literal scans with `memchr::memmem`;
- eliminate repeated whole-prefix newline counting for changed line ranges;
- preallocate exact output capacity in the byte-edit engine;
- avoid computing a redundant post-commit SHA-256 on the success path;
- remove the unconditional general-purpose line diff from effect accounting only after equivalence tests prove an edit-derived implementation matches existing semantics;
- add a regression test for empty replacement handling in text `set`/`rename` paths;
- preserve all safety, cardinality, refusal, transaction, and certificate semantics;
- run `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, the full test suite, and the frozen torture suite before merge.

The branch is intentionally isolated from `main` until those checks pass.
