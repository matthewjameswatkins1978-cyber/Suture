#!/usr/bin/env sh
set -eu
export THREADMOTH_BUILD_FLAVOR=modern
export THREADMOTH_CPU_BASELINE=x86-64-v3
export THREADMOTH_PGO_USED=false
export THREADMOTH_OPTIMIZATION_PROFILE=maxperf
rustc --version
printf '%s\n' 'Threadmoth modern maxperf build (target-cpu=x86-64-v3)'
RUSTFLAGS='-C target-cpu=x86-64-v3' cargo build --profile maxperf --locked
printf 'Output: %s\n' "$(pwd)/target/maxperf/threadmoth"
