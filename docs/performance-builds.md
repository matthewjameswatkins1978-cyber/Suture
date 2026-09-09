# Performance builds

All build flavours compile the same Threadmoth Core and provider set. Only
code generation differs.

| Flavour | Intended use | CPU baseline |
| --- | --- | --- |
| portable | default distributed release | compiler default |
| modern | distributed Windows/Linux x86-64 option | `x86-64-v3` |
| native | local build only | `target-cpu=native` |
| PGO | experiment only until repeatable benefit is proven | measured profile |

Portable is recommended when the machine is uncertain. Modern is labelled
`-v3` because it requires the x86-64-v3 feature baseline. Native is never
published as a general download: it is tuned to the build machine.

The `maxperf` Cargo profile uses fat LTO, one codegen unit, no incremental
state, stripped symbols and opt-level 3. This named release profile does not
alter normal development builds or global Cargo configuration.

Use `scripts/build-native.ps1` or `scripts/build-native.sh` for a local native
build. Use the modern scripts for an explicit x86-64-v3 build. Each script
sets tuning only for its child Cargo process, uses `--locked`, prints the
compiler and output path, and embeds metadata visible in `threadmoth doctor
--json`.

Optimization settings must be selected by measured Threadmoth workloads,
including CLI startup, structured and syntax edits, plans, transactions and
refusals. PGO remains evidence-gated and is not enabled by default.
