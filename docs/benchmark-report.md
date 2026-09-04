# Benchmark report

This report is generated from the checked-in `examples/benchmark.rs` harness. It measures deterministic local library execution, not AI token savings or a model comparison.

The primary falsification metric is **wrong mutation reported as successful**. The harness checks expected bytes for successful mutations and refusal for malformed, ambiguous, and stale cases. It also records certificate size and elapsed time for a tiny text file, roughly 30 KB, and roughly 1 MB with a tiny edit.

Run:

```text
cargo run --release --example benchmark
```

Results are environment-dependent. The checked-in harness is the reproducible evidence source; future runs should record machine, Rust version, and output rather than treating local numbers as universal claims.

## Windows x86_64 run

The following was produced on 2026-09-03 with the release binary on this Windows x86_64 host:

| case | bytes | iterations | wrong applied | average | certificate |
|---|---:|---:|---:|---:|---:|
| tiny | 21 | 25 | 0 | 367.2 us | 999 B |
| config_30k | 30,008 | 25 | 0 | 705.8 us | 1,013 B |
| text_1m | 1,000,008 | 25 | 0 | 4,755.2 us | 1,021 B |

This is one local run, not a cross-platform release claim. The important result is zero wrong successful mutations in the checked cases; Linux numbers and CI results must be recorded separately when available.

## Windows x86_64 pass 1 rerun

The pass-1 branch was rerun on 2026-09-04 with the same checked-in harness after the newline-index and success-path verification changes:

| case | bytes | iterations | wrong applied | average | certificate |
|---|---:|---:|---:|---:|---:|
| tiny | 21 | 25 | 0 | 622.1 us | 1,483 B |
| config_30k | 30,008 | 25 | 0 | 812.1 us | 1,497 B |
| text_1m | 1,000,008 | 25 | 0 | 7,937.5 us | 1,505 B |

The timings are a single Windows run and are not a before/after claim; the certificate sizes differ from the earlier run because the branch includes later protocol/runtime changes. The safety signal remains zero incorrect successful mutations.
