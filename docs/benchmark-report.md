# Benchmark report

The release binary includes correctness-checked benchmark and torture modes. They measure deterministic local execution, not AI token savings or a model comparison.

The primary falsification metric is **wrong mutation reported as successful**. The harness checks expected bytes for successful mutations and refusal for malformed, ambiguous, and stale cases. It also records elapsed time across increasingly difficult file shapes.

Run the standard benchmark and the safety suite:

```text
threadmoth benchmark
threadmoth benchmark --torture
```

The benchmark modes are:

```text
threadmoth benchmark --quick
threadmoth benchmark
threadmoth benchmark --tough
threadmoth benchmark --torture
```

Add `--json` for machine-readable output. The tough profile adds 5 MiB and 32 MiB files, a two-million-byte long-line case, a many-line case, and 250 repeated small-file checks. Every benchmark reports correctness before timing and exits non-zero if an expected successful mutation is wrong.

The torture mode runs deterministic apply/refusal/stale-identity/transaction-cleanup checks, symlink containment where the host permits link creation, and FOOTGUN-100. It emits visible states such as `SETTING_UP`, `RUNNING`, `CHECKING`, `PASS`, `FAIL`, and `SKIP`; a host capability skip is not treated as a product failure, while any failed safety case is non-zero.

For Threadmoth 1.3, the older forms `threadmoth benchmark tough` and `threadmoth torture` remain compatibility routes, but the flag forms above are canonical.

Results are environment-dependent. The checked-in harness is the reproducible evidence source; future runs should record machine, Rust version, and output rather than treating local numbers as universal claims.

## Current observed Threadmoth 1.2 tough-profile run

This run was supplied from a live tough-profile execution on 2026-09-04:

| case | bytes | iterations | wrong applied | average |
|---|---:|---:|---:|---:|
| tiny | 21 | 100 | 0 | 866.7 us |
| config_30k | 30,008 | 50 | 0 | 1,329.4 us |
| text_1m | 1,000,008 | 25 | 0 | 14,263.7 us |
| text_5m | 5,000,008 | 10 | 0 | 55,721.7 us |
| text_32m | 32,000,008 | 3 | 0 | 290,643.4 us |
| many_lines_2m | 1,999,992 | 10 | 0 | 28,951.9 us |
| long_line_2m | 2,000,006 | 5 | 0 | 17,139.8 us |
| small_files_250 | 2,500 total | 1 batch | 0 | 5,253.5 us |

The run completed with:

```text
state: CHECKING wrong_successful_mutations=0
state: PASS profile=tough
```

The important correctness result is **zero wrong successful mutations across the entire tough profile**.

The timing result is also useful context: in this observed run, a 1 MB mutation completed in about 14.3 ms, 5 MB in about 55.7 ms, and 32 MB in about 291 ms. These are local measurements, not universal performance guarantees.

## Earlier Windows x86_64 run

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

## Earlier Threadmoth 1.2 tough-profile run

An earlier release-candidate command was run on 2026-09-04 on a Windows x86_64 host:

| case | bytes | iterations | wrong applied | average |
|---|---:|---:|---:|---:|
| tiny | 21 | 100 | 0 | 456.1 us |
| config_30k | 30,008 | 50 | 0 | 835.7 us |
| text_1m | 1,000,008 | 25 | 0 | 7,571.8 us |
| text_5m | 5,000,008 | 10 | 0 | 37,258.2 us |
| text_32m | 32,000,008 | 3 | 0 | 251,627.2 us |
| many_lines_2m | 1,999,992 | 10 | 0 | 27,400.9 us |
| long_line_2m | 2,000,006 | 5 | 0 | 16,046.4 us |
| small_files_250 | 2,500 total | 250 | 0 | 2,580.8 us |

These earlier measurements are retained for history, but the README now uses the current observed tough-profile run above.
