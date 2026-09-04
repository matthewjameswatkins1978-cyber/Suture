# Benchmark report

The release binary includes the correctness-checked benchmark and torture modules. They measure deterministic local execution, not AI token savings or a model comparison.

The primary falsification metric is **wrong mutation reported as successful**. The harness checks expected bytes for successful mutations and refusal for malformed, ambiguous, and stale cases. It also records certificate size and elapsed time for a tiny text file, roughly 30 KB, and roughly 1 MB with a tiny edit.

Run the standard benchmark and the safety suite:

```text
suture benchmark
suture torture
```

The benchmark accepts `quick`, `standard` (the default), or `tough` profiles. Add `--json` for machine-readable output. The tough profile adds 5 MiB and 32 MiB files, a two-million-byte long-line case, a many-line case, and 250 repeated small-file checks. Every benchmark reports correctness before timing and exits non-zero if an expected successful mutation is wrong.

The torture command runs deterministic apply/refusal/stale-identity/transaction-cleanup checks, symlink containment where the host permits link creation, and FOOTGUN-100. It emits visible states such as `SETTING_UP`, `RUNNING`, `CHECKING`, `PASS`, `FAIL`, and `SKIP`; a host capability skip is not treated as a product failure, while any failed safety case is non-zero.

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

## Release tough-profile run

The new release command was run on 2026-09-04 on this Windows x86_64 host at pass-1 commit `68a05da`:

| case | bytes | iterations | wrong applied | average |
|---|---:|---:|---:|---:|
| tiny | 21 | 100 | 0 | 389.8 us |
| config_30k | 30,008 | 50 | 0 | 759.0 us |
| text_1m | 1,000,008 | 25 | 0 | 8,090.9 us |
| text_5m | 5,000,008 | 10 | 0 | 38,774.7 us |
| text_32m | 32,000,008 | 3 | 0 | 247,907.3 us |
| many_lines_2m | 1,999,992 | 10 | 0 | 28,223.8 us |
| long_line_2m | 2,000,006 | 5 | 0 | 15,997.5 us |
| small_files_250 | 2,500 total | 250 | 0 | 1,609.1 us |

These are local measurements, not universal performance claims. The important correctness result is zero wrong successful mutations across the tough profile.
