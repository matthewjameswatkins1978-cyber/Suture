#!/usr/bin/env sh
set -eu
export THREADMOTH_BUILD_FLAVOR=native
export THREADMOTH_CPU_BASELINE=native
export THREADMOTH_PGO_USED=false
export THREADMOTH_OPTIMIZATION_PROFILE=maxperf
rustc --version
printf '%s\n' 'Threadmoth native maxperf build (target-cpu=native)'
RUSTFLAGS='-C target-cpu=native' cargo build --profile maxperf --locked
printf 'Output: %s\n' "$(pwd)/target/maxperf/threadmoth"
