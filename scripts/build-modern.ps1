$ErrorActionPreference = 'Stop'
$saved = @{}
foreach ($name in 'THREADMOTH_BUILD_FLAVOR', 'THREADMOTH_CPU_BASELINE', 'THREADMOTH_PGO_USED', 'THREADMOTH_OPTIMIZATION_PROFILE', 'RUSTFLAGS') {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name)
}
try {
    $env:THREADMOTH_BUILD_FLAVOR = 'modern'
    $env:THREADMOTH_CPU_BASELINE = 'x86-64-v3'
    $env:THREADMOTH_PGO_USED = 'false'
    $env:THREADMOTH_OPTIMIZATION_PROFILE = 'maxperf'
    Write-Host (rustc --version)
    Write-Host "Threadmoth modern maxperf build (target-cpu=x86-64-v3)"
    $env:RUSTFLAGS = '-C target-cpu=x86-64-v3'
    cargo build --profile maxperf --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
    Write-Host "Output: $(Resolve-Path target\maxperf\threadmoth.exe)"
}
finally {
    foreach ($name in $saved.Keys) {
        [Environment]::SetEnvironmentVariable($name, $saved[$name])
    }
}
