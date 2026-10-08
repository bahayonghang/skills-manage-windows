param(
    [ValidatePattern('^[a-z0-9-]+$')][string]$Label = 'baseline',
    [string]$Files = '128,4096,13016,18000,32',
    [ValidateRange(1, 7)][int]$Samples = 7,
    [string]$Binary,
    [ValidateSet('service', 'stage')][string]$Mode = 'service'
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../../../..')).Path
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$outputPath = Join-Path $PSScriptRoot "$Label-$timestamp.log"
$priorFiles = $env:SKILLPORT_BENCH_FILES
$priorSamples = $env:SKILLPORT_BENCH_SAMPLES
$priorMode = $env:SKILLPORT_BENCH_MODE
Push-Location -LiteralPath $repositoryRoot
try {
    $env:SKILLPORT_BENCH_FILES = $Files
    $env:SKILLPORT_BENCH_SAMPLES = "$Samples"
    $env:SKILLPORT_BENCH_MODE = $Mode
    if ($Binary) {
        $resolvedBinary = (Resolve-Path -LiteralPath $Binary).Path
        [ordered]@{
            captured_utc = (Get-Date).ToUniversalTime().ToString('o')
            binary = $Binary
            binary_sha256 = (Get-FileHash -LiteralPath $resolvedBinary -Algorithm SHA256).Hash
            source_head_at_invocation = (git rev-parse HEAD)
            current_harness_sha256 = (Get-FileHash -LiteralPath 'src-tauri/src/services/central_updates/core/performance_benchmark.rs' -Algorithm SHA256).Hash
            files = $Files
            samples = $Samples
            mode = $Mode
            identity_note = 'Binary source and actual harness identity must match the frozen build receipt. Current worktree fields are invocation context only.'
        } | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath ([System.IO.Path]::ChangeExtension($outputPath, '.identity.json')) -Encoding utf8
        & $resolvedBinary small_files_service_baseline --ignored --nocapture --test-threads=1 2>&1 |
            Tee-Object -FilePath $outputPath
    }
    else {
        cargo test --manifest-path src-tauri/Cargo.toml --release --locked --lib small_files_service_baseline -- --ignored --nocapture --test-threads=1 2>&1 |
            Tee-Object -FilePath $outputPath
    }
    $benchmarkExit = $LASTEXITCODE
    Write-Output "Benchmark output: $outputPath"
    exit $benchmarkExit
}
finally {
    $env:SKILLPORT_BENCH_FILES = $priorFiles
    $env:SKILLPORT_BENCH_SAMPLES = $priorSamples
    $env:SKILLPORT_BENCH_MODE = $priorMode
    Pop-Location
}
